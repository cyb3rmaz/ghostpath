pub const GHOST_FWMARK: u32 = 0x4750;
pub const GHOST_ROUTING_TABLE: u32 = 5180;
pub const GHOST_RULE_PRIO_BYPASS: u32 = 5180;
pub const GHOST_RULE_PRIO_CAPTURE: u32 = 5181;
pub const GHOST_INTERFACE_NAME: &str = "ghost0";
pub const GHOST_NFT_TABLE: &str = "ghostpath";

pub mod journal;

pub use journal::{JournalAction, UndoJournal};
