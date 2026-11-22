pub const GHOST_FWMARK: u32 = 0x4750;
pub const GHOST_ROUTING_TABLE: u32 = 5180;
pub const GHOST_RULE_PRIO_BYPASS: u32 = 5180;
pub const GHOST_RULE_PRIO_CAPTURE: u32 = 5181;
pub const GHOST_INTERFACE_NAME: &str = "ghost0";
pub const GHOST_NFT_TABLE: &str = "ghostpath";

pub mod coexistence;
pub mod dns;
pub mod dns_proxy;
pub mod journal;
pub mod killswitch;
pub mod manager;

pub use coexistence::{ChainMode, CoexistenceConfig, CoexistenceGuard, CoexistenceManager};
pub use dns::{DnsGuard, DnsManager, DnsStrategy};
pub use dns_proxy::{DnsProxy, DEFAULT_DNS_PORT};
pub use journal::{JournalAction, UndoJournal};
pub use killswitch::{KillSwitch, KillSwitchGuard};
pub use manager::{RouteGuard, RouteManager};
