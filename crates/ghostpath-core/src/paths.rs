use std::path::PathBuf;

pub struct GhostPaths;

impl GhostPaths {
    /// User configuration directory (~/.config/ghostpath)
    pub fn config_dir() -> PathBuf {
        if let Ok(override_dir) = std::env::var("GHOSTPATH_CONFIG_DIR") {
            return PathBuf::from(override_dir);
        }
        dirs_fallback_config()
    }

    /// User configuration file (~/.config/ghostpath/config.toml)
    pub fn config_file() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// Profiles file (~/.config/ghostpath/profiles.json)
    pub fn profiles_file() -> PathBuf {
        Self::config_dir().join("profiles.json")
    }

    /// System state directory (/var/lib/ghostpath)
    pub fn state_dir() -> PathBuf {
        if let Ok(override_dir) = std::env::var("GHOSTPATH_STATE_DIR") {
            return PathBuf::from(override_dir);
        }
        PathBuf::from("/var/lib/ghostpath")
    }

    /// Runtime socket path (/run/ghostpath/ghostpathd.sock)
    pub fn socket_path() -> PathBuf {
        if let Ok(override_path) = std::env::var("GHOSTPATH_SOCKET") {
            return PathBuf::from(override_path);
        }
        PathBuf::from("/run/ghostpath/ghostpathd.sock")
    }

    /// Undo journal file for crash recovery (/var/lib/ghostpath/undo_journal.json)
    pub fn undo_journal_file() -> PathBuf {
        Self::state_dir().join("undo_journal.json")
    }

    /// Resolv.conf backup (/var/lib/ghostpath/resolv.conf.backup)
    pub fn resolv_conf_backup() -> PathBuf {
        Self::state_dir().join("resolv.conf.backup")
    }

    /// Timezone backup (/var/lib/ghostpath/timezone.backup.json)
    pub fn timezone_backup() -> PathBuf {
        Self::state_dir().join("timezone.backup.json")
    }

    /// Runtime directory (/run/ghostpath)
    pub fn runtime_dir() -> PathBuf {
        if let Ok(override_dir) = std::env::var("GHOSTPATH_RUNTIME_DIR") {
            return PathBuf::from(override_dir);
        }
        PathBuf::from("/run/ghostpath")
    }

    /// Private directory for ephemeral proxy core configs (/run/ghostpath/cores)
    pub fn runtime_core_dir() -> PathBuf {
        Self::runtime_dir().join("cores")
    }

    /// GeoClue geolocation backup (/var/lib/ghostpath/geolocation.backup.json)
    pub fn geolocation_backup() -> PathBuf {
        Self::state_dir().join("geolocation.backup.json")
    }
}

fn dirs_fallback_config() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            return PathBuf::from(xdg).join("ghostpath");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home).join(".config").join("ghostpath");
        }
    }
    // Never fall back to world-writable /tmp when HOME/XDG are unset (e.g. systemd units)
    GhostPaths::state_dir().join("config")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirs_fallback_never_uses_tmp() {
        let state = GhostPaths::state_dir().join("config");
        assert!(!state.starts_with("/tmp"));
        assert!(!GhostPaths::runtime_core_dir().starts_with("/tmp"));
    }
}
