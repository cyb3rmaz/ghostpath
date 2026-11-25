import {
  AppSettings,
  ConnectionStatus,
  DiagnosticLog,
  GeoCountryPreset,
  GeoSpoofSettings,
  GeoSpoofStatus,
  RuleAction,
  RoutingRule,
  ServerNode,
  Subscription,
} from '../types/vpn';

export interface RpcResponse<T = any> {
  jsonrpc: string;
  id: number;
  result?: T;
  error?: {
    code: number;
    message: string;
    data?: any;
  };
}

/**
 * Check if running inside the Tauri native desktop shell.
 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && Boolean((window as any).__TAURI__);
}

/**
 * Pure offline desktop JSON-RPC dispatcher using Tauri IPC invoke.
 * Eliminates all network fetch/HTTP calls to 127.0.0.1.
 */
export async function callRpc<T = any>(method: string, params: Record<string, any> = {}): Promise<T> {
  if (isTauri()) {
    const tauri = (window as any).__TAURI__;
    const invoke = tauri.core?.invoke || tauri.invoke;
    if (typeof invoke === 'function') {
      try {
        const res = await invoke('rpc_request', { method, params });
        return res as T;
      } catch (e: any) {
        const msg = typeof e === 'string' ? e : e?.message || `RPC invocation failed for ${method}`;
        throw new Error(msg);
      }
    }
  }

  throw new Error(`GhostPath desktop IPC is unavailable. Please run inside the native GhostPath application.`);
}

/**
 * Subscribe to Tauri push events (state_changed, traffic_updated, log_appended, deep_link).
 */
export function subscribeTauriEvent<T = any>(event: string, handler: (payload: T) => void): () => void {
  if (isTauri()) {
    const tauri = (window as any).__TAURI__;
    const listen = tauri.event?.listen;
    if (typeof listen === 'function') {
      let unlistenFn: (() => void) | null = null;
      listen(event, (e: any) => handler(e.payload)).then((unlisten: any) => {
        unlistenFn = unlisten;
      });
      return () => {
        if (unlistenFn) unlistenFn();
      };
    }
  }
  return () => {};
}

export const GhostApi = {
  async connect(nodeId?: string) {
    return callRpc<{ status: ConnectionStatus; nodeId?: string; interface?: string }>('connect', {
      nodeId,
    });
  },

  async disconnect() {
    return callRpc<{ status: 'disconnected' }>('disconnect', {});
  },

  async getStatus() {
    return callRpc<{
      status: ConnectionStatus;
      activeNode: ServerNode | null;
      routingMode: string;
      interfaceMode: string;
      traffic: {
        downloadMbps: number;
        uploadMbps: number;
        totalDownMb: number;
        totalUpMb: number;
        sessionSeconds: number;
      };
      interface: string;
      mtu: number;
      core: string;
    }>('getStatus', {});
  },

  async listNodes() {
    return callRpc<ServerNode[]>('listNodes', {});
  },

  async importProfile(raw: string, name?: string) {
    return callRpc<{
      type: string;
      node?: ServerNode;
      nodes?: ServerNode[];
      importedCount?: number;
      unsupportedContainers?: { container: string; reason: string }[];
    }>('importProfile', {
      raw,
      name,
    });
  },

  async deleteNode(id: string) {
    return callRpc<{ deleted: string }>('deleteNode', { id });
  },

  async pingNode(nodeId: string) {
    return callRpc<{ nodeId: string; latencyMs: number | null }>('pingNode', { nodeId });
  },

  async pingAll() {
    return callRpc<{ results: Array<{ nodeId: string; latencyMs: number | null }> }>('pingAll', {});
  },

  async listSubscriptions() {
    return callRpc<Subscription[]>('listSubscriptions', {});
  },

  async importSubscription(url: string, name?: string) {
    return callRpc<Subscription>('importSubscription', { url, name });
  },

  async deleteSubscription(id: string) {
    return callRpc<{ deleted: string }>('deleteSubscription', { id });
  },

  async refreshSubscription(id: string) {
    return callRpc<Subscription>('refreshSubscription', { id });
  },

  async listRules() {
    return callRpc<RoutingRule[]>('listRules', {});
  },

  async addRule(rule: Partial<RoutingRule>) {
    return callRpc<RoutingRule>('addRule', rule);
  },

  async deleteRule(id: string) {
    return callRpc<{ deleted: string }>('deleteRule', { id });
  },

  async toggleRule(id: string) {
    return callRpc<{ id: string; enabled: boolean }>('toggleRule', { id });
  },

  async updateRuleAction(id: string, action: RuleAction) {
    return callRpc<{ id: string; action: RuleAction }>('updateRuleAction', { id, action });
  },

  async listGeoCountries() {
    return callRpc<GeoCountryPreset[]>('listGeoCountries', {});
  },

  async getGeoSpoofStatus() {
    return callRpc<GeoSpoofStatus>('getGeoSpoofStatus', {});
  },

  async updateGeoSpoof(patch: Partial<GeoSpoofSettings>) {
    return callRpc<GeoSpoofStatus>('updateGeoSpoof', patch);
  },

  async applyGeoSpoof(countryCode?: string) {
    return callRpc<GeoSpoofStatus>('applyGeoSpoof', countryCode ? { countryCode } : {});
  },

  async restoreGeoSpoof() {
    return callRpc<GeoSpoofStatus>('restoreGeoSpoof', {});
  },

  async getSettings() {
    return callRpc<AppSettings>('getSettings', {});
  },

  async updateSettings(settings: Partial<AppSettings>) {
    return callRpc<AppSettings>('updateSettings', settings);
  },

  async getLogs(limit: number = 50) {
    return callRpc<DiagnosticLog[]>('getLogs', { limit });
  },

  async runDoctor() {
    return callRpc<{
      checks: Array<{
        id: string;
        name: string;
        passed: boolean;
        severity?: string;
        details?: string;
        fix?: string | null;
      }>;
      overall_healthy?: boolean;
    }>('runDoctor', {});
  },

  async checkDaemonStatus(): Promise<{
    state: 'running' | 'not_running' | 'not_installed' | 'permission_denied' | 'version_mismatch';
    message: string;
    fix_command?: string | null;
    details?: string | null;
  }> {
    if (isTauri()) {
      const tauri = (window as any).__TAURI__;
      const invoke = tauri.core?.invoke || tauri.invoke;
      if (typeof invoke === 'function') {
        try {
          return await invoke('check_daemon_status');
        } catch (e) {
          console.warn('Tauri check_daemon_status failed:', e);
        }
      }
    }
    return {
      state: 'not_running',
      message: 'GhostPath desktop IPC is unavailable. Please run inside the native GhostPath desktop app.',
      fix_command: 'ghostpath-gui',
      details: 'Native IPC requires the Tauri desktop environment',
    };
  },

  // Events
  onStateChanged(callback: (status: any) => void) {
    return subscribeTauriEvent('state_changed', callback);
  },

  onTrafficUpdated(callback: (traffic: { downloadMbps: number; uploadMbps: number; totalDownMb: number; totalUpMb: number; sessionSeconds: number }) => void) {
    return subscribeTauriEvent('traffic_updated', callback);
  },

  onLogAppended(callback: (log: DiagnosticLog) => void) {
    return subscribeTauriEvent('log_appended', callback);
  },

  onDeepLinkImported(callback: (link: string) => void) {
    return subscribeTauriEvent('deep_link_imported', callback);
  },

  onTrayToggleConnect(callback: () => void) {
    return subscribeTauriEvent('tray_toggle_connect', callback);
  },
};
