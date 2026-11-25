use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ghostpath_core::error::{CoreError, Result};
use ghostpath_core::fs_safe::{
    atomic_write_private, atomic_write_system_file, read_to_string_no_follow,
};
use ghostpath_core::geo::{validate_timezone_name, GeoCoordinates, GeoCountryPreset};
use ghostpath_core::paths::GhostPaths;
use serde::{Deserialize, Serialize};

use crate::journal::{JournalAction, UndoJournal};

/// Filesystem targets used by `GeoSpoofManager`, parameterizable for isolated testing.
#[derive(Debug, Clone)]
pub struct GeoSpoofPaths {
    pub localtime_path: PathBuf,
    pub timezone_file_path: PathBuf,
    pub zoneinfo_dir: PathBuf,
    pub geolocation_path: PathBuf,
    pub geoclue_dropin_path: PathBuf,
    pub timezone_backup_path: PathBuf,
    pub geolocation_backup_path: PathBuf,
    pub use_systemd_tools: bool,
}

impl GeoSpoofPaths {
    /// Production Linux paths managed by `ghostpathd`.
    pub fn system() -> Self {
        Self {
            localtime_path: PathBuf::from("/etc/localtime"),
            timezone_file_path: PathBuf::from("/etc/timezone"),
            zoneinfo_dir: PathBuf::from("/usr/share/zoneinfo"),
            geolocation_path: PathBuf::from("/etc/geolocation"),
            geoclue_dropin_path: PathBuf::from("/etc/geoclue/conf.d/99-ghostpath.conf"),
            timezone_backup_path: GhostPaths::timezone_backup(),
            geolocation_backup_path: GhostPaths::geolocation_backup(),
            use_systemd_tools: true,
        }
    }

    /// Isolated filesystem root for unit and integration tests without touching the host OS.
    pub fn with_root(root: &Path) -> Self {
        Self {
            localtime_path: root.join("etc/localtime"),
            timezone_file_path: root.join("etc/timezone"),
            zoneinfo_dir: root.join("usr/share/zoneinfo"),
            geolocation_path: root.join("etc/geolocation"),
            geoclue_dropin_path: root.join("etc/geoclue/conf.d/99-ghostpath.conf"),
            timezone_backup_path: root.join("var/lib/ghostpath/timezone.backup.json"),
            geolocation_backup_path: root.join("var/lib/ghostpath/geolocation.backup.json"),
            use_systemd_tools: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimezoneBackup {
    original_timezone: String,
    localtime_symlink_target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeolocationBackup {
    existed: bool,
    original_content: Option<String>,
}

/// Manages reversible Linux OS timezone and desktop GeoClue2 geolocation spoofing.
pub struct GeoSpoofManager {
    paths: GeoSpoofPaths,
    dry_run: bool,
}

impl GeoSpoofManager {
    pub fn new(dry_run: bool) -> Self {
        Self {
            paths: GeoSpoofPaths::system(),
            dry_run,
        }
    }

    pub fn with_paths(paths: GeoSpoofPaths, dry_run: bool) -> Self {
        Self { paths, dry_run }
    }

    pub fn paths(&self) -> &GeoSpoofPaths {
        &self.paths
    }

    /// Reads the live Linux timezone (via `timedatectl`, `/etc/localtime` symlink, or `/etc/timezone`).
    pub fn read_active_timezone(&self) -> String {
        read_active_timezone(&self.paths)
    }

    /// Reads currently active static coordinates from `/etc/geolocation` if present and valid.
    pub fn read_active_geolocation(&self) -> Option<GeoCoordinates> {
        read_active_geolocation(&self.paths)
    }

    /// Returns the backed-up original timezone if a spoofing session backup currently exists.
    pub fn read_backed_up_timezone(&self) -> Option<String> {
        let content = read_to_string_no_follow(&self.paths.timezone_backup_path, 64 * 1024).ok()?;
        let backup: TimezoneBackup = serde_json::from_str(&content).ok()?;
        Some(backup.original_timezone)
    }

    /// Checks whether a timezone and/or geolocation spoofing backup is active.
    pub fn is_spoof_active(&self) -> bool {
        self.paths.timezone_backup_path.exists() || self.paths.geolocation_backup_path.exists()
    }

    /// Checks whether `timedatectl` and `geoclue` are present on the system.
    pub fn detect_capabilities(&self) -> (bool, bool) {
        if !self.paths.use_systemd_tools {
            return (true, true);
        }
        let timedatectl = Command::new("timedatectl")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        let geoclue = Path::new("/etc/geoclue").exists()
            || Path::new("/usr/libexec/geoclue").exists()
            || Path::new("/usr/lib/geoclue-2.0/geoclue").exists();
        (timedatectl, geoclue)
    }

    /// Applies system timezone and/or GeoClue static geolocation for `preset`, recording undo state.
    pub fn apply(
        &self,
        preset: &GeoCountryPreset,
        spoof_timezone: bool,
        spoof_geolocation: bool,
        journal: &mut UndoJournal,
    ) -> Result<GeoSpoofGuard> {
        if !validate_timezone_name(&preset.timezone) {
            return Err(CoreError::Config(format!(
                "Invalid timezone identifier '{}'",
                preset.timezone
            )));
        }

        let current_tz = self.read_active_timezone();

        if self.dry_run {
            tracing::info!(
                country = %preset.country_code,
                timezone = %preset.timezone,
                spoof_timezone,
                spoof_geolocation,
                "[dry-run] Applying GeoSpoofManager state"
            );
            if spoof_timezone {
                journal.record(JournalAction::ModifiedTimezone {
                    backup_path: self
                        .paths
                        .timezone_backup_path
                        .to_string_lossy()
                        .to_string(),
                    original_timezone: current_tz,
                });
            }
            if spoof_geolocation {
                journal.record(JournalAction::ModifiedGeolocation {
                    backup_path: self
                        .paths
                        .geolocation_backup_path
                        .to_string_lossy()
                        .to_string(),
                    dropin_path: self.paths.geoclue_dropin_path.to_string_lossy().to_string(),
                });
            }
            return Ok(GeoSpoofGuard {
                active: true,
                paths: self.paths.clone(),
                dry_run: true,
            });
        }

        let (_, geoclue_ready) = self.detect_capabilities();
        let should_apply_geo = if spoof_geolocation && !geoclue_ready {
            if !spoof_timezone {
                return Err(CoreError::Config(
                    "GeoClue2 is not installed on this system. Install geoclue-2.0 or enable System Timezone spoofing."
                        .to_string(),
                ));
            }
            tracing::info!("GeoClue2 not installed on host; applying system timezone only");
            false
        } else {
            spoof_geolocation
        };

        if spoof_timezone {
            self.apply_timezone(&preset.timezone, &current_tz, journal)?;
        }

        if should_apply_geo {
            if let Err(err) = self.apply_geolocation(preset, journal) {
                if spoof_timezone {
                    let _ = restore_with_paths(&self.paths, false);
                }
                return Err(err);
            }
        }

        Ok(GeoSpoofGuard {
            active: spoof_timezone || should_apply_geo,
            paths: self.paths.clone(),
            dry_run: false,
        })
    }

    fn apply_timezone(
        &self,
        target_tz: &str,
        current_tz: &str,
        journal: &mut UndoJournal,
    ) -> Result<()> {
        // Never overwrite an existing original backup across hot country swaps.
        if !self.paths.timezone_backup_path.exists() {
            if let Some(parent) = self.paths.timezone_backup_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let symlink_target = fs::read_link(&self.paths.localtime_path)
                .ok()
                .map(|p| p.to_string_lossy().to_string());
            let backup = TimezoneBackup {
                original_timezone: current_tz.to_string(),
                localtime_symlink_target: symlink_target,
            };
            let json = serde_json::to_string_pretty(&backup)
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
            atomic_write_private(&self.paths.timezone_backup_path, json.as_bytes())?;
        }

        journal.record(JournalAction::ModifiedTimezone {
            backup_path: self
                .paths
                .timezone_backup_path
                .to_string_lossy()
                .to_string(),
            original_timezone: current_tz.to_string(),
        });

        apply_timezone_target(&self.paths, target_tz)
    }

    fn apply_geolocation(
        &self,
        preset: &GeoCountryPreset,
        journal: &mut UndoJournal,
    ) -> Result<()> {
        if !self.paths.geolocation_backup_path.exists() {
            if let Some(parent) = self.paths.geolocation_backup_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let existed = self.paths.geolocation_path.exists();
            let original_content = if existed {
                fs::read_to_string(&self.paths.geolocation_path).ok()
            } else {
                None
            };
            let backup = GeolocationBackup {
                existed,
                original_content,
            };
            let json = serde_json::to_string_pretty(&backup)
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
            atomic_write_private(&self.paths.geolocation_backup_path, json.as_bytes())?;
        }

        journal.record(JournalAction::ModifiedGeolocation {
            backup_path: self
                .paths
                .geolocation_backup_path
                .to_string_lossy()
                .to_string(),
            dropin_path: self.paths.geoclue_dropin_path.to_string_lossy().to_string(),
        });

        if let Some(parent) = self.paths.geolocation_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let geolocation_body = format!(
            "# Managed by GhostPath VPN ({})\n{:.6}\n{:.6}\n{:.1}\n{:.1}\n",
            preset.country_code,
            preset.latitude,
            preset.longitude,
            preset.altitude_m,
            preset.accuracy_m
        );
        atomic_write_system_file(
            &self.paths.geolocation_path,
            geolocation_body.as_bytes(),
            0o644,
        )?;

        if let Some(parent) = self.paths.geoclue_dropin_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                tracing::warn!(error = %e, "Could not create GeoClue drop-in directory");
            }
        }
        if self
            .paths
            .geoclue_dropin_path
            .parent()
            .is_some_and(|p| p.exists())
        {
            let dropin_body = "\
# Managed by GhostPath VPN while Geo Location Spoofing is active
[static-source]
enable=true

[wifi]
enable=false

[3g]
enable=false

[cdma]
enable=false

[modem-gps]
enable=false
";
            if let Err(e) = atomic_write_system_file(
                &self.paths.geoclue_dropin_path,
                dropin_body.as_bytes(),
                0o644,
            ) {
                tracing::warn!(error = %e, "Could not write GeoClue drop-in override");
            }
        }

        if self.paths.use_systemd_tools {
            let systemctl_bin = GhostPaths::resolve_trusted_system_bin("systemctl");
            let _ = Command::new(systemctl_bin)
                .args(["try-restart", "geoclue"])
                .output();
        }

        tracing::info!(
            country = %preset.country_code,
            city = %preset.city,
            "GeoClue static geolocation applied"
        );
        Ok(())
    }

    /// Restores original timezone and GeoClue geolocation from backups.
    pub fn restore(&self) -> Result<()> {
        restore_with_paths(&self.paths, self.dry_run)
    }

    /// Crash-recovery helper invoked by `ghostpathd --recover`.
    pub fn recover_from_journal(journal: &UndoJournal, dry_run: bool) -> Result<()> {
        let paths = GeoSpoofPaths::system();
        let has_geo_or_tz = journal.actions.iter().any(|a| {
            matches!(
                a,
                JournalAction::ModifiedTimezone { .. } | JournalAction::ModifiedGeolocation { .. }
            )
        });
        if has_geo_or_tz
            || paths.timezone_backup_path.exists()
            || paths.geolocation_backup_path.exists()
        {
            restore_with_paths(&paths, dry_run)?;
        }
        Ok(())
    }
}

/// RAII guard restoring original OS timezone and geolocation when dropped.
pub struct GeoSpoofGuard {
    active: bool,
    paths: GeoSpoofPaths,
    dry_run: bool,
}

impl GeoSpoofGuard {
    pub fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for GeoSpoofGuard {
    fn drop(&mut self) {
        if self.active && !self.dry_run {
            if let Err(e) = restore_with_paths(&self.paths, false) {
                tracing::warn!(error = %e, "Failed to restore geo/timezone on guard drop");
            }
        }
    }
}

fn read_active_timezone(paths: &GeoSpoofPaths) -> String {
    if paths.use_systemd_tools {
        if let Ok(out) = Command::new("timedatectl")
            .args(["show", "--property=Timezone", "--value"])
            .output()
        {
            if out.status.success() {
                let tz = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !tz.is_empty() && validate_timezone_name(&tz) {
                    return tz;
                }
            }
        }
    }

    if let Ok(target) = fs::read_link(&paths.localtime_path) {
        let zoneinfo_prefix = paths.zoneinfo_dir.to_string_lossy().to_string();
        let target_str = target.to_string_lossy();
        if let Some(rel) = target_str
            .strip_prefix(&zoneinfo_prefix)
            .or_else(|| target_str.strip_prefix("/usr/share/zoneinfo"))
        {
            let trimmed = rel.trim_start_matches('/');
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    if let Ok(tz_text) = fs::read_to_string(&paths.timezone_file_path) {
        let trimmed = tz_text.trim();
        if !trimmed.is_empty() && validate_timezone_name(trimmed) {
            return trimmed.to_string();
        }
    }

    "UTC".to_string()
}

fn read_active_geolocation(paths: &GeoSpoofPaths) -> Option<GeoCoordinates> {
    let content = fs::read_to_string(&paths.geolocation_path).ok()?;
    let nums: Vec<f64> = content
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.parse::<f64>().ok())
        .collect();

    if nums.len() >= 4 {
        Some(GeoCoordinates {
            latitude: nums[0],
            longitude: nums[1],
            altitude_m: nums[2],
            accuracy_m: nums[3],
        })
    } else if nums.len() >= 2 {
        Some(GeoCoordinates {
            latitude: nums[0],
            longitude: nums[1],
            altitude_m: 0.0,
            accuracy_m: 150.0,
        })
    } else {
        None
    }
}

fn apply_timezone_target(paths: &GeoSpoofPaths, target_tz: &str) -> Result<()> {
    if !validate_timezone_name(target_tz) {
        return Err(CoreError::Config(format!(
            "Unsafe timezone name '{target_tz}'"
        )));
    }

    if paths.use_systemd_tools {
        if let Ok(out) = Command::new("timedatectl")
            .args(["set-timezone", target_tz])
            .output()
        {
            if out.status.success() {
                tracing::info!(
                    timezone = target_tz,
                    "Applied system timezone via timedatectl"
                );
                return Ok(());
            }
            let err = String::from_utf8_lossy(&out.stderr);
            tracing::warn!(error = %err, "timedatectl set-timezone failed, falling back to /etc/localtime");
        }
    }

    let zone_target = paths.zoneinfo_dir.join(target_tz);
    if !zone_target.exists() {
        return Err(CoreError::Config(format!(
            "Zoneinfo file not found at {}",
            zone_target.display()
        )));
    }

    if let Some(parent) = paths.localtime_path.parent() {
        fs::create_dir_all(parent)?;
    }

    replace_symlink(&zone_target, &paths.localtime_path)?;

    if let Some(parent) = paths.timezone_file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&paths.timezone_file_path, format!("{target_tz}\n"))?;
    tracing::info!(
        timezone = target_tz,
        "Applied system timezone via localtime symlink"
    );
    Ok(())
}

fn restore_with_paths(paths: &GeoSpoofPaths, dry_run: bool) -> Result<()> {
    if dry_run {
        tracing::info!("[dry-run] Restoring original timezone and GeoClue state");
        return Ok(());
    }

    if paths.timezone_backup_path.exists() {
        if let Ok(content) = fs::read_to_string(&paths.timezone_backup_path) {
            if let Ok(backup) = serde_json::from_str::<TimezoneBackup>(&content) {
                let mut restored = false;
                if paths.use_systemd_tools && validate_timezone_name(&backup.original_timezone) {
                    if let Ok(out) = Command::new("timedatectl")
                        .args(["set-timezone", &backup.original_timezone])
                        .output()
                    {
                        restored = out.status.success();
                    }
                }
                if !restored {
                    if let Some(ref sym) = backup.localtime_symlink_target {
                        let target_path = PathBuf::from(sym);
                        if target_path.exists() {
                            let _ = replace_symlink(&target_path, &paths.localtime_path);
                        }
                    } else if validate_timezone_name(&backup.original_timezone) {
                        let _ = apply_timezone_target(paths, &backup.original_timezone);
                    }
                    if validate_timezone_name(&backup.original_timezone) {
                        let _ = fs::write(
                            &paths.timezone_file_path,
                            format!("{}\n", backup.original_timezone),
                        );
                    }
                }
                tracing::info!(timezone = %backup.original_timezone, "Restored original system timezone");
            }
        }
        let _ = fs::remove_file(&paths.timezone_backup_path);
    }

    let mut restored_geo = false;
    if paths.geoclue_dropin_path.exists() {
        let _ = fs::remove_file(&paths.geoclue_dropin_path);
        restored_geo = true;
    }

    if paths.geolocation_backup_path.exists() {
        if let Ok(content) = fs::read_to_string(&paths.geolocation_backup_path) {
            if let Ok(backup) = serde_json::from_str::<GeolocationBackup>(&content) {
                if backup.existed {
                    if let Some(orig) = backup.original_content {
                        let _ = fs::write(&paths.geolocation_path, orig);
                    }
                } else if paths.geolocation_path.exists() {
                    let _ = fs::remove_file(&paths.geolocation_path);
                }
                restored_geo = true;
            }
        }
        let _ = fs::remove_file(&paths.geolocation_backup_path);
    }

    if restored_geo && paths.use_systemd_tools {
        let _ = Command::new("systemctl")
            .args(["try-restart", "geoclue"])
            .output();
        tracing::info!("Restored original GeoClue location posture");
    }

    Ok(())
}

fn replace_symlink(target: &Path, link_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        if fs::symlink_metadata(link_path).is_ok() {
            fs::remove_file(link_path)?;
        }
        std::os::unix::fs::symlink(target, link_path)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = (target, link_path);
        Ok(())
    }
}

fn set_world_readable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o644));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ghostpath_core::geo::find_country_preset;
    use tempfile::tempdir;

    #[cfg(unix)]
    fn seed_zoneinfo(paths: &GeoSpoofPaths, zones: &[&str]) {
        for z in zones {
            let p = paths.zoneinfo_dir.join(z);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, format!("TZif2-{z}")).unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_apply_and_restore_isolated_root_preserves_original_across_swaps() {
        let dir = tempdir().unwrap();
        let paths = GeoSpoofPaths::with_root(dir.path());
        seed_zoneinfo(
            &paths,
            &["Asia/Tehran", "Europe/Berlin", "Europe/Amsterdam"],
        );

        fs::create_dir_all(paths.localtime_path.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(
            paths.zoneinfo_dir.join("Asia/Tehran"),
            &paths.localtime_path,
        )
        .unwrap();
        fs::write(&paths.timezone_file_path, "Asia/Tehran\n").unwrap();

        let mgr = GeoSpoofManager::with_paths(paths.clone(), false);
        assert_eq!(mgr.read_active_timezone(), "Asia/Tehran");
        assert!(mgr.read_active_geolocation().is_none());

        let mut journal = UndoJournal::new();
        let de = find_country_preset("DE").unwrap();
        let mut guard = mgr.apply(&de, true, true, &mut journal).unwrap();

        assert_eq!(mgr.read_active_timezone(), "Europe/Berlin");
        assert_eq!(
            mgr.read_backed_up_timezone().as_deref(),
            Some("Asia/Tehran")
        );
        let coords = mgr.read_active_geolocation().unwrap();
        assert!((coords.latitude - de.latitude).abs() < 0.001);
        assert!(paths.geoclue_dropin_path.exists());
        assert!(journal
            .actions
            .iter()
            .any(|a| matches!(a, JournalAction::ModifiedTimezone { .. })));
        assert!(journal
            .actions
            .iter()
            .any(|a| matches!(a, JournalAction::ModifiedGeolocation { .. })));

        // Hot-swap to Netherlands without disconnecting must keep Asia/Tehran as backup!
        guard.disarm();
        let nl = find_country_preset("NL").unwrap();
        let guard2 = mgr.apply(&nl, true, true, &mut journal).unwrap();
        assert_eq!(mgr.read_active_timezone(), "Europe/Amsterdam");
        assert_eq!(
            mgr.read_backed_up_timezone().as_deref(),
            Some("Asia/Tehran")
        );

        // Dropping guard2 restores Asia/Tehran and deletes GhostPath-created geolocation files
        drop(guard2);
        assert_eq!(mgr.read_active_timezone(), "Asia/Tehran");
        assert!(!paths.geolocation_path.exists());
        assert!(!paths.geoclue_dropin_path.exists());
        assert!(!paths.timezone_backup_path.exists());
        assert!(!paths.geolocation_backup_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_restore_preserves_preexisting_geolocation_file() {
        let dir = tempdir().unwrap();
        let paths = GeoSpoofPaths::with_root(dir.path());
        seed_zoneinfo(&paths, &["UTC", "America/New_York"]);
        fs::create_dir_all(paths.geolocation_path.parent().unwrap()).unwrap();
        fs::write(&paths.timezone_file_path, "UTC\n").unwrap();
        fs::write(&paths.geolocation_path, "1.0\n2.0\n10.0\n50.0\n").unwrap();

        let mgr = GeoSpoofManager::with_paths(paths.clone(), false);
        let mut journal = UndoJournal::new();
        let us = find_country_preset("US").unwrap();
        let guard = mgr.apply(&us, true, true, &mut journal).unwrap();

        let spoofed_coords = mgr.read_active_geolocation().unwrap();
        assert!((spoofed_coords.latitude - us.latitude).abs() < 0.001);

        drop(guard);
        let restored = mgr.read_active_geolocation().unwrap();
        assert!((restored.latitude - 1.0).abs() < 0.001);
        assert!((restored.longitude - 2.0).abs() < 0.001);
        assert_eq!(mgr.read_active_timezone(), "UTC");
    }

    #[test]
    fn test_dry_run_records_journal_without_writing_files() {
        let dir = tempdir().unwrap();
        let paths = GeoSpoofPaths::with_root(dir.path());
        let mgr = GeoSpoofManager::with_paths(paths.clone(), true);
        let mut journal = UndoJournal::new();
        let de = find_country_preset("DE").unwrap();
        let guard = mgr.apply(&de, true, true, &mut journal).unwrap();

        assert_eq!(journal.actions.len(), 2);
        assert!(!paths.geolocation_path.exists());
        assert!(!paths.timezone_backup_path.exists());
        drop(guard);
    }

    #[cfg(unix)]
    #[test]
    fn test_apply_rolls_back_timezone_if_geolocation_fails() {
        let dir = tempdir().unwrap();
        let mut paths = GeoSpoofPaths::with_root(dir.path());
        seed_zoneinfo(&paths, &["Asia/Tehran", "Europe/Berlin"]);
        fs::create_dir_all(paths.localtime_path.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(
            paths.zoneinfo_dir.join("Asia/Tehran"),
            &paths.localtime_path,
        )
        .unwrap();
        fs::write(&paths.timezone_file_path, "Asia/Tehran\n").unwrap();

        // Force geolocation write to fail by pointing geolocation_path to an existing directory
        fs::create_dir_all(&paths.geolocation_path).unwrap();
        paths.use_systemd_tools = false;

        let mgr = GeoSpoofManager::with_paths(paths.clone(), false);
        let mut journal = UndoJournal::new();
        let de = find_country_preset("DE").unwrap();
        let res = mgr.apply(&de, true, true, &mut journal);
        assert!(res.is_err());

        // Timezone must have rolled back cleanly to Asia/Tehran
        assert_eq!(mgr.read_active_timezone(), "Asia/Tehran");
        assert!(!paths.timezone_backup_path.exists());
    }
}
