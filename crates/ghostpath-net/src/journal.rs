use std::fs;
use std::path::Path;

use ghostpath_core::error::{CoreError, Result};
use ghostpath_core::fs_safe::{atomic_write_private, read_to_string_no_follow};
use ghostpath_core::paths::GhostPaths;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "details", rename_all = "snake_case")]
pub enum JournalAction {
    AddedIpRule {
        priority: u32,
        fwmark: Option<u32>,
        table: u32,
        description: String,
    },
    AddedRoute {
        destination: String,
        table: u32,
        device: String,
    },
    CreatedInterface {
        name: String,
    },
    ModifiedDns {
        original_backup_path: String,
        method: String,
    },
    CreatedNftTable {
        family: String,
        name: String,
    },
    SysctlChanged {
        key: String,
        original_value: String,
    },
    ModifiedTimezone {
        backup_path: String,
        original_timezone: String,
    },
    ModifiedGeolocation {
        backup_path: String,
        dropin_path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UndoJournal {
    pub actions: Vec<JournalAction>,
    pub created_at: u64,
}

impl UndoJournal {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            actions: Vec::new(),
            created_at: now,
        }
    }

    pub fn record(&mut self, action: JournalAction) {
        self.actions.push(action);
    }

    pub fn save(&self) -> Result<()> {
        let path = GhostPaths::undo_journal_file();
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        atomic_write_private(path, json.as_bytes())?;
        Ok(())
    }

    pub fn load() -> Result<Option<Self>> {
        let path = GhostPaths::undo_journal_file();
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = read_to_string_no_follow(path, 256 * 1024)?;
        let journal: Self =
            serde_json::from_str(&content).map_err(|e| CoreError::Serialization(e.to_string()))?;
        Ok(Some(journal))
    }

    pub fn remove_journal_file() -> Result<()> {
        let path = GhostPaths::undo_journal_file();
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_journal_serialization_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("undo_journal.json");

        let mut journal = UndoJournal::new();
        journal.record(JournalAction::AddedIpRule {
            priority: 5180,
            fwmark: Some(0x4750),
            table: 5180,
            description: "Bypass rule".to_string(),
        });
        journal.record(JournalAction::CreatedInterface {
            name: "ghost0".to_string(),
        });

        journal.save_to_path(&path).unwrap();
        assert!(path.exists());

        let loaded = UndoJournal::load_from_path(&path).unwrap().unwrap();
        assert_eq!(loaded.actions.len(), 2);
        assert_eq!(loaded.actions[0], journal.actions[0]);
    }
}
