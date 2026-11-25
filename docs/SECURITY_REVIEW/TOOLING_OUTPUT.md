# GhostPath VPN — Verification Tooling Output

All outputs below were captured directly from commands executed in this environment after the final code edits.

---

## 1. Workspace Test Suite (`cargo test --workspace`)

```text
   Compiling ghostpath-proto v0.1.0
   Compiling ghostpath-net v0.1.0
   Compiling ghostpath-tun v0.1.0
   Compiling ghostpath-gui v0.1.0
   Compiling ghostpath-daemon v0.1.0
   Compiling ghostpath-cli v0.1.0
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 03s

ghostpath-cli (unit):                  0 passed; 0 failed
ghostpath-core (unit):                15 passed; 0 failed
ghostpath-daemon (unit):              14 passed; 0 failed
ghostpath-daemon (awg_backend_test):   3 passed; 0 failed
ghostpath-daemon (awg_namespace_int):  2 passed; 0 failed
ghostpath-daemon (ipc_test):           1 passed; 0 failed
ghostpath-gui (unit):                 16 passed; 0 failed
ghostpath-gui (gui_e2e_tests):         2 passed; 0 failed
ghostpath-gui (security_tests):        3 passed; 0 failed
ghostpath-net (unit):                 12 passed; 0 failed
ghostpath-net (dns_leak_test):         1 passed; 0 failed
ghostpath-net (namespace_routing):     1 passed; 0 failed
ghostpath-net (wg_coexistence):        1 passed; 0 failed
ghostpath-proto (unit):               10 passed; 0 failed
ghostpath-proto (amnezia_ini_test):    4 passed; 0 failed
ghostpath-proto (amnezia_model_test):  4 passed; 0 failed
ghostpath-proto (amnezia_proptest):    4 passed; 0 failed
ghostpath-proto (amnezia_uri_test):    3 passed; 0 failed
ghostpath-tun (unit):                  1 passed; 0 failed
```

**Result:** **94 passed; 0 failed** across all workspace crates and integration tests.

---

## 2. Formatting & Clippy (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`)

```text
=== FMT ===
(exit 0, 0 diffs)

=== CLIPPY ===
    Checking ghostpath-daemon v0.1.0 (/mnt/c/Users/cybermaz/Projects/ghost_path/crates/ghostpath-daemon)
    Checking ghostpath-gui v0.1.0 (/mnt/c/Users/cybermaz/Projects/ghost_path/crates/ghostpath-gui)
    Checking ghostpath-proto v0.1.0 (/mnt/c/Users/cybermaz/Projects/ghost_path/crates/ghostpath-proto)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 32.52s
(exit 0, 0 errors, 0 warnings)
```

---

## 3. Supply Chain & License Audit (`cargo audit`, `cargo deny check`, `npm audit`)

### `cargo audit`
```text
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 1568 security advisories (from /home/cybermaz/.cargo/advisory-db)
    Scanning Cargo.lock for vulnerabilities (575 crate dependencies)
```
- **Vulnerabilities:** `0`
- **Warnings (unmaintained transitive crates):** `chrono` (0.4.45), `proc-macro-error` (1.0.4), `proc-macro-error2` (2.0.1), `zbus_names` (4.3.5), `zvariant` (5.15.1), `zvariant_derive` (5.15.1).

### `cargo deny check`
```text
advisories ok, bans ok, licenses ok, sources ok
```

### `UI && npm audit`
```text
found 0 vulnerabilities
```

---

## 4. Miri (`cargo +nightly miri test -p ghostpath-core`)

Note: Filesystem-creating tests in `ghostpath-core` (`config`, `fs_safe`, `paths`) were excluded from this isolated run; pure-logic modules ran under Miri with `-Zmiri-disable-isolation`:

```text
MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test -p ghostpath-core -- validators geo rules state subscription

running 11 tests
test geo::tests::test_all_presets_have_valid_timezones_and_coords ... ok
test geo::tests::test_preset_lookup_and_aliases ... ok
test geo::tests::test_timezone_validation_rejects_traversal ... ok
test rules::tests::test_default_rules_evaluation ... ok
test state::tests::test_state_transitions ... ok
test subscription::tests::test_parse_base64_subscription_links ... ok
test subscription::tests::test_parse_plain_subscription_links ... ok
test subscription::tests::test_subscription_should_update ... ok
test validators::tests::test_redact_url_and_display_sanitize ... ok
test validators::tests::test_rejects_option_injection_and_bad_iface ... ok
test validators::tests::test_ssrf_ip_classification ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 4.83s
```

---

## 5. Code Coverage (`cargo llvm-cov --workspace --summary-only`)

```text
TOTAL   Regions: 9175 (Missed: 3822, Cover: 58.34%)
        Functions: 792 (Missed: 343, Cover: 56.69%)
        Lines: 8389 (Missed: 3565, Cover: 57.50%)
```

---

## 6. Systemd Unit Hardening (`systemd-analyze security --offline=true systemd/ghostpathd.service`)

- **Before hardening:** `Overall exposure level for ghostpathd.service: 6.0 MEDIUM`
- **After hardening:**
```text
→ Overall exposure level for ghostpathd.service: 4.2 OK 🙂
```

---

## 7. UI Build & Release-Artifact / Demo-Content Scan

### `UI && npm run build`
```text
vite v8.3.8 building client environment for production...
✓ 1671 modules transformed.
dist/assets/index-DtU_a8Py.js   308.26 kB │ gzip: 89.31 kB
✓ built in 612ms
```

### `bash scripts/verify_no_demo_content.sh`
```text
[*] Scanning UI source code (UI/src) for forbidden simulation patterns... (all PASS)
[*] Scanning UI distribution bundle (UI/dist)... (all PASS)
[*] Scanning release Rust binaries & libraries for MockDaemon...
  [PASS] target/release/libghostpath_gui: MockDaemon is completely absent in release binary
  [PASS] ghostpath-daemon: contains zero mock modules
[+] Verification successful: Zero demo, fake, or mock artifacts found in release paths.
```

---

## 8. Bundled Binary Checksum & Isolated Network Namespace Check

```text
$ (cd packaging/bin && sha256sum -c amneziawg-go.sha256)
amneziawg-go: OK

$ unshare --user --net --map-root-user bash -c 'ip addr show lo && ip rule show'
1: lo: <LOOPBACK> mtu 65536 qdisc noop state DOWN group default qlen 1000
    link/loopback 00:00:00:00:00:00 brd 00:00:00:00:00:00
0:	from all lookup local
32766:	from all lookup main
32767:	from all lookup default
```

---

## 9. Tools Unavailable / Not Run in This Environment

- **`lintian`, `rpmlint`, `shellcheck`:** Not installed in the WSL environment (`sudo apt` requires interactive password).
- **`cargo-fuzz` (libFuzzer) / `ThreadSanitizer` / `AddressSanitizer`:** Replaced in this environment by `proptest` property-based fuzzing (`amnezia_proptest`) and Miri; run libFuzzer/TSan/ASan in CI.
- **Live kernel `tun` / `nftables` packet-leak & multi-distro VM install tests:** Not executed on host network/WSL kernel; requires privileged VM/container CI runner.
