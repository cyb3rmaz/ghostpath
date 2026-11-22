import { ServerNode, AppSettings, ConnectionStatus, DiagnosticLog } from '../types/vpn';

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

let requestId = 1;

export async function callRpc<T = any>(method: string, params: Record<string, any> = {}): Promise<T> {
  const payload = {
    jsonrpc: '2.0',
    id: requestId++,
    method,
    params,
  };

  // 1. Check if running inside Tauri
  if (typeof window !== 'undefined' && (window as any).__TAURI__) {
    try {
      const invoke = (window as any).__TAURI__.core?.invoke || (window as any).__TAURI__.invoke;
      const res = await invoke('rpc_request', { method, params });
      return res as T;
    } catch (e) {
      console.warn('Tauri RPC failed, falling back to HTTP:', e);
    }
  }

  // 2. HTTP JSON-RPC bridge (default: http://127.0.0.1:3001/rpc)
  try {
    const res = await fetch('http://127.0.0.1:3001/rpc', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload),
    });

    if (!res.ok) {
      throw new Error(`HTTP error ${res.status}: ${res.statusText}`);
    }

    const data: RpcResponse<T> = await res.json();
    if (data.error) {
      throw new Error(`Daemon error [${data.error.code}]: ${data.error.message}`);
    }

    return data.result as T;
  } catch (err) {
    // If daemon is not running on 3001, log and throw
    console.debug(`Backend RPC [${method}] unavailable:`, err);
    throw err;
  }
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
    return callRpc<string[]>('getLogs', { limit });
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
};
