export type NavSection = 'tunnel' | 'subscriptions' | 'routing' | 'diagnostics' | 'settings';

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected';

export type RoutingMode = 'rule' | 'global' | 'direct';

export type NetworkInterfaceMode = 'tun' | 'system-proxy';

export type NodeProtocol = 'VLESS' | 'Hysteria2' | 'WireGuard' | 'Trojan' | 'VMess' | 'Shadowsocks' | 'AmneziaWG';

export type CoreEngine = 'sing-box' | 'xray-core';

export interface ServerNode {
  id: string;
  name: string;
  countryCode: string;
  city: string;
  country: string;
  protocol: NodeProtocol;
  transport: string;
  security: string;
  host: string;
  port: number;
  latencyMs: number | null;
  pinging?: boolean;
  loadPercent: number;
  subscriptionId: string;
}

export interface Subscription {
  id: string;
  name: string;
  url: string;
  usedGb: number;
  totalGb: number;
  expiresAt: string;
  lastUpdated: string;
  autoUpdateHours: number;
  updating?: boolean;
}

export type RuleAction = 'proxy' | 'direct' | 'block';

export interface RoutingRule {
  id: string;
  type: 'domain' | 'geosite' | 'geoip' | 'cidr' | 'process';
  pattern: string;
  action: RuleAction;
  enabled: boolean;
  note: string;
}

export type LogLevel = 'info' | 'warn' | 'error';

export interface DiagnosticLog {
  id: string;
  timestamp: string;
  level: LogLevel;
  component: string;
  message: string;
}

export interface TrafficPoint {
  time: string;
  downloadMbps: number;
  uploadMbps: number;
}

export interface AppSettings {
  coreEngine: CoreEngine;
  launchOnBoot: boolean;
  killSwitch: boolean;
  ipv6Routing: boolean;
  allowLan: boolean;
  tunStack: 'system' | 'gvisor' | 'mixed';
  mtu: number;
  socksPort: number;
  httpPort: number;
  dnsPrimary: string;
  dnsFallback: string;
}
