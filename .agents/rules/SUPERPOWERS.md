# SUPERPOWERS: Operating Rules for GhostPath VPN (Rust Linux VPN Client)

These rules are always active. Read this file at the start of every session and before every phase. If a rule conflicts with a user instruction, follow the user and note the deviation in `docs/DECISIONS.md`.

---

## 0. Mission (never lose sight of it)

1. **Coexistence:** other VPN apps (WireGuard, OpenVPN, etc.) must be able to connect *through* our tunnel, and must not break us.
2. **Working Linux TUN mode** with V2Ray-family configs (VLESS, VMess, Trojan, Shadowsocks; Reality/WS/gRPC/TLS) on heavily filtered networks. No routing loops, no DNS leaks, no IPv6 leaks, works with NetworkManager, systemd-resolved, and Docker.
3. Written in **Rust**.

Every decision is judged against these three points.

---

## 1. Workflow Discipline

- **Spec first, code second.** For each phase: write a short plan (goal, files touched, risks, tests), then implement, then verify, then summarize.
- **One phase at a time.** Do not start phase N+1 until phase N compiles, passes tests, and is committed.
- **Small, reversible steps.** Prefer many small commits over one large change. Each commit builds and passes tests.
- **Read before writing.** Inspect existing code, `docs/ARCHITECTURE.md`, and `docs/DECISIONS.md` before changing anything. Do not duplicate existing abstractions.
- **Verify, don't assume.** Run `cargo build`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` after every meaningful change. Report real output, never claim success without running it.
- **Record decisions.** Any ambiguity or non-obvious choice goes into `docs/DECISIONS.md` (date, context, decision, alternatives). Ask the user only for blocking decisions.
- **No scope creep.** Do not add features that are not in the current phase. Park ideas in `docs/BACKLOG.md`.

---

## 2. Safety Rules for Network Code (critical)

This project rewrites routes, rules, DNS, and firewall state. A bug can cut the developer off from the network.

- **Never test network-mutating code on the host's real network namespace.** Use network namespaces (`unshare -n`, `ip netns`), veth pairs, or a container/VM. Host-touching tests must be behind an explicit opt-in flag (e.g. `GHOSTPATH_ALLOW_HOST_TESTS=1`).
- **Every mutation has a guaranteed undo.** Use RAII guards, a persistent "undo journal" on disk, signal handlers (SIGINT, SIGTERM, SIGHUP), a panic hook, and startup recovery for stale state after `SIGKILL`.
- **Idempotent apply and cleanup.** Running either twice must be harmless.
- **Own your objects.** Tag every route, `ip rule`, nft table/chain, and routing table with our identifiers (a dedicated fwmark, table id, nft table name `ghostpath`). Never flush or delete things we did not create.
- **No blanket flushes.** Never `ip route flush`, `iptables -F`, `nft flush ruleset`, or overwrite `/etc/resolv.conf` without a backup and restore path.
- **Dry-run mode.** Every network-changing command supports `--dry-run` that prints the exact plan (routes, rules, nft, DNS) without applying it.
- **Destructive or privileged shell commands need a stated reason.** Before running anything with `sudo`, `ip`, `nft`, or `iptables` on the host, say what it changes and how to undo it.
- **Never exfiltrate or log secrets.** Server addresses, UUIDs, passwords, Reality keys, and subscription URLs are redacted in logs by default.

---

## 3. Rust Standards

- Edition 2021+, stable toolchain pinned in `rust-toolchain.toml`.
- Cargo workspace with the crates defined in the project prompt; keep crate boundaries clean (no cyclic dependencies, no CLI/GUI logic in core crates).
- `#![forbid(unsafe_code)]` by default. `unsafe` only in small, isolated wrapper modules, each block with a `// SAFETY:` comment explaining the invariant.
- Errors: `thiserror` for libraries, `anyhow` only in binaries. No `unwrap()`/`expect()` in non-test code unless provably infallible (explain in a comment).
- Async: Tokio. No blocking calls on the async runtime; use `spawn_blocking` where needed. Every long-running task is cancellable and shut down gracefully.
- Logging: `tracing` with structured fields and levels. No `println!` outside the CLI's user-facing output.
- Traits for seams that need swapping or mocking (`Outbound`, `RouteManager`, `DnsManager`, `Firewall`). Real implementations and in-memory fakes both exist so logic is unit-testable without root.
- Keep dependencies minimal and well-maintained. Before adding a crate, check maintenance status, license (MIT/Apache-2.0 preferred), and whether `std`/an existing dependency already covers it. Record the choice in `DECISIONS.md`.
- Public items have doc comments. Modules have a short header explaining their responsibility.

---

## 4. Testing Rules

- Test pyramid: many unit tests, focused integration tests, few end-to-end tests.
- **Pure logic is tested without privileges:** config/link parsing, route-plan generation, rule building, DNS-plan selection, state machine transitions.
- **Integration tests** run in namespaces/veth setups and cover at minimum: routing-loop prevention, DNS leak check, IPv6 leak check, crash recovery after `SIGKILL`, WireGuard-over-tunnel coexistence.
- Every bug fix starts with a failing regression test.
- Parsers get fuzz or property tests (`proptest`/`cargo-fuzz`) for `vless://`, `vmess://`, `trojan://`, `ss://`, and subscription formats, including malformed and hostile input.
- CI must run: fmt, clippy (deny warnings), unit tests, `cargo deny` (licenses/advisories), and the namespace integration suite on a privileged runner.
- A test that is flaky is a bug. Fix or quarantine it with an issue note; never retry-until-green silently.

---

## 5. Linux Networking Playbook

Use these patterns by default and justify any deviation:

- **Routing loop prevention:** mark the proxy core's outbound sockets (`SO_MARK`), add `ip rule fwmark <mark> lookup main` (or the bypass table) at higher priority than the tunnel rule. Alternative: cgroup/UID-based exclusion. Do not rely only on a `/32` route to the server IP.
- **Tunnel capture:** dedicated routing table plus `ip rule` for traffic to the TUN, or split defaults (`0.0.0.0/1`, `128.0.0.0/1`). Leave priority gaps so other VPN interfaces can insert their own rules (coexistence).
- **Netlink over shelling out.** Prefer `rtnetlink`/`netlink-*` crates for routes, rules, links, addresses. Shell out only when no reliable crate exists, and capture and parse the error output.
- **DNS:** detect the active stack (systemd-resolved, NetworkManager, plain resolv.conf) and use per-link DNS via resolved over D-Bus (`zbus`) when available. Handle DNS inside the tunnel (fake-IP or DoH/DoT through the proxy) so ISP resolvers never see queries.
- **IPv6:** tunnel it fully or blackhole it. Never leave it half-configured.
- **MTU/MSS:** choose a conservative TUN MTU, clamp MSS on forwarded TCP, make it configurable.
- **Docker/containers:** detect `docker0`, bridge networks, and their iptables/nft rules; make sure our rules neither break nor are bypassed by container traffic. Document behavior.
- **Interface exclusions:** configurable list of interfaces/CIDRs/UIDs that bypass or chain (`wg*`, `tailscale*`, LAN ranges).
- **Privilege model:** unprivileged GUI/CLI talks to a privileged daemon over a Unix socket with `SO_PEERCRED` checks. Prefer minimal capabilities (`CAP_NET_ADMIN`, `CAP_NET_RAW`) over full root. The daemon runs as a hardened systemd service (`NoNewPrivileges`, `ProtectSystem`, restricted capability set).

---

## 6. Filtered-Network Resilience

- Support Reality, uTLS fingerprints, TLS fragmentation, WS/gRPC/HTTPUpgrade transports, SNI/host overrides, and multiple fallback servers.
- Never hard-code a single endpoint or a single DNS provider.
- Connection health checks with automatic failover; exponential backoff with jitter; clear user-visible state (`Connecting`, `Connected`, `Degraded`, `Failed`) with a reason.
- Geo data (geoip/geosite) is updatable and ships with a bundled fallback so it works offline at first run.

---

## 7. Security Rules

- No telemetry, no analytics, no phoning home.
- Validate and bound all external input (links, subscriptions, JSON, IPC messages). Subscription fetching goes through the tunnel or a user-chosen path, with size/time limits.
- Store config under `~/.config/ghostpath` with `0600` permissions. Do not store secrets in world-readable files or in logs.
- IPC messages are versioned, authenticated, and rate-limited. The daemon never executes user-supplied strings as commands.
- Run `cargo audit`/`cargo deny` before each release. Pin and review any dependency that touches crypto or networking.

---

## 8. UX and CLI Rules

- CLI commands: `connect`, `disconnect`, `status`, `import`, `doctor`, `logs`. Output is human-readable by default and `--json` for scripting.
- `ghostpath doctor` checks: kernel TUN support, permissions/capabilities, resolved/NM state, Docker rules, IPv6, `rp_filter`, `ip_forward`, conflicting VPN interfaces, stale `ghostpath` state. Each failure prints the cause and the exact fix.
- Error messages say what happened, why, and what to do next. No raw stack traces for expected failures.
- Provide English and Persian (Farsi) docs and UI strings from the start (i18n-ready, RTL-safe in the GUI).

---

## 9. Git and Documentation

- Conventional Commits (`feat:`, `fix:`, `test:`, `docs:`, `refactor:`, `chore:`), one logical change per commit.
- Keep these files current: `README.md`, `docs/ARCHITECTURE.md`, `docs/DECISIONS.md`, `docs/BACKLOG.md`, `docs/TROUBLESHOOTING.md`.
- After each phase, write a short phase report: what was built, what was tested (with command output summary), known gaps, next phase risks.

---

## 10. Definition of Done (per phase and overall)

A phase is done only when:

- [ ] Code compiles with no warnings; `fmt`, `clippy -D warnings`, and all tests pass
- [ ] New logic has unit tests; new network behavior has namespace integration tests
- [ ] Cleanup and crash-recovery paths are tested
- [ ] No secrets in logs; `doctor` updated if new failure modes were introduced
- [ ] Docs and `DECISIONS.md` updated; phase report written

The project is done when:

- [ ] Full-system TUN with a VLESS/Reality config works on stock Ubuntu with NetworkManager + systemd-resolved **and** Docker running, with no Docker-based workaround
- [ ] WireGuard/OpenVPN connect through the tunnel
- [ ] Automated tests show no DNS, IPv6, or route leaks
- [ ] `SIGKILL` on the daemon leaves the network recoverable automatically

---

## 11. Behavior When Stuck

1. Reproduce the problem in a namespace and capture evidence (`ip rule`, `ip route show table all`, `nft list ruleset`, `resolvectl status`, `tcpdump` output).
2. Form a hypothesis, test one variable at a time, record findings.
3. If blocked after two honest attempts, stop, summarize what you tried and what you learned, and ask the user a specific question instead of guessing.
