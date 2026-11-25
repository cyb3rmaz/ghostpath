import {
  AppSettings,
  DiagnosticLog,
  GeoSpoofSettings,
  RoutingRule,
  ServerNode,
  Subscription,
  TrafficPoint,
} from '../types/vpn';

export const INITIAL_GEO_SPOOF_SETTINGS: GeoSpoofSettings = {
  enabled: false,
  mode: 'match-node',
  countryCode: 'DE',
  spoofTimezone: true,
  spoofGeolocation: true,
  applyOnConnect: true,
  strictGeoRouting: false,
};

export const INITIAL_SUBSCRIPTIONS: Subscription[] = [];

export const INITIAL_NODES: ServerNode[] = [];

export const INITIAL_RULES: RoutingRule[] = [];

export const INITIAL_LOGS: DiagnosticLog[] = [];

export const INITIAL_TRAFFIC_HISTORY: TrafficPoint[] = Array.from({ length: 24 }, (_, idx) => ({
  time: `${String(idx).padStart(2, '0')}`,
  downloadMbps: 0,
  uploadMbps: 0,
}));

export const INITIAL_SETTINGS: AppSettings = {
  coreEngine: 'sing-box',
  launchOnBoot: true,
  killSwitch: true,
  ipv6Routing: false,
  allowLan: true,
  tunStack: 'system',
  mtu: 1420,
  socksPort: 10808,
  httpPort: 10809,
  dnsPrimary: 'https://1.1.1.1/dns-query',
  dnsFallback: 'tls://9.9.9.9:853',
};
