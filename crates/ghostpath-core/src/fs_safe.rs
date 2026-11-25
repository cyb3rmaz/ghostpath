use std::fs::{self, OpenOptions};
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

use crate::error::{CoreError, Result};

/// O_NOFOLLOW constant for Linux/Unix open flags without requiring extra unsafe wrappers.
#[cfg(all(unix, target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0o400_000;
#[cfg(all(unix, not(target_os = "linux")))]
const O_NOFOLLOW_FLAG: i32 = 0x0100;

/// Ensures a directory exists, is not a symlink, and has mode `0700` on Unix.
pub fn ensure_private_dir(dir: &Path) -> Result<()> {
    if let Ok(meta) = fs::symlink_metadata(dir) {
        if meta.file_type().is_symlink() {
            return Err(CoreError::Config(format!(
                "Refusing to use symlinked private directory: {}",
                dir.display()
            )));
        }
        if !meta.is_dir() {
            return Err(CoreError::Config(format!(
                "Expected directory but found non-directory: {}",
                dir.display()
            )));
        }
    } else {
        fs::create_dir_all(dir)?;
    }

    #[cfg(unix)]
    {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }

    Ok(())
}

/// Validates that an existing file target is a regular file (never symlink or multi-hardlink).
pub fn verify_safe_regular_target(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err(CoreError::Config(format!(
                    "Refusing symlink target: {}",
                    path.display()
                )));
            }
            if !meta.is_file() {
                return Err(CoreError::Config(format!(
                    "Refusing non-regular file target: {}",
                    path.display()
                )));
            }
            #[cfg(unix)]
            if meta.nlink() > 1 {
                return Err(CoreError::Config(format!(
                    "Refusing hardlinked file target (nlink={}): {}",
                    meta.nlink(),
                    path.display()
                )));
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(CoreError::Io(e)),
    }
}

/// Atomically writes sensitive bytes to `path` using a `0700` parent directory,
/// `O_CREAT | O_EXCL | O_NOFOLLOW` temp file with mode `0600` (or `mode`), `sync_all`, and rename.
pub fn atomic_write_private(path: &Path, data: &[u8]) -> Result<()> {
    atomic_write_with_mode(path, data, 0o600)
}

/// Atomically writes bytes to `path` with explicit Unix permissions inside a `0700` private dir.
pub fn atomic_write_with_mode(path: &Path, data: &[u8], mode: u32) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        CoreError::Config(format!("Path has no parent directory: {}", path.display()))
    })?;
    ensure_private_dir(parent)?;
    atomic_write_in_dir(parent, path, data, mode)
}

/// Atomically writes a system file (e.g. `/etc/resolv.conf`, `/etc/geolocation`) without altering
/// the parent directory's permissions, while still rejecting symlinks and hardlinks.
pub fn atomic_write_system_file(path: &Path, data: &[u8], mode: u32) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        CoreError::Config(format!("Path has no parent directory: {}", path.display()))
    })?;
    if let Ok(meta) = fs::symlink_metadata(parent) {
        if meta.file_type().is_symlink() {
            return Err(CoreError::Config(format!(
                "Refusing to write inside symlinked directory: {}",
                parent.display()
            )));
        }
        if !meta.is_dir() {
            return Err(CoreError::Config(format!(
                "Expected directory but found non-directory: {}",
                parent.display()
            )));
        }
    } else {
        fs::create_dir_all(parent)?;
    }
    atomic_write_in_dir(parent, path, data, mode)
}

fn atomic_write_in_dir(parent: &Path, path: &Path, data: &[u8], mode: u32) -> Result<()> {
    verify_safe_regular_target(path)?;

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("ghostpath");
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let tmp_path = parent.join(format!(".{file_name}.tmp.{nonce}"));

    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        opts.mode(mode).custom_flags(O_NOFOLLOW_FLAG);
    }

    let write_res = (|| -> Result<()> {
        use std::io::Write;
        let mut file = opts.open(&tmp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
        #[cfg(unix)]
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(mode))?;
        Ok(())
    })();

    if let Err(err) = write_res {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }

    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(CoreError::Io(err));
    }

    #[cfg(unix)]
    {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }

    Ok(())
}

/// Reads a regular file safely without following symlinks and bounded by `max_bytes`.
pub fn read_to_string_no_follow(path: &Path, max_bytes: usize) -> Result<String> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        return Err(CoreError::Config(format!(
            "Refusing to read symlink: {}",
            path.display()
        )));
    }
    if !meta.is_file() {
        return Err(CoreError::Config(format!(
            "Refusing to read non-regular file: {}",
            path.display()
        )));
    }

    let mut opts = OpenOptions::new();
    opts.read(true);
    #[cfg(unix)]
    opts.custom_flags(O_NOFOLLOW_FLAG);

    let file = opts.open(path)?;
    let mut limited = file.take((max_bytes as u64).saturating_add(1));
    let mut buf = String::new();
    limited.read_to_string(&mut buf)?;
    if buf.len() > max_bytes {
        return Err(CoreError::Config(format!(
            "File {} exceeds maximum allowed size of {} bytes",
            path.display(),
            max_bytes
        )));
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write_and_read_no_follow() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("secret.toml");
        atomic_write_private(&target, b"token=abc").unwrap();

        #[cfg(unix)]
        {
            let mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        let got = read_to_string_no_follow(&target, 1024).unwrap();
        assert_eq!(got, "token=abc");
    }

    #[cfg(unix)]
    #[test]
    fn test_refuses_symlink_write_and_read() {
        let dir = tempdir().unwrap();
        let victim = dir.path().join("victim");
        fs::write(&victim, b"untouched").unwrap();
        let link = dir.path().join("link.toml");
        std::os::unix::fs::symlink(&victim, &link).unwrap();

        assert!(atomic_write_private(&link, b"pwned").is_err());
        assert!(read_to_string_no_follow(&link, 1024).is_err());
        assert_eq!(fs::read_to_string(&victim).unwrap(), "untouched");
    }

    #[cfg(unix)]
    #[test]
    fn test_atomic_write_system_file_preserves_parent_mode() {
        let dir = tempdir().unwrap();
        let etc_like = dir.path().join("etc_sim");
        fs::create_dir_all(&etc_like).unwrap();
        fs::set_permissions(&etc_like, fs::Permissions::from_mode(0o755)).unwrap();

        let target = etc_like.join("resolv.conf");
        atomic_write_system_file(&target, b"nameserver 1.1.1.1\n", 0o644).unwrap();

        let dir_mode = fs::metadata(&etc_like).unwrap().permissions().mode() & 0o777;
        let file_mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            dir_mode, 0o755,
            "parent dir permissions must not be altered"
        );
        assert_eq!(file_mode, 0o644, "system file must be world-readable 0644");
    }
}
