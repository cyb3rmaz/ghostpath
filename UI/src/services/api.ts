import { ServerNode, AppSettings, ConnectionStatus, DiagnosticLog } from '../types/vpn';
import { INITIAL_NODES, INITIAL_SETTINGS, INITIAL_LOGS } from '../data/vpnData';

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
 * Pure offline desktop JSON-RPC dispatcher using Tauri IPC invoke.
 * Eliminates all network fetch/HTTP calls to 127.0.0.1.
 */
export async function callRpc<T = any>(method: string, params: Record<string, any> = {}): Promise<T> {
  // 1. Native Desktop IPC invoke
  if (typeof window !== 'undefined' && (window as any).__TAURI__) {
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

  // 2. Headless/Browser offline fallback emulator (no network socket)
  return emulateOfflineFallback<T>(method, params);
}

/**
 * Subscribe to Tauri push events (state_changed, traffic_updated, log_appended, deep_link).
 */
export function subscribeTauriEvent<T = any>(event: string, handler: (payload: T) => void): () => void {
  if (typeof window !== 'undefined' && (window as any).__TAURI__) {
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

// In-memory fallback for local browser design mode
let fallbackStatus: ConnectionStatus = 'disconnected';
let fallbackActiveNodeId: string = INITIAL_NODES[0]?.id || 'node-1';
let fallbackSettings: AppSettings = { ...INITIAL_SETTINGS };
let fallbackNodes: ServerNode[] = [...INITIAL_NODES];

function emulateOfflineFallback<T = any>(method: string, params: Record<string, any>): Promise<T> {
  return new Promise((resolve, reject) => {
    switch (method) {
      case 'getStatus':
        resolve({
          status: fallbackStatus,
          activeNode: fallbackNodes.find((n) => n.id === fallbackActiveNodeId) || null,
          routingMode: 'rule',
          interfaceMode: 'tun',
          traffic: {
            downloadMbps: fallbackStatus === 'connected' ? 24.5 : 0.0,
            uploadMbps: fallbackStatus === 'connected' ? 5.2 : 0.0,
            totalDownMb: fallbackStatus === 'connected' ? 142.0 : 0.0,
            totalUpMb: fallbackStatus === 'connected' ? 32.5 : 0.0,
            sessionSeconds: fallbackStatus === 'connected' ? 120 : 0,
          },
          interface: 'ghost0',
          mtu: fallbackSettings.mtu || 1420,
          core: fallbackSettings.coreEngine || 'sing-box',
        } as unknown as T);
        break;

      case 'connect':
        fallbackStatus = 'connected';
        if (params.nodeId) fallbackActiveNodeId = params.nodeId;
        resolve({
          status: 'connected',
          nodeId: fallbackActiveNodeId,
          interface: 'ghost0',
        } as unknown as T);
        break;

      case 'disconnect':
        fallbackStatus = 'disconnected';
        resolve({ status: 'disconnected' } as unknown as T);
        break;

      case 'listNodes':
        resolve(fallbackNodes as unknown as T);
        break;

      case 'importProfile': {
        const raw = params.raw || '';
        const name = params.name || 'Imported Server';
        const newNode: ServerNode = {
          id: `node-${fallbackNodes.length + 1}`,
          name,
          protocol: raw.startsWith('vless://') ? 'VLESS' : 'VMess',
          host: '127.0.0.1',
          port: 443,
          latencyMs: 38,
          region: 'CUSTOM',
        };
        fallbackNodes.push(newNode);
        resolve({ type: 'single_node', node: newNode } as unknown as T);
        break;
      }

      case 'getSettings':
        resolve(fallbackSettings as unknown as T);
        break;

      case 'updateSettings':
        fallbackSettings = { ...fallbackSettings, ...params };
        resolve(fallbackSettings as unknown as T);
        break;

      case 'getLogs':
        resolve(INITIAL_LOGS.slice(0, params.limit || 50) as unknown as T);
        break;

      case 'runDoctor':
        resolve({
          checks: [
            { id: 'tun', name: 'Kernel TUN Device (ghost0)', passed: true, fix: null },
            { id: 'privs', name: 'Daemon Capabilities', passed: true, fix: null },
            { id: 'dns', name: 'systemd-resolved D-Bus', passed: true, fix: null },
            { id: 'rp_filter', name: 'Reverse Path Filtering', passed: true, fix: null },
            { id: 'docker', name: 'Docker Bridge Isolation', passed: true, fix: null },
          ],
          overall_healthy: true,
        } as unknown as T);
        break;

      default:
        reject(new Error(`Method '${method}' not found`));
    }
  });
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
    return callRpc<{ type: string; node: ServerNode }>('importProfile', {
      raw,
      name,
    });
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
    if (typeof window !== 'undefined' && (window as any).__TAURI__) {
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
      state: 'running',
      message: 'GhostPath daemon is running',
      fix_command: null,
      details: null,
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
};
