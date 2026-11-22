pub const UI_ASSETS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../UI/dist");

pub fn get_ui_dist_path() -> std::path::PathBuf {
    std::path::PathBuf::from(UI_ASSETS_DIR)
}
