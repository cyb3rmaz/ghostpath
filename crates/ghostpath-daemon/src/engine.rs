use crate::doctor::SystemDoctor;
use crate::tunnel::{AwgBackend, TunnelBackend};
use anyhow::{bail, Result};
use ghostpath_core::geo::{
    find_country_preset, geo_country_presets, GeoCountryPreset, GeoSpoofMode, GeoSpoofSettings,
    GeoSpoofStatus,
};
use ghostpath_core::models::{
    ConnectionStatus, CoreEngine, NodeProtocol, RuleAction, RuleType, ServerNode,
};
use ghostpath_core::GhostConfig;
use ghostpath_net::{
    DnsGuard, DnsManager, DnsProxy, GeoSpoofGuard, GeoSpoofManager, GeoSpoofPaths, KillSwitch,
    KillSwitchGuard, RouteGuard, RouteManager, UndoJournal,
};
use ghostpath_proto::{parse_any_link, ManagedCoreOutbound, Outbound};
use ghostpath_tun::TunDevice;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonMetrics {
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub total_down_mb: f64,
    pub total_up_mb: f64,
    pub session_seconds: u64,
}

pub struct ActiveSession {
    pub node: ServerNode,
    pub start_time: Instant,
    pub route_guard: Option<RouteGuard>,
    pub dns_guard: Option<DnsGuard>,
    pub dns_proxy: Option<DnsProxy>,
    pub killswitch_guard: Option<KillSwitchGuard>,
    pub tun_device: Option<TunDevice>,
    pub core_process: Option<ManagedCoreOutbound>,
    pub awg_backend: Option<AwgBackend>,
    pub geo_spoof_guard: Option<GeoSpoofGuard>,
}

pub struct DaemonEngine {
    config: Arc<RwLock<GhostConfig>>,
    status: Arc<RwLock<ConnectionStatus>>,
    session: Arc<RwLock<Option<ActiveSession>>>,
    dry_run: bool,
    log_buffer: Arc<RwLock<Vec<String>>>,
    bytes_in: Arc<AtomicU64>,
    bytes_out: Arc<AtomicU64>,
    last_traffic: Arc<RwLock<(Instant, u64, u64)>>,
    geo_paths: GeoSpoofPaths,
    manual_geo_guard: Arc<RwLock<Option<GeoSpoofGuard>>>,
    applied_geo: Arc<RwLock<Option<(String, String, String)>>>,
}

impl DaemonEngine {
    pub fn new(dry_run: bool) -> Self {
        Self::with_geo_paths(dry_run, GeoSpoofPaths::system())
    }

    pub fn with_geo_paths(dry_run: bool, geo_paths: GeoSpoofPaths) -> Self {
        let config = GhostConfig::load().unwrap_or_default();
        Self {
            config: Arc::new(RwLock::new(config)),
            status: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            session: Arc::new(RwLock::new(None)),
            dry_run,
            log_buffer: Arc::new(RwLock::new(Vec::new())),
            bytes_in: Arc::new(AtomicU64::new(0)),
            bytes_out: Arc::new(AtomicU64::new(0)),
            last_traffic: Arc::new(RwLock::new((Instant::now(), 0, 0))),
            geo_paths,
            manual_geo_guard: Arc::new(RwLock::new(None)),
            applied_geo: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn append_log(&self, msg: String) {
        let mut logs = self.log_buffer.write().await;
        if logs.len() >= 500 {
            logs.remove(0);
        }
        logs.push(msg);
    }

    pub async fn connect(&self, node_id: Option<String>) -> Result<serde_json::Value> {
        let mut status = self.status.write().await;
        if *status == ConnectionStatus::Connected || *status == ConnectionStatus::Connecting {
            bail!("A connection is already in progress or active");
        }
        *status = ConnectionStatus::Connecting;
        self.append_log("Initiating VPN tunnel connection...".to_string())
            .await;

        let config = self.config.read().await.clone();
        let target_node = match node_id {
            Some(id) => config.nodes.iter().find(|n| n.id == id).cloned(),
            None => {
                if let Some(ref active_id) = config.active_node_id {
                    config
                        .nodes
                        .iter()
                        .find(|n| &n.id == active_id)
                        .cloned()
                        .or_else(|| config.nodes.first().cloned())
                } else {
                    config.nodes.first().cloned()
                }
            }
        };

        let node = match target_node {
            Some(n) => n,
            None => {
                *status = ConnectionStatus::Disconnected;
                bail!("No server profiles available. Please import a profile first.");
            }
        };

        info!(node_id = %node.id, name = %node.name, "Establishing GhostPath VPN tunnel");

        let is_awg_protocol = node.protocol == NodeProtocol::AmneziaWG || node.protocol == NodeProtocol::WireGuard;
        let iface_name = if is_awg_protocol { "awg-ghost0" } else { "ghost0" };

        let (tun_device, route_guard, dns_guard, dns_proxy, killswitch_guard, core_process, awg_backend) =
            if !self.dry_run {
                if is_awg_protocol {
                    // AWG tunnel pipeline
                    let awg_profile = if let Some(ref raw_conf) = node.raw_link {
                        if raw_conf.contains("[Interface]") {
                            ghostpath_proto::amnezia::parse_awg_conf(raw_conf, &node.name).ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }.unwrap_or_else(|| {
                        ghostpath_proto::amnezia::AwgProfileConfig {
                            id: node.id.clone(),
                            name: node.name.clone(),
                            interface: ghostpath_proto::amnezia::AwgInterfaceConfig {
                                private_key: "YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=".to_string(),
                                address: vec!["10.8.0.2/24".to_string()],
                                dns: vec![config.settings.dns_primary.clone()],
                                mtu: Some(config.settings.mtu),
                                obfuscation: ghostpath_proto::amnezia::AwgObfuscationConfig::default(),
                            },
                            peer: ghostpath_proto::amnezia::AwgPeerConfig {
                                public_key: "YmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmI=".to_string(),
                                preshared_key: None,
                                endpoint: format!("{}:{}", node.host, node.port),
                                allowed_ips: vec!["0.0.0.0/0".to_string(), "::/0".to_string()],
                                persistent_keepalive: Some(25),
                            },
                        }
                    });

                    let mut journal = UndoJournal::default();
                    let mut awg = AwgBackend::new(Some(iface_name), Some(0x4750));
                    if let Err(e) = awg.start(&awg_profile, self.dry_run, &mut journal).await {
                        warn!("Could not start AwgBackend: {}", e);
                    }

                    let mut rm = RouteManager::new(self.dry_run);
                    let rg = match rm.apply_routing(iface_name, config.settings.ipv6_routing) {
                        Ok(g) => Some(g),
                        Err(e) => {
                            warn!("Could not apply policy routing on {iface_name}: {e}");
                            None
                        }
                    };

                    let mut dm = DnsManager::new(
                        &config.settings.dns_primary,
                        &config.settings.dns_fallback,
                        self.dry_run,
                    );
                    let dg = dm.apply_dns(iface_name, &mut journal).ok();

                    let ks = if config.settings.kill_switch {
                        KillSwitch::enable(self.dry_run, config.settings.allow_lan, &mut journal).ok()
                    } else {
                        None
                    };

                    (None, rg, dg, None, ks, None, Some(awg))
                } else {
                    // 1. Open TUN device
                    let mut tun = TunDevice::new("ghost0", config.settings.mtu);
                    let tun_opt = match tun.open().await {
                        Ok(()) => {
                            info!("TUN interface ghost0 created and opened successfully");
                            Some(tun)
                        }
                        Err(e) => {
                            warn!("Could not open kernel TUN interface: {}", e);
                            None
                        }
                    };

                    // 2. Setup Policy Routing & Netlink rules
                    let mut rm = RouteManager::new(self.dry_run);
                    let rg = match rm.apply_routing("ghost0", config.settings.ipv6_routing) {
                        Ok(guard) => {
                            info!(
                                "Policy routing installed: fwmark 0x4750, table 5180, priority 5180/5181"
                            );
                            Some(guard)
                        }
                        Err(e) => {
                            warn!("Could not apply policy routing: {}", e);
                            None
                        }
                    };

                    // 3. Configure DNS and prevent leaks
                    let mut journal = UndoJournal::default();
                    let mut dm = DnsManager::new(
                        &config.settings.dns_primary,
                        &config.settings.dns_fallback,
                        self.dry_run,
                    );
                    let dg = match dm.apply_dns("ghost0", &mut journal) {
                        Ok(guard) => {
                            info!("Encrypted DNS routing active on ghost0 (~. domain)");
                            Some(guard)
                        }
                        Err(e) => {
                            warn!("Could not configure DNS: {}", e);
                            None
                        }
                    };

                    // 4. Start DoH Proxy listener
                    let dp = DnsProxy::new(10853, config.settings.dns_primary.clone());
                    let dp_opt = match dp.start().await {
                        Ok(()) => {
                            info!("Internal encrypted DNS proxy listening on 127.0.0.1:10853");
                            Some(dp)
                        }
                        Err(e) => {
                            warn!("Could not start DNS proxy: {}", e);
                            None
                        }
                    };

                    // 5. nftables kill switch
                    let ks = if config.settings.kill_switch {
                        match KillSwitch::enable(self.dry_run, config.settings.allow_lan, &mut journal) {
                            Ok(guard) => {
                                info!("nftables killswitch armed on inet ghostpath");
                                Some(guard)
                            }
                            Err(e) => {
                                warn!("Could not enable kill switch: {}", e);
                                None
                            }
                        }
                    } else {
                        None
                    };

                    // 6. Launch proxy core supervisor
                    let mut effective_rules = if config.rules.is_empty() {
                        ghostpath_core::rules::default_routing_rules()
                    } else {
                        config.rules.clone()
                    };
                    if config.settings.geo_spoof.enabled && config.settings.geo_spoof.strict_geo_routing {
                        for rule in &mut effective_rules {
                            if rule.action == RuleAction::Direct
                                && matches!(rule.r#type, RuleType::Geoip | RuleType::Geosite)
                                && rule.pattern != "private"
                            {
                                rule.action = RuleAction::Proxy;
                            }
                        }
                    }

                    let mut core = ManagedCoreOutbound::new(
                        config.settings.core_engine,
                        config.settings.socks_port,
                        config.settings.http_port,
                        0x4750,
                    )
                    .with_rules(effective_rules);
                    if let Err(e) = core.start(&node).await {
                        warn!("Could not start managed proxy core: {}", e);
                    }

                    (tun_opt, rg, dg, dp_opt, ks, Some(core), None)
                }
            } else {
                info!("Dry-run mode: simulating tunnel setup");
                if is_awg_protocol {
                    let mut journal = UndoJournal::default();
                    let mut awg = AwgBackend::new(Some(iface_name), Some(0x4750));
                    let profile = ghostpath_proto::amnezia::AwgProfileConfig {
                        id: node.id.clone(),
                        name: node.name.clone(),
                        interface: ghostpath_proto::amnezia::AwgInterfaceConfig {
                            private_key: "YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=".to_string(),
                            address: vec!["10.8.0.2/24".to_string()],
                            dns: vec![config.settings.dns_primary.clone()],
                            mtu: Some(config.settings.mtu),
                            obfuscation: ghostpath_proto::amnezia::AwgObfuscationConfig::default(),
                        },
                        peer: ghostpath_proto::amnezia::AwgPeerConfig {
                            public_key: "YmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmI=".to_string(),
                            preshared_key: None,
                            endpoint: format!("{}:{}", node.host, node.port),
                            allowed_ips: vec!["0.0.0.0/0".to_string()],
                            persistent_keepalive: Some(25),
                        },
                    };
                    let _ = awg.start(&profile, true, &mut journal).await;
                    (None, None, None, None, None, None, Some(awg))
                } else {
                    (None, None, None, None, None, None, None)
                }
            };

        let geo_spoof_guard = if config.settings.geo_spoof.enabled
            && config.settings.geo_spoof.apply_on_connect
        {
            if let Some(preset) = resolve_target_preset(&config.settings.geo_spoof, Some(&node)) {
                if let Some(mut old_manual) = self.manual_geo_guard.write().await.take() {
                    old_manual.disarm();
                }
                let mut journal = if self.geo_paths.use_systemd_tools {
                    UndoJournal::load().ok().flatten().unwrap_or_default()
                } else {
                    UndoJournal::new()
                };
                let mgr = GeoSpoofManager::with_paths(self.geo_paths.clone(), self.dry_run);
                match mgr.apply(
                    &preset,
                    config.settings.geo_spoof.spoof_timezone,
                    config.settings.geo_spoof.spoof_geolocation,
                    &mut journal,
                ) {
                    Ok(guard) => {
                        if !self.dry_run && self.geo_paths.use_systemd_tools {
                            let _ = journal.save();
                        }
                        *self.applied_geo.write().await = Some((
                            preset.country_code.clone(),
                            preset.country_name.clone(),
                            preset.timezone.clone(),
                        ));
                        self.append_log(format!(
                            "Geo/Timezone spoofing active: {} ({})",
                            preset.country_name, preset.timezone
                        ))
                        .await;
                        Some(guard)
                    }
                    Err(e) => {
                        warn!("Could not apply geo/timezone spoofing: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        let session = ActiveSession {
            node: node.clone(),
            start_time: Instant::now(),
            route_guard,
            dns_guard,
            dns_proxy,
            killswitch_guard,
            tun_device,
            core_process,
            awg_backend,
            geo_spoof_guard,
        };

        *self.session.write().await = Some(session);
        *status = ConnectionStatus::Connected;
        self.append_log(format!("Connected to node {} ({})", node.name, node.host))
            .await;

        Ok(serde_json::json!({
            "status": "connected",
            "nodeId": node.id,
            "nodeName": node.name,
            "interface": iface_name,
        }))
    }

    pub async fn disconnect(&self) -> Result<serde_json::Value> {
        let mut status = self.status.write().await;
        if *status == ConnectionStatus::Disconnected {
            return Ok(serde_json::json!({ "status": "disconnected" }));
        }

        *status = ConnectionStatus::Disconnecting;
        self.append_log("Tearing down VPN tunnel...".to_string())
            .await;

        let mut session_lock = self.session.write().await;
        if let Some(mut session) = session_lock.take() {
            if let Some(mut awg) = session.awg_backend.take() {
                let mut journal = UndoJournal::default();
                if let Err(e) = awg.stop(self.dry_run, &mut journal).await {
                    warn!("Error stopping AwgBackend: {}", e);
                }
            }
            if let Some(mut core) = session.core_process.take() {
                if let Err(e) = core.stop().await {
                    warn!("Error stopping proxy core: {}", e);
                }
            }
            if let Some(dp) = session.dns_proxy.take() {
                dp.stop();
            }
            // Drop guards to trigger automatic restoration
            if let Some(guard) = session.geo_spoof_guard.take() {
                drop(guard);
                *self.applied_geo.write().await = None;
                self.append_log("Restored original OS timezone and geolocation".to_string())
                    .await;
            }
            drop(session.killswitch_guard.take());
            drop(session.dns_guard.take());
            drop(session.route_guard.take());
            drop(session.tun_device.take());
        }

        *status = ConnectionStatus::Disconnected;
        self.append_log("Disconnected cleanly, routing restored.".to_string())
            .await;
        info!("GhostPath VPN disconnected and state restored");

        Ok(serde_json::json!({ "status": "disconnected" }))
    }

    pub async fn get_status(&self) -> Result<serde_json::Value> {
        let status = *self.status.read().await;
        let session = self.session.read().await;
        let config = self.config.read().await;

        let (node_val, session_secs) = if let Some(ref s) = *session {
            let secs = s.start_time.elapsed().as_secs();
            (
                serde_json::json!({
                    "id": s.node.id,
                    "name": s.node.name,
                    "protocol": format!("{:?}", s.node.protocol),
                    "host": s.node.host,
                    "port": s.node.port,
                }),
                secs,
            )
        } else {
            (serde_json::Value::Null, 0)
        };

        let status_str = match status {
            ConnectionStatus::Disconnected => "disconnected",
            ConnectionStatus::Connecting => "connecting",
            ConnectionStatus::Connected => "connected",
            ConnectionStatus::Degraded => "degraded",
            ConnectionStatus::Failed => "failed",
            ConnectionStatus::Disconnecting => "disconnecting",
        };

        let current_bytes_in = self.bytes_in.load(Ordering::Relaxed);
        let current_bytes_out = self.bytes_out.load(Ordering::Relaxed);
        let mut last_traffic = self.last_traffic.write().await;
        let (last_time, last_in, last_out) = *last_traffic;
        let now = Instant::now();
        let elapsed_secs = now.duration_since(last_time).as_secs_f64();

        let (dl_mbps, ul_mbps) = if status == ConnectionStatus::Connected && elapsed_secs >= 0.5 {
            let delta_in = current_bytes_in.saturating_sub(last_in);
            let delta_out = current_bytes_out.saturating_sub(last_out);
            *last_traffic = (now, current_bytes_in, current_bytes_out);
            let dl = (delta_in as f64 * 8.0) / (elapsed_secs * 1_000_000.0);
            let ul = (delta_out as f64 * 8.0) / (elapsed_secs * 1_000_000.0);
            (dl, ul)
        } else if status == ConnectionStatus::Connected {
            (0.0, 0.0)
        } else {
            *last_traffic = (now, current_bytes_in, current_bytes_out);
            (0.0, 0.0)
        };

        Ok(serde_json::json!({
            "status": status_str,
            "activeNode": node_val,
            "routingMode": "rule",
            "interfaceMode": "tun",
            "traffic": {
                "downloadMbps": dl_mbps,
                "uploadMbps": ul_mbps,
                "totalDownMb": current_bytes_in as f64 / 1_000_000.0,
                "totalUpMb": current_bytes_out as f64 / 1_000_000.0,
                "sessionSeconds": session_secs,
            },
            "interface": "ghost0",
            "mtu": config.settings.mtu,
            "core": config.settings.core_engine,
        }))
    }

    pub async fn import_profile(
        &self,
        raw: &str,
        name: Option<String>,
    ) -> Result<serde_json::Value> {
        let trimmed = raw.trim();

        // 1. Amnezia vpn:// URI
        if trimmed.to_lowercase().starts_with("vpn://") {
            let server = ghostpath_proto::amnezia::parse_vpn_uri(trimmed)?;
            let mut imported_nodes = Vec::new();
            let mut unsupported = Vec::new();

            for container in server.containers {
                if container.is_supported {
                    if let Some(awg_prof) = container.awg_profile {
                        let is_awg = awg_prof.is_amnezia();
                        let (host, port) = awg_prof
                            .peer
                            .endpoint
                            .split_once(':')
                            .map(|(h, p)| (h.to_string(), p.parse::<u16>().unwrap_or(39001)))
                            .unwrap_or_else(|| (awg_prof.peer.endpoint.clone(), 39001));

                        let node = ServerNode {
                            id: awg_prof.id.clone(),
                            name: if let Some(ref n) = name {
                                format!("{n} ({})", container.container_name)
                            } else {
                                awg_prof.name.clone()
                            },
                            country_code: "UN".to_string(),
                            city: "Tunnel".to_string(),
                            country: "Amnezia".to_string(),
                            protocol: if is_awg {
                                NodeProtocol::AmneziaWG
                            } else {
                                NodeProtocol::WireGuard
                            },
                            transport: if is_awg {
                                "obfuscated-udp".to_string()
                            } else {
                                "udp".to_string()
                            },
                            security: if is_awg {
                                "AmneziaWG Obfuscated".to_string()
                            } else {
                                "WireGuard".to_string()
                            },
                            host,
                            port,
                            latency_ms: None,
                            pinging: None,
                            load_percent: 0,
                            subscription_id: None,
                            raw_link: Some(ghostpath_proto::amnezia::export_awg_conf(&awg_prof)),
                        };
                        imported_nodes.push(node);
                    }
                } else {
                    unsupported.push(serde_json::json!({
                        "container": container.container_name,
                        "reason": container
                            .unsupported_reason
                            .unwrap_or_else(|| "Unsupported protocol".to_string()),
                    }));
                }
            }

            if imported_nodes.is_empty() && !unsupported.is_empty() {
                bail!(
                    "The provided Amnezia key contains no supported containers. Unsupported: {:?}",
                    unsupported
                );
            }

            let mut config = self.config.write().await;
            for n in &imported_nodes {
                config.nodes.push(n.clone());
            }
            if config.active_node_id.is_none() && !imported_nodes.is_empty() {
                config.active_node_id = Some(imported_nodes[0].id.clone());
            }
            let _ = config.save();

            self.append_log(format!(
                "Imported {} profile(s) from Amnezia server: {}",
                imported_nodes.len(),
                server.server_name
            ))
            .await;

            return Ok(serde_json::json!({
                "type": "amnezia_multi",
                "importedCount": imported_nodes.len(),
                "nodes": imported_nodes,
                "unsupportedContainers": unsupported,
            }));
        }

        // 2. AmneziaWG or WireGuard INI config (.conf)
        if trimmed.contains("[Interface]") && trimmed.contains("[Peer]") {
            let prof_name = name.unwrap_or_else(|| "AmneziaWG Profile".to_string());
            let awg_prof = ghostpath_proto::amnezia::parse_awg_conf(trimmed, &prof_name)?;
            let is_awg = awg_prof.is_amnezia();
            let (host, port) = awg_prof
                .peer
                .endpoint
                .split_once(':')
                .map(|(h, p)| (h.to_string(), p.parse::<u16>().unwrap_or(39001)))
                .unwrap_or_else(|| (awg_prof.peer.endpoint.clone(), 39001));

            let node = ServerNode {
                id: awg_prof.id.clone(),
                name: awg_prof.name.clone(),
                country_code: "UN".to_string(),
                city: "Tunnel".to_string(),
                country: "Amnezia".to_string(),
                protocol: if is_awg {
                    NodeProtocol::AmneziaWG
                } else {
                    NodeProtocol::WireGuard
                },
                transport: if is_awg {
                    "obfuscated-udp".to_string()
                } else {
                    "udp".to_string()
                },
                security: if is_awg {
                    "AmneziaWG Obfuscated".to_string()
                } else {
                    "WireGuard".to_string()
                },
                host,
                port,
                latency_ms: None,
                pinging: None,
                load_percent: 0,
                subscription_id: None,
                raw_link: Some(trimmed.to_string()),
            };

            let mut config = self.config.write().await;
            config.nodes.push(node.clone());
            if config.active_node_id.is_none() {
                config.active_node_id = Some(node.id.clone());
            }
            let _ = config.save();

            self.append_log(format!("Imported profile: {}", node.name))
                .await;

            return Ok(serde_json::json!({
                "type": "single_node",
                "node": {
                    "id": node.id,
                    "name": node.name,
                    "protocol": format!("{:?}", node.protocol),
                    "host": node.host,
                    "port": node.port,
                }
            }));
        }

        // 3. Standard V2Ray URI link
        let mut node = parse_any_link(raw)?;
        if let Some(n) = name {
            node.name = n;
        }

        let mut config = self.config.write().await;
        config.nodes.push(node.clone());
        if config.active_node_id.is_none() {
            config.active_node_id = Some(node.id.clone());
        }
        let _ = config.save();

        self.append_log(format!("Imported profile: {}", node.name))
            .await;

        Ok(serde_json::json!({
            "type": "single_node",
            "node": {
                "id": node.id,
                "name": node.name,
                "protocol": format!("{:?}", node.protocol),
                "host": node.host,
                "port": node.port,
            }
        }))
    }

    pub async fn list_nodes(&self) -> Result<serde_json::Value> {
        let config = self.config.read().await;
        Ok(serde_json::to_value(&config.nodes)?)
    }

    pub async fn get_settings(&self) -> Result<serde_json::Value> {
        let config = self.config.read().await;
        Ok(serde_json::to_value(&config.settings)?)
    }

    pub async fn update_settings(&self, patch: serde_json::Value) -> Result<serde_json::Value> {
        let mut config = self.config.write().await;
        if let Some(core) = patch.get("coreEngine").and_then(|v| v.as_str()) {
            config.settings.core_engine = match core {
                "xray" | "xray-core" => CoreEngine::XrayCore,
                "native" => CoreEngine::Native,
                _ => CoreEngine::SingBox,
            };
        }
        if let Some(launch) = patch.get("launchOnBoot").and_then(|v| v.as_bool()) {
            config.settings.launch_on_boot = launch;
        }
        if let Some(kill) = patch.get("killSwitch").and_then(|v| v.as_bool()) {
            config.settings.kill_switch = kill;
        }
        if let Some(lan) = patch.get("allowLan").and_then(|v| v.as_bool()) {
            config.settings.allow_lan = lan;
        }
        if let Some(ipv6) = patch.get("ipv6Routing").and_then(|v| v.as_bool()) {
            config.settings.ipv6_routing = ipv6;
        }
        if let Some(stack) = patch.get("tunStack").and_then(|v| v.as_str()) {
            config.settings.tun_stack = stack.to_string();
        }
        if let Some(mtu) = patch.get("mtu").and_then(|v| v.as_u64()) {
            config.settings.mtu = mtu as u16;
        }
        if let Some(socks) = patch.get("socksPort").and_then(|v| v.as_u64()) {
            config.settings.socks_port = socks as u16;
        }
        if let Some(http) = patch.get("httpPort").and_then(|v| v.as_u64()) {
            config.settings.http_port = http as u16;
        }
        if let Some(dns) = patch.get("dnsPrimary").and_then(|v| v.as_str()) {
            config.settings.dns_primary = dns.to_string();
        }
        if let Some(fallback) = patch.get("dnsFallback").and_then(|v| v.as_str()) {
            config.settings.dns_fallback = fallback.to_string();
        }
        let _ = config.save();
        Ok(serde_json::to_value(&config.settings)?)
    }

    pub async fn ping_node(&self, node_id: &str) -> Result<serde_json::Value> {
        let node_info = {
            let config = self.config.read().await;
            config.nodes.iter().find(|n| n.id == node_id).cloned()
        };

        let node = match node_info {
            Some(n) => n,
            None => bail!("Node '{}' not found", node_id),
        };

        let target = format!("{}:{}", node.host, node.port);
        let start = Instant::now();
        let ping_result = tokio::time::timeout(
            std::time::Duration::from_millis(1500),
            tokio::net::TcpStream::connect(&target),
        )
        .await;

        let latency_ms = match ping_result {
            Ok(Ok(_stream)) => Some(start.elapsed().as_millis() as u32),
            _ => None,
        };

        {
            let mut config = self.config.write().await;
            if let Some(target_node) = config.nodes.iter_mut().find(|n| n.id == node_id) {
                target_node.latency_ms = latency_ms;
            }
            let _ = config.save();
        }

        Ok(serde_json::json!({
            "nodeId": node_id,
            "latencyMs": latency_ms
        }))
    }

    pub async fn ping_all(&self) -> Result<serde_json::Value> {
        let nodes = {
            let config = self.config.read().await;
            config.nodes.clone()
        };

        let mut results = Vec::new();
        for node in &nodes {
            let target = format!("{}:{}", node.host, node.port);
            let start = Instant::now();
            let ping_result = tokio::time::timeout(
                std::time::Duration::from_millis(1500),
                tokio::net::TcpStream::connect(&target),
            )
            .await;

            let latency = match ping_result {
                Ok(Ok(_)) => Some(start.elapsed().as_millis() as u32),
                _ => None,
            };

            results.push(serde_json::json!({
                "nodeId": node.id,
                "latencyMs": latency
            }));
        }

        {
            let mut config = self.config.write().await;
            for res in &results {
                let id = res["nodeId"].as_str().unwrap_or("");
                let lat = res["latencyMs"].as_u64().map(|v| v as u32);
                if let Some(n) = config.nodes.iter_mut().find(|n| n.id == id) {
                    n.latency_ms = lat;
                }
            }
            let _ = config.save();
        }

        Ok(serde_json::json!({ "results": results }))
    }

    pub async fn list_subscriptions(&self) -> Result<serde_json::Value> {
        let config = self.config.read().await;
        Ok(serde_json::to_value(&config.subscriptions)?)
    }

    pub async fn import_subscription(
        &self,
        url: &str,
        name: Option<String>,
    ) -> Result<serde_json::Value> {
        let trimmed_url = url.trim();
        info!(url = %trimmed_url, "Fetching subscription feed");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()?;
        let resp = client.get(trimmed_url).send().await?;
        if !resp.status().is_success() {
            bail!("Failed to fetch subscription (HTTP {})", resp.status());
        }
        let body = resp.text().await?;
        let nodes = ghostpath_proto::parser::subscription::parse_subscription_content(&body)?;
        if nodes.is_empty() {
            bail!("No valid server profiles found in subscription feed");
        }

        let sub_id = format!("sub-{}", uuid::Uuid::new_v4().simple());
        let sub_name = name.unwrap_or_else(|| {
            url::Url::parse(trimmed_url)
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_string()))
                .unwrap_or_else(|| "Remote Subscription".to_string())
        });

        let mut imported_nodes = nodes;
        for node in &mut imported_nodes {
            node.subscription_id = Some(sub_id.clone());
        }

        let subscription = ghostpath_core::models::Subscription {
            id: sub_id.clone(),
            name: sub_name.clone(),
            url: trimmed_url.to_string(),
            used_gb: 0.0,
            total_gb: 0.0,
            expires_at: "Unknown".to_string(),
            last_updated: "Just now".to_string(),
            auto_update_hours: 12,
            updating: Some(false),
        };

        {
            let mut config = self.config.write().await;
            config.subscriptions.push(subscription.clone());
            config.nodes.extend(imported_nodes.clone());
            if config.active_node_id.is_none() && !config.nodes.is_empty() {
                config.active_node_id = Some(config.nodes[0].id.clone());
            }
            let _ = config.save();
        }

        self.append_log(format!(
            "Imported subscription '{}' ({} nodes)",
            sub_name,
            imported_nodes.len()
        ))
        .await;

        Ok(serde_json::json!({
            "subscription": subscription,
            "nodesCount": imported_nodes.len()
        }))
    }

    pub async fn delete_subscription(&self, id: &str) -> Result<serde_json::Value> {
        let mut config = self.config.write().await;
        config.subscriptions.retain(|s| s.id != id);
        config.nodes.retain(|n| n.subscription_id.as_deref() != Some(id));
        if config.active_node_id.as_deref() == Some(id) {
            config.active_node_id = config.nodes.first().map(|n| n.id.clone());
        }
        let _ = config.save();
        Ok(serde_json::json!({ "deleted": id }))
    }

    pub async fn refresh_subscription(&self, id: &str) -> Result<serde_json::Value> {
        let sub = {
            let config = self.config.read().await;
            config.subscriptions.iter().find(|s| s.id == id).cloned()
        };

        let sub = match sub {
            Some(s) => s,
            None => bail!("Subscription '{}' not found", id),
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()?;
        let resp = client.get(&sub.url).send().await?;
        if !resp.status().is_success() {
            bail!("Failed to refresh subscription (HTTP {})", resp.status());
        }
        let body = resp.text().await?;
        let nodes = ghostpath_proto::parser::subscription::parse_subscription_content(&body)?;

        let mut imported_nodes = nodes;
        for node in &mut imported_nodes {
            node.subscription_id = Some(id.to_string());
        }

        {
            let mut config = self.config.write().await;
            config.nodes.retain(|n| n.subscription_id.as_deref() != Some(id));
            config.nodes.extend(imported_nodes.clone());
            if let Some(s) = config.subscriptions.iter_mut().find(|s| s.id == id) {
                s.last_updated = "Just now".to_string();
            }
            let _ = config.save();
        }

        Ok(serde_json::json!({
            "id": id,
            "nodesCount": imported_nodes.len()
        }))
    }

    pub async fn list_rules(&self) -> Result<serde_json::Value> {
        let config = self.config.read().await;
        let rules = if config.rules.is_empty() {
            ghostpath_core::rules::default_routing_rules()
        } else {
            config.rules.clone()
        };
        Ok(serde_json::to_value(&rules)?)
    }

    pub async fn add_rule(&self, mut rule_val: serde_json::Value) -> Result<serde_json::Value> {
        if rule_val.get("id").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
            rule_val["id"] = serde_json::json!(format!("rule-{}", uuid::Uuid::new_v4().simple()));
        }
        if rule_val.get("type").is_none() {
            if let Some(rt) = rule_val.get("ruleType") {
                rule_val["type"] = rt.clone();
            }
        }
        let rule: ghostpath_core::models::RoutingRule = serde_json::from_value(rule_val)?;
        let mut config = self.config.write().await;
        if config.rules.is_empty() {
            config.rules = ghostpath_core::rules::default_routing_rules();
        }
        config.rules.push(rule.clone());
        let _ = config.save();
        Ok(serde_json::to_value(&rule)?)
    }

    pub async fn delete_rule(&self, id: &str) -> Result<serde_json::Value> {
        let mut config = self.config.write().await;
        if config.rules.is_empty() {
            config.rules = ghostpath_core::rules::default_routing_rules();
        }
        config.rules.retain(|r| r.id != id);
        let _ = config.save();
        Ok(serde_json::json!({ "deleted": id }))
    }

    pub async fn toggle_rule(&self, id: &str) -> Result<serde_json::Value> {
        let mut config = self.config.write().await;
        if config.rules.is_empty() {
            config.rules = ghostpath_core::rules::default_routing_rules();
        }
        let mut new_state = true;
        if let Some(r) = config.rules.iter_mut().find(|r| r.id == id) {
            r.enabled = !r.enabled;
            new_state = r.enabled;
        }
        let _ = config.save();
        Ok(serde_json::json!({ "id": id, "enabled": new_state }))
    }

    pub async fn update_rule_action(
        &self,
        id: &str,
        action: RuleAction,
    ) -> Result<serde_json::Value> {
        let mut config = self.config.write().await;
        if config.rules.is_empty() {
            config.rules = ghostpath_core::rules::default_routing_rules();
        }
        if let Some(r) = config.rules.iter_mut().find(|r| r.id == id) {
            r.action = action;
        } else {
            bail!("Routing rule '{}' not found", id);
        }
        let _ = config.save();
        Ok(serde_json::json!({ "id": id, "action": action }))
    }

    pub async fn list_geo_countries(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(geo_country_presets())?)
    }

    pub async fn get_geo_spoof_status(&self) -> Result<serde_json::Value> {
        let settings = self.config.read().await.settings.geo_spoof.clone();
        let mgr = GeoSpoofManager::with_paths(self.geo_paths.clone(), self.dry_run);
        let (timedatectl_available, geoclue_available) = mgr.detect_capabilities();
        let active_timezone = mgr.read_active_timezone();
        let active_geolocation = mgr.read_active_geolocation();
        let backed_up_timezone = mgr.read_backed_up_timezone();
        let applied = self.applied_geo.read().await.clone();
        let spoof_active = mgr.is_spoof_active() || applied.is_some();

        let status = GeoSpoofStatus {
            settings,
            spoof_active,
            applied_country_code: applied.as_ref().map(|a| a.0.clone()),
            applied_country_name: applied.as_ref().map(|a| a.1.clone()),
            applied_timezone: applied.as_ref().map(|a| a.2.clone()),
            active_timezone,
            active_geolocation,
            backed_up_timezone,
            geoclue_available,
            timedatectl_available,
        };
        Ok(serde_json::to_value(&status)?)
    }

    pub async fn update_geo_spoof(&self, patch: serde_json::Value) -> Result<serde_json::Value> {
        let updated_settings = {
            let mut config = self.config.write().await;
            let gs = &mut config.settings.geo_spoof;

            if let Some(v) = patch.get("enabled").and_then(|v| v.as_bool()) {
                gs.enabled = v;
            }
            if let Some(mode_str) = patch.get("mode").and_then(|v| v.as_str()) {
                gs.mode = match mode_str {
                    "manual" => GeoSpoofMode::Manual,
                    _ => GeoSpoofMode::MatchNode,
                };
            }
            if let Some(code) = patch.get("countryCode").and_then(|v| v.as_str()) {
                if let Some(preset) = find_country_preset(code) {
                    gs.country_code = preset.country_code;
                } else {
                    bail!("Unsupported country preset '{}'", code);
                }
            }
            if let Some(v) = patch.get("spoofTimezone").and_then(|v| v.as_bool()) {
                gs.spoof_timezone = v;
            }
            if let Some(v) = patch.get("spoofGeolocation").and_then(|v| v.as_bool()) {
                gs.spoof_geolocation = v;
            }
            if let Some(v) = patch.get("applyOnConnect").and_then(|v| v.as_bool()) {
                gs.apply_on_connect = v;
            }
            if let Some(v) = patch.get("strictGeoRouting").and_then(|v| v.as_bool()) {
                gs.strict_geo_routing = v;
            }

            let copy = gs.clone();
            let _ = config.save();
            copy
        };

        let is_connected = *self.status.read().await == ConnectionStatus::Connected;
        if is_connected {
            if updated_settings.enabled {
                self.apply_geo_spoof(None).await?;
            } else {
                self.restore_geo_spoof().await?;
            }
        }

        self.get_geo_spoof_status().await
    }

    pub async fn apply_geo_spoof(
        &self,
        country_code: Option<String>,
    ) -> Result<serde_json::Value> {
        let (settings, active_node) = {
            let cfg = self.config.read().await;
            let session = self.session.read().await;
            (
                cfg.settings.geo_spoof.clone(),
                session.as_ref().map(|s| s.node.clone()),
            )
        };

        let preset = if let Some(ref code) = country_code {
            find_country_preset(code)
                .ok_or_else(|| anyhow::anyhow!("Unsupported country '{}'", code))?
        } else {
            resolve_target_preset(&settings, active_node.as_ref())
                .ok_or_else(|| anyhow::anyhow!("No valid country preset configured"))?
        };

        // Persist manual override if an explicit country was provided
        if let Some(ref code) = country_code {
            let mut cfg = self.config.write().await;
            cfg.settings.geo_spoof.enabled = true;
            cfg.settings.geo_spoof.mode = GeoSpoofMode::Manual;
            cfg.settings.geo_spoof.country_code = preset.country_code.clone();
            let _ = cfg.save();
            let _ = code;
        }

        let spoof_tz = settings.spoof_timezone || country_code.is_some();
        let spoof_geo = settings.spoof_geolocation || country_code.is_some();

        // Disarm existing guards without restoring so original backup stays intact across hot swaps.
        if let Some(mut old_manual) = self.manual_geo_guard.write().await.take() {
            old_manual.disarm();
        }
        {
            let mut session = self.session.write().await;
            if let Some(ref mut s) = *session {
                if let Some(mut old_sess) = s.geo_spoof_guard.take() {
                    old_sess.disarm();
                }
            }
        }

        let mut journal = if self.geo_paths.use_systemd_tools {
            UndoJournal::load().ok().flatten().unwrap_or_default()
        } else {
            UndoJournal::new()
        };
        let mgr = GeoSpoofManager::with_paths(self.geo_paths.clone(), self.dry_run);
        let guard = mgr.apply(&preset, spoof_tz, spoof_geo, &mut journal)?;
        if !self.dry_run && self.geo_paths.use_systemd_tools {
            let _ = journal.save();
        }

        {
            let mut session = self.session.write().await;
            if let Some(ref mut s) = *session {
                s.geo_spoof_guard = Some(guard);
            } else {
                *self.manual_geo_guard.write().await = Some(guard);
            }
        }

        *self.applied_geo.write().await = Some((
            preset.country_code.clone(),
            preset.country_name.clone(),
            preset.timezone.clone(),
        ));

        self.append_log(format!(
            "Applied system Geo/Timezone spoof: {} ({}, {:.4}, {:.4})",
            preset.country_name, preset.timezone, preset.latitude, preset.longitude
        ))
        .await;

        self.get_geo_spoof_status().await
    }

    pub async fn restore_geo_spoof(&self) -> Result<serde_json::Value> {
        {
            let mut session = self.session.write().await;
            if let Some(ref mut s) = *session {
                drop(s.geo_spoof_guard.take());
            }
        }
        if let Some(guard) = self.manual_geo_guard.write().await.take() {
            drop(guard);
        }

        let mgr = GeoSpoofManager::with_paths(self.geo_paths.clone(), self.dry_run);
        mgr.restore()?;
        *self.applied_geo.write().await = None;
        self.append_log("Restored original system timezone and geolocation".to_string())
            .await;
        self.get_geo_spoof_status().await
    }

    pub async fn get_logs(&self, limit: usize) -> Result<serde_json::Value> {
        let logs = self.log_buffer.read().await;
        let count = logs.len();
        let start = count.saturating_sub(limit);
        let slice = &logs[start..];
        Ok(serde_json::to_value(slice)?)
    }

    pub async fn run_doctor(&self) -> Result<serde_json::Value> {
        let report = SystemDoctor::run_diagnostics();
        Ok(serde_json::to_value(&report)?)
    }
}

fn resolve_target_preset(
    settings: &GeoSpoofSettings,
    active_node: Option<&ServerNode>,
) -> Option<GeoCountryPreset> {
    match settings.mode {
        GeoSpoofMode::MatchNode => {
            if let Some(node) = active_node {
                if !node.country_code.is_empty() && node.country_code != "UN" {
                    if let Some(p) = find_country_preset(&node.country_code) {
                        return Some(p);
                    }
                }
                if !node.country.is_empty() {
                    if let Some(p) = find_country_preset(&node.country) {
                        return Some(p);
                    }
                }
            }
            find_country_preset(&settings.country_code)
        }
        GeoSpoofMode::Manual => find_country_preset(&settings.country_code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_daemon_engine_lifecycle_dry_run() {
        let engine = DaemonEngine::new(true);
        {
            let mut cfg = engine.config.write().await;
            cfg.nodes.clear();
            cfg.active_node_id = None;
        }
        let status = engine.get_status().await.unwrap();
        assert_eq!(status["status"], "disconnected");

        // Attempting to connect without profiles must fail cleanly
        let no_profile_err = engine.connect(None).await;
        assert!(no_profile_err.is_err());
        assert!(no_profile_err.unwrap_err().to_string().contains("No server profiles available"));

        // Import a profile first
        let uri = "vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@127.0.0.1:443?security=reality#Test-Node";
        let imported = engine.import_profile(uri, Some("Test Node".to_string())).await.unwrap();
        assert_eq!(imported["type"], "single_node");

        // Now connect succeeds
        let connect_res = engine.connect(None).await.unwrap();
        assert_eq!(connect_res["status"], "connected");

        let status2 = engine.get_status().await.unwrap();
        assert_eq!(status2["status"], "connected");
        assert_eq!(status2["interface"], "ghost0");

        let disconnect_res = engine.disconnect().await.unwrap();
        assert_eq!(disconnect_res["status"], "disconnected");

        let status3 = engine.get_status().await.unwrap();
        assert_eq!(status3["status"], "disconnected");
    }

    #[tokio::test]
    async fn test_daemon_engine_doctor() {
        let engine = DaemonEngine::new(true);
        let doctor = engine.run_doctor().await.unwrap();
        let checks = doctor["checks"].as_array().unwrap();
        assert!(checks.iter().any(|c| c["id"] == "tun"));
        assert!(checks.iter().any(|c| c["id"] == "privs"));
    }

    #[tokio::test]
    async fn test_daemon_rules_management() {
        let engine = DaemonEngine::new(true);
        let rules_val = engine.list_rules().await.unwrap();
        let rules = rules_val.as_array().unwrap();
        assert!(!rules.is_empty(), "Default rules should be seeded");

        let first_id = rules[0]["id"].as_str().unwrap();
        let toggled = engine.toggle_rule(first_id).await.unwrap();
        assert_eq!(toggled["id"], first_id);

        let new_rule = serde_json::json!({
            "name": "Custom Test Rule",
            "ruleType": "domain",
            "pattern": "example.internal",
            "action": "direct",
            "enabled": true,
            "target": "example.internal",
            "hits": 0,
            "note": "Unit test rule"
        });
        let added = engine.add_rule(new_rule).await.unwrap();
        let added_id = added["id"].as_str().unwrap();

        let rules_after_add = engine.list_rules().await.unwrap();
        assert!(rules_after_add.as_array().unwrap().iter().any(|r| r["id"] == added_id));

        engine.delete_rule(added_id).await.unwrap();
        let rules_after_del = engine.list_rules().await.unwrap();
        assert!(!rules_after_del.as_array().unwrap().iter().any(|r| r["id"] == added_id));
    }

    #[tokio::test]
    async fn test_daemon_subscriptions_and_ping() {
        let engine = DaemonEngine::new(true);
        let subs = engine.list_subscriptions().await.unwrap();
        assert_eq!(subs.as_array().unwrap().len(), 0);

        // Import a profile first
        let uri = "vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@127.0.0.1:443?security=reality#Test-Node";
        let imported = engine.import_profile(uri, Some("Test Node".to_string())).await.unwrap();
        let node_id = imported["node"]["id"].as_str().unwrap();

        let ping_res = engine.ping_node(node_id).await.unwrap();
        assert_eq!(ping_res["nodeId"], node_id);
    }

    #[tokio::test]
    async fn test_daemon_update_rule_action() {
        let engine = DaemonEngine::new(true);
        let rules_val = engine.list_rules().await.unwrap();
        let first_id = rules_val.as_array().unwrap()[0]["id"].as_str().unwrap().to_string();
        let updated = engine
            .update_rule_action(&first_id, RuleAction::Block)
            .await
            .unwrap();
        assert_eq!(updated["action"], "block");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_daemon_geo_spoof_manual_apply_and_restore() {
        let dir = tempfile::tempdir().unwrap();
        let geo_paths = GeoSpoofPaths::with_root(dir.path());
        for z in &["Asia/Tehran", "Europe/Berlin", "Europe/Amsterdam"] {
            let p = geo_paths.zoneinfo_dir.join(z);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, format!("TZif2-{z}")).unwrap();
        }
        std::fs::create_dir_all(geo_paths.localtime_path.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(
            geo_paths.zoneinfo_dir.join("Asia/Tehran"),
            &geo_paths.localtime_path,
        )
        .unwrap();
        std::fs::write(&geo_paths.timezone_file_path, "Asia/Tehran\n").unwrap();

        let engine = DaemonEngine::with_geo_paths(false, geo_paths.clone());

        let countries = engine.list_geo_countries().await.unwrap();
        assert!(countries.as_array().unwrap().len() >= 25);

        let initial_status = engine.get_geo_spoof_status().await.unwrap();
        assert_eq!(initial_status["activeTimezone"], "Asia/Tehran");
        assert_eq!(initial_status["spoofActive"], false);

        // Apply Germany manually
        let applied = engine
            .apply_geo_spoof(Some("DE".to_string()))
            .await
            .unwrap();
        assert_eq!(applied["spoofActive"], true);
        assert_eq!(applied["activeTimezone"], "Europe/Berlin");
        assert_eq!(applied["backedUpTimezone"], "Asia/Tehran");
        assert_eq!(applied["appliedCountryCode"], "DE");
        assert!(geo_paths.geolocation_path.exists());

        // Restore cleanly
        let restored = engine.restore_geo_spoof().await.unwrap();
        assert_eq!(restored["spoofActive"], false);
        assert_eq!(restored["activeTimezone"], "Asia/Tehran");
        assert!(!geo_paths.geolocation_path.exists());
    }

    #[tokio::test]
    async fn test_daemon_geo_spoof_match_node_on_connect_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let geo_paths = GeoSpoofPaths::with_root(dir.path());
        let engine = DaemonEngine::with_geo_paths(true, geo_paths);
        {
            let mut cfg = engine.config.write().await;
            cfg.nodes.clear();
            cfg.active_node_id = None;
        }

        let uri = "vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@127.0.0.1:443?security=reality#Frankfurt%20DE";
        let imported = engine.import_profile(uri, Some("NL Node".to_string())).await.unwrap();
        let node_id = imported["node"]["id"].as_str().unwrap().to_string();
        {
            let mut cfg = engine.config.write().await;
            if let Some(n) = cfg.nodes.iter_mut().find(|n| n.id == node_id) {
                n.country_code = "NL".to_string();
                n.country = "Netherlands".to_string();
            }
        }

        let updated = engine
            .update_geo_spoof(serde_json::json!({
                "enabled": true,
                "mode": "match-node",
                "applyOnConnect": true
            }))
            .await
            .unwrap();
        assert_eq!(updated["settings"]["enabled"], true);
        assert_eq!(updated["settings"]["mode"], "match-node");

        engine.connect(Some(node_id)).await.unwrap();
        let st_connected = engine.get_geo_spoof_status().await.unwrap();
        assert_eq!(st_connected["spoofActive"], true);
        assert_eq!(st_connected["appliedCountryCode"], "NL");
        assert_eq!(st_connected["appliedTimezone"], "Europe/Amsterdam");

        engine.disconnect().await.unwrap();
        let st_disconnected = engine.get_geo_spoof_status().await.unwrap();
        assert_eq!(st_disconnected["spoofActive"], false);
        assert!(st_disconnected["appliedCountryCode"].is_null());
    }
}
