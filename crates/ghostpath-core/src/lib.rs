#![forbid(unsafe_code)]

pub mod config;
pub mod crypto;
pub mod error;
pub mod event;
pub mod fs_safe;
pub mod geo;
pub mod models;
pub mod paths;
pub mod rules;
pub mod state;
pub mod subscription;
pub mod validators;

pub use config::GhostConfig;
pub use error::{CoreError, Result};
pub use event::{EventBus, GhostEvent};
pub use fs_safe::{
    atomic_write_private, atomic_write_system_file, atomic_write_with_mode, ensure_private_dir,
    read_to_string_no_follow,
};
pub use geo::{
    find_country_preset, geo_country_presets, validate_timezone_name, GeoCoordinates,
    GeoCountryPreset, GeoSpoofMode, GeoSpoofSettings, GeoSpoofStatus,
};
pub use models::*;
pub use paths::GhostPaths;
pub use rules::{default_routing_rules, RuleEngine};
pub use state::{ConnectionState, StateManager};
pub use subscription::SubscriptionManager;
pub use validators::{
    is_public_ip, redact_url_for_log, sanitize_display_text, validate_cidr, validate_dns_endpoint,
    validate_host_port, validate_iface_glob, validate_iface_name, validate_ip_addr,
    validate_local_port, validate_mtu, validate_tun_stack, validate_uapi_value,
};
