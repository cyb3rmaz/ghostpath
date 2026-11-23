use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn get_repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../../")
}

#[test]
fn test_tauri_conf_security_csp() {
    let conf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    assert!(conf_path.exists(), "tauri.conf.json must exist");

    let content = fs::read_to_string(&conf_path).expect("Read tauri.conf.json");
    let json: Value = serde_json::from_str(&content).expect("Valid JSON");

    let csp = json["app"]["security"]["csp"]
        .as_str()
        .expect("CSP must be configured in app.security.csp");

    // Strict offline security checks
    assert!(csp.contains("default-src 'self'"), "CSP must restrict default-src to 'self'");
    assert!(csp.contains("connect-src 'none'"), "CSP must block all network connections with connect-src 'none'");
    assert!(csp.contains("script-src 'self'"), "CSP must restrict scripts to 'self'");
    assert!(csp.contains("font-src 'self'"), "CSP must restrict fonts to 'self'");
    assert!(!csp.contains("http://"), "CSP must not allow insecure http origins");
    assert!(!csp.contains("https://"), "CSP must not allow external https origins");
}

#[test]
fn test_capabilities_lockdown() {
    let cap_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("capabilities/default.json");
    assert!(cap_path.exists(), "capabilities/default.json must exist");

    let content = fs::read_to_string(&cap_path).expect("Read capabilities/default.json");
    let json: Value = serde_json::from_str(&content).expect("Valid JSON");

    let perms = json["permissions"]
        .as_array()
        .expect("Permissions must be an array");

    let forbidden_prefixes = ["shell:", "fs:", "http:", "opener:", "process:"];
    for perm in perms {
        let perm_str = perm.as_str().expect("Permission must be string");
        for forbidden in &forbidden_prefixes {
            assert!(
                !perm_str.starts_with(forbidden),
                "Forbidden dangerous capability '{}' must not be enabled",
                perm_str
            );
        }
    }
}

#[test]
fn test_ui_no_remote_network_links() {
    let repo_root = get_repo_root();
    let index_src = repo_root.join("UI/index.html");
    assert!(index_src.exists(), "UI/index.html must exist");

    let src_content = fs::read_to_string(&index_src).expect("Read UI/index.html");
    assert!(!src_content.contains("fonts.googleapis.com"), "No external Google Fonts allowed");
    assert!(!src_content.contains("fonts.gstatic.com"), "No external font preconnect allowed");

    // If dist exists, check dist/index.html as well
    let dist_index = repo_root.join("UI/dist/index.html");
    if dist_index.exists() {
        let dist_content = fs::read_to_string(&dist_index).expect("Read UI/dist/index.html");
        assert!(!dist_content.contains("http://"), "Built dist must contain zero remote http references");
        assert!(!dist_content.contains("https://"), "Built dist must contain zero remote https references");
    }
}
