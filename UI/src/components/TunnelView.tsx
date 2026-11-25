import React, { useMemo, useState } from 'react';
import {
  Activity,
  ArrowDown,
  ArrowUp,
  Check,
  ChevronDown,
  Globe,
  Loader2,
  Power,
  RefreshCw,
  Search,
  ShieldCheck,
  ShieldOff,
  SlidersHorizontal,
} from 'lucide-react';
import {
  ConnectionStatus,
  CoreEngine,
  NetworkInterfaceMode,
  NodeProtocol,
  RoutingMode,
  ServerNode,
  Subscription,
  TrafficPoint,
} from '../types/vpn';
import { ThroughputChart } from './ThroughputChart';

interface TunnelViewProps {
  status: ConnectionStatus;
  onToggleConnect: () => void;
  activeNode: ServerNode | null | undefined;
  nodes: ServerNode[];
  subscriptions: Subscription[];
  onSelectNode: (nodeId: string) => void;
  onPingAll: () => void;
  onPingNode: (nodeId: string) => void;
  isPingingAll: boolean;
  routingMode: RoutingMode;
  onChangeRoutingMode: (mode: RoutingMode) => void;
  interfaceMode: NetworkInterfaceMode;
  onChangeInterfaceMode: (mode: NetworkInterfaceMode) => void;
  coreEngine: CoreEngine;
  sessionSeconds: number;
  downloadMbps: number;
  uploadMbps: number;
  totalDownMb: number;
  totalUpMb: number;
  trafficHistory: TrafficPoint[];
  socksPort: number;
}

const formatDuration = (totalSeconds: number) => {
  const hrs = Math.floor(totalSeconds / 3600);
  const mins = Math.floor((totalSeconds % 3600) / 60);
  const secs = totalSeconds % 60;
  return [hrs, mins, secs].map((v) => String(v).padStart(2, '0')).join(':');
};

const formatDataSize = (mb: number) => {
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(2)} GB`;
  }
  return `${mb.toFixed(1)} MB`;
};

const getLatencyTone = (ms: number | null) => {
  if (ms === null) return { text: 'Timeout', color: 'text-[#F87171]' };
  if (ms <= 60) return { text: `${ms} ms`, color: 'text-[#22C55E]' };
  if (ms <= 140) return { text: `${ms} ms`, color: 'text-[#FBBF24]' };
  return { text: `${ms} ms`, color: 'text-[#F87171]' };
};

export const TunnelView: React.FC<TunnelViewProps> = ({
  status,
  onToggleConnect,
  activeNode,
  nodes,
  subscriptions,
  onSelectNode,
  onPingAll,
  onPingNode,
  isPingingAll,
  routingMode,
  onChangeRoutingMode,
  interfaceMode,
  onChangeInterfaceMode,
  coreEngine,
  sessionSeconds,
  downloadMbps,
  uploadMbps,
  totalDownMb,
  totalUpMb,
  trafficHistory,
  socksPort,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedSubFilter, setSelectedSubFilter] = useState<string>('all');
  const [protocolFilter, setProtocolFilter] = useState<'all' | NodeProtocol>('all');

  const isConnected = status === 'connected';
  const isConnecting = status === 'connecting';

  const filteredNodes = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    return nodes.filter((node) => {
      if (selectedSubFilter !== 'all' && node.subscriptionId !== selectedSubFilter) {
        return false;
      }
      if (protocolFilter !== 'all' && node.protocol !== protocolFilter) {
        return false;
      }
      if (!q) return true;
      return (
        node.name.toLowerCase().includes(q) ||
        node.city.toLowerCase().includes(q) ||
        node.country.toLowerCase().includes(q) ||
        node.protocol.toLowerCase().includes(q) ||
        node.host.includes(q)
      );
    });
  }, [nodes, searchQuery, selectedSubFilter, protocolFilter]);

  return (
    <div className="space-y-6">
      {/* Top 2-Column Focal Control & Telemetry Area */}
      <div className="grid grid-cols-1 xl:grid-cols-12 gap-6 items-stretch">
        {/* Connection Control Card */}
        <section
          aria-label="Tunnel connection control"
          className="xl:col-span-5 rounded-xl bg-[#1A1C20] border border-[#292D34] p-6 flex flex-col justify-between"
        >
          <div className="flex items-start justify-between gap-4">
            <div>
              <div className="flex items-center gap-2 text-xs font-medium">
                {isConnected ? (
                  <ShieldCheck className="w-4 h-4 text-[#22C55E]" />
                ) : isConnecting ? (
                  <Loader2 className="w-4 h-4 text-[#FBBF24] animate-spin" />
                ) : (
                  <ShieldOff className="w-4 h-4 text-[#7E8691]" />
                )}
                <span
                  className={
                    isConnected
                      ? 'text-[#22C55E]'
                      : isConnecting
                        ? 'text-[#FBBF24]'
                        : 'text-[#959CA6]'
                  }
                >
                  {isConnected
                    ? 'Tunnel Connected'
                    : isConnecting
                      ? 'Establishing Handshake'
                      : 'Tunnel Disconnected'}
                </span>
              </div>
              <h2 className="text-xl font-semibold text-[#F1F3F5] mt-2 tracking-tight">
                {activeNode ? activeNode.name : 'No Server Profile Selected'}
              </h2>
              <p className="text-xs text-[#959CA6] mt-1 font-mono tabular-nums">
                {activeNode
                  ? `${activeNode.protocol} · ${activeNode.security} · ${activeNode.host}:${activeNode.port}`
                  : 'Import a subscription or server URI to connect'}
              </p>
            </div>

            <div className="text-right shrink-0">
              <div className="text-xs text-[#7E8691]">Session</div>
              <div className="text-sm font-mono tabular-nums text-[#F1F3F5] mt-0.5">
                {formatDuration(sessionSeconds)}
              </div>
            </div>
          </div>

          {/* Central Tactile Power Toggle */}
          <div className="py-7 flex items-center justify-center">
            <button
              type="button"
              onClick={onToggleConnect}
              disabled={isConnecting || !activeNode}
              aria-label={isConnected ? 'Disconnect VPN tunnel' : 'Connect VPN tunnel'}
              className={`group relative w-32 h-32 rounded-full flex flex-col items-center justify-center transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#22C55E] focus-visible:ring-offset-2 focus-visible:ring-offset-[#1A1C20] ${
                isConnected
                  ? 'bg-[#22C55E]/15 border-2 border-[#22C55E] text-[#22C55E] shadow-[0_0_32px_rgba(34,197,94,0.18)] hover:bg-[#22C55E]/20'
                  : isConnecting
                    ? 'bg-[#FBBF24]/10 border-2 border-[#FBBF24]/60 text-[#FBBF24] cursor-wait'
                    : 'bg-[#202328] border-2 border-[#343942] text-[#F1F3F5] hover:border-[#22C55E]/70 hover:text-[#22C55E]'
              }`}
            >
              {isConnecting ? (
                <Loader2 className="w-8 h-8 animate-spin" />
              ) : (
                <Power className="w-8 h-8 transition-transform duration-150 group-hover:scale-105" />
              )}
              <span className="text-xs font-semibold mt-2 whitespace-nowrap">
                {isConnected ? 'Disconnect' : isConnecting ? 'Connecting' : 'Connect'}
              </span>
            </button>
          </div>

          {/* Routing & Interface Mode Segmented Controls */}
          <div className="space-y-3 pt-4 border-t border-[#272A30]">
            <div className="flex items-center justify-between gap-4">
              <span className="text-xs text-[#959CA6] whitespace-nowrap">Routing Mode</span>
              <div className="grid grid-cols-3 gap-1 p-1 rounded-lg bg-[#131518] border border-[#272A30]">
                {(['rule', 'global', 'direct'] as RoutingMode[]).map((mode) => (
                  <button
                    key={mode}
                    type="button"
                    onClick={() => onChangeRoutingMode(mode)}
                    className={`px-3 py-1 rounded-md text-xs font-medium capitalize transition-colors whitespace-nowrap ${
                      routingMode === mode
                        ? 'bg-[#22C55E] text-[#07140B] font-semibold'
                        : 'text-[#959CA6] hover:text-[#F1F3F5]'
                    }`}
                  >
                    {mode}
                  </button>
                ))}
              </div>
            </div>

            <div className="flex items-center justify-between gap-4">
              <span className="text-xs text-[#959CA6] whitespace-nowrap">Interface</span>
              <div className="grid grid-cols-2 gap-1 p-1 rounded-lg bg-[#131518] border border-[#272A30]">
                <button
                  type="button"
                  onClick={() => onChangeInterfaceMode('tun')}
                  className={`px-3 py-1 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
                    interfaceMode === 'tun'
                      ? 'bg-[#252930] text-[#F1F3F5] border border-[#22C55E]/70'
                      : 'text-[#959CA6] border border-transparent hover:text-[#F1F3F5]'
                  }`}
                >
                  TUN ({activeNode?.protocol === 'AmneziaWG' || activeNode?.protocol === 'WireGuard' ? 'awg-ghost0' : 'ghost0'})
                </button>
                <button
                  type="button"
                  onClick={() => onChangeInterfaceMode('system-proxy')}
                  className={`px-3 py-1 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
                    interfaceMode === 'system-proxy'
                      ? 'bg-[#252930] text-[#F1F3F5] border border-[#22C55E]/70'
                      : 'text-[#959CA6] border border-transparent hover:text-[#F1F3F5]'
                  }`}
                >
                  System Proxy
                </button>
              </div>
            </div>
          </div>
        </section>

        {/* Live Telemetry & Interface Metrics Card */}
        <section
          aria-label="Live interface throughput"
          className="xl:col-span-7 rounded-xl bg-[#1A1C20] border border-[#292D34] p-6 flex flex-col justify-between"
        >
          <div className="flex flex-wrap items-start justify-between gap-4 mb-4">
            <div>
              <div className="flex items-center gap-2 text-xs text-[#959CA6]">
                <Activity className="w-3.5 h-3.5 text-[#22C55E]" />
                <span>Interface Throughput</span>
              </div>
              <div className="flex flex-wrap items-baseline gap-6 mt-2">
                <div className="flex items-baseline gap-2">
                  <ArrowDown className="w-3.5 h-3.5 text-[#22C55E]" />
                  <span className="text-2xl font-semibold font-mono tabular-nums text-[#F1F3F5]">
                    {downloadMbps.toFixed(1)}
                  </span>
                  <span className="text-xs text-[#959CA6]">Mbps down</span>
                </div>
                <div className="flex items-baseline gap-2">
                  <ArrowUp className="w-3.5 h-3.5 text-[#858D98]" />
                  <span className="text-2xl font-semibold font-mono tabular-nums text-[#F1F3F5]">
                    {uploadMbps.toFixed(1)}
                  </span>
                  <span className="text-xs text-[#959CA6]">Mbps up</span>
                </div>
              </div>
            </div>

            <div className="flex items-center gap-6 text-xs font-mono tabular-nums">
              <div>
                <div className="text-[#7E8691]">Session RX</div>
                <div className="text-[#F1F3F5] mt-0.5">{formatDataSize(totalDownMb)}</div>
              </div>
              <div>
                <div className="text-[#7E8691]">Session TX</div>
                <div className="text-[#F1F3F5] mt-0.5">{formatDataSize(totalUpMb)}</div>
              </div>
            </div>
          </div>

          <ThroughputChart data={trafficHistory} isConnected={isConnected} />

          {/* Clean Unboxed Interface Summary Strip */}
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 pt-4 mt-4 border-t border-[#272A30] text-xs">
            <div>
              <div className="text-[#7E8691]">Engine</div>
              <div className="font-mono text-[#F1F3F5] mt-0.5">{coreEngine}</div>
            </div>
            <div>
              <div className="text-[#7E8691]">Local Bind</div>
              <div className="font-mono tabular-nums text-[#F1F3F5] mt-0.5">
                {interfaceMode === 'tun' ? '10.8.0.2 (ghost0)' : `127.0.0.1:${socksPort}`}
              </div>
            </div>
            <div>
              <div className="text-[#7E8691]">Exit IPv4</div>
              <div className="font-mono tabular-nums text-[#F1F3F5] mt-0.5">
                {isConnected && activeNode ? activeNode.host : 'Disconnected'}
              </div>
            </div>
            <div>
              <div className="text-[#7E8691]">Node Latency</div>
              <div
                className={`font-mono tabular-nums mt-0.5 ${
                  getLatencyTone(activeNode?.latencyMs ?? null).color
                }`}
              >
                {activeNode ? getLatencyTone(activeNode.latencyMs).text : '—'}
              </div>
            </div>
          </div>
        </section>
      </div>

      {/* Server Nodes Section */}
      <section
        aria-label="Server nodes"
        className="rounded-xl bg-[#1A1C20] border border-[#292D34] overflow-hidden"
      >
        {/* Toolbar */}
        <div className="p-5 border-b border-[#272A30] flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
          <div className="flex flex-wrap items-center gap-3">
            <h3 className="text-base font-semibold text-[#F1F3F5] mr-2">Server Nodes</h3>

            {/* Subscription Filter */}
            <div className="flex items-center gap-1 p-1 rounded-lg bg-[#131518] border border-[#272A30]">
              <button
                type="button"
                onClick={() => setSelectedSubFilter('all')}
                className={`px-3 py-1 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
                  selectedSubFilter === 'all'
                    ? 'bg-[#252930] text-[#F1F3F5]'
                    : 'text-[#959CA6] hover:text-[#F1F3F5]'
                }`}
              >
                All ({nodes.length})
              </button>
              {subscriptions.map((sub) => (
                <button
                  key={sub.id}
                  type="button"
                  onClick={() => setSelectedSubFilter(sub.id)}
                  className={`px-3 py-1 rounded-md text-xs font-medium transition-colors whitespace-nowrap max-w-[160px] truncate ${
                    selectedSubFilter === sub.id
                      ? 'bg-[#252930] text-[#F1F3F5]'
                      : 'text-[#959CA6] hover:text-[#F1F3F5]'
                  }`}
                >
                  {sub.name}
                </button>
              ))}
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            {/* Protocol Filter */}
            <div className="flex items-center gap-1.5">
              <SlidersHorizontal className="w-3.5 h-3.5 text-[#7E8691]" />
              <div className="relative">
                <select
                  aria-label="Filter by protocol"
                  value={protocolFilter}
                  onChange={(e) => setProtocolFilter(e.target.value as 'all' | NodeProtocol)}
                  className="appearance-none pl-2.5 pr-7 py-1.5 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs text-[#F1F3F5] focus:outline-none focus:border-[#22C55E]"
                >
                  <option value="all">All Protocols</option>
                  <option value="AmneziaWG">AmneziaWG</option>
                  <option value="WireGuard">WireGuard</option>
                  <option value="VLESS">VLESS</option>
                  <option value="Hysteria2">Hysteria2</option>
                  <option value="Trojan">Trojan</option>
                  <option value="VMess">VMess</option>
                  <option value="Shadowsocks">Shadowsocks</option>
                </select>
                <ChevronDown className="w-3 h-3 text-[#959CA6] absolute right-2 top-1/2 -translate-y-1/2 pointer-events-none" />
              </div>
            </div>

            {/* Search */}
            <div className="relative">
              <Search className="w-3.5 h-3.5 text-[#7E8691] absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search city, country, IP..."
                aria-label="Search server nodes"
                className="w-52 pl-8 pr-3 py-1.5 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs text-[#F1F3F5] placeholder-[#606772] focus:outline-none focus:border-[#22C55E]"
              />
            </div>

            {/* Ping All */}
            <button
              type="button"
              onClick={onPingAll}
              disabled={isPingingAll}
              className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-lg bg-[#22252A] border border-[#30353D] text-xs font-medium text-[#F1F3F5] hover:border-[#22C55E]/60 hover:text-[#22C55E] transition-colors disabled:opacity-60 whitespace-nowrap"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isPingingAll ? 'animate-spin text-[#22C55E]' : ''}`} />
              {isPingingAll ? 'Testing...' : 'Ping All'}
            </button>
          </div>
        </div>

        {/* High-Density Node Table */}
        <div className="divide-y divide-[#24272D]">
          {nodes.length === 0 ? (
            <div className="py-12 px-6 text-center">
              <Globe className="w-8 h-8 text-[#565C66] mx-auto mb-2" />
              <p className="text-sm font-medium text-[#F1F3F5]">No server profiles yet</p>
              <p className="text-xs text-[#959CA6] mt-1">
                Import a subscription or server URI (vless://, vmess://, trojan://, ss://) to begin.
              </p>
            </div>
          ) : filteredNodes.length === 0 ? (
            <div className="py-12 px-6 text-center">
              <Globe className="w-8 h-8 text-[#565C66] mx-auto mb-2" />
              <p className="text-sm font-medium text-[#F1F3F5]">No matching server nodes</p>
              <p className="text-xs text-[#959CA6] mt-1">
                Adjust your search query or protocol filter to view available endpoints.
              </p>
            </div>
          ) : (
            filteredNodes.map((node) => {
              const isSelected = activeNode ? node.id === activeNode.id : false;
              const latency = getLatencyTone(node.latencyMs);
              return (
                <div
                  key={node.id}
                  onClick={() => onSelectNode(node.id)}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      onSelectNode(node.id);
                    }
                  }}
                  className={`flex items-center justify-between gap-4 px-5 py-3.5 cursor-pointer transition-colors focus-visible:outline-none focus-visible:bg-[#1D2D23] ${
                    isSelected
                      ? 'bg-[#19261E] hover:bg-[#1D2D23]'
                      : 'hover:bg-[#1F2227]'
                  }`}
                >
                  <div className="flex items-center gap-4 min-w-0">
                    <div
                      className={`w-8 h-8 rounded-md flex items-center justify-center font-mono text-xs font-semibold shrink-0 border ${
                        isSelected
                          ? 'bg-[#22C55E]/15 border-[#22C55E]/50 text-[#22C55E]'
                          : 'bg-[#141619] border-[#2A2E35] text-[#A1A7B0]'
                      }`}
                    >
                      {node.countryCode}
                    </div>
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span
                          className={`text-sm font-medium truncate ${
                            isSelected ? 'text-[#22C55E]' : 'text-[#F1F3F5]'
                          }`}
                        >
                          {node.name}
                        </span>
                        {isSelected && (
                          <Check className="w-3.5 h-3.5 text-[#22C55E] shrink-0" aria-label="Active node" />
                        )}
                      </div>
                      <div className="text-xs text-[#858D98] truncate mt-0.5">
                        {node.protocol} · {node.transport} · {node.security} · {node.city}
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center gap-6 shrink-0">
                    <div className="hidden md:block text-right">
                      <div className="text-xs font-mono tabular-nums text-[#959CA6]">
                        {node.host}:{node.port}
                      </div>
                      <div className="text-[11px] font-mono tabular-nums text-[#717883]">
                        Load {node.loadPercent}%
                      </div>
                    </div>

                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        onPingNode(node.id);
                      }}
                      title="Test node latency"
                      className="min-w-[72px] text-right px-2.5 py-1 rounded-md hover:bg-[#282C33] transition-colors font-mono text-xs tabular-nums"
                    >
                      {node.pinging ? (
                        <span className="text-[#959CA6]">...</span>
                      ) : (
                        <span className={latency.color}>{latency.text}</span>
                      )}
                    </button>
                  </div>
                </div>
              );
            })
          )}
        </div>
      </section>
    </div>
  );
};
