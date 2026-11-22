import React, { useMemo, useState } from 'react';
import {
  Activity,
  ArrowDown,
  ArrowUp,
  Check,
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
  activeNode: ServerNode;
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
          className="xl:col-span-5 rounded-xl bg-[#101512] border border-[#1D2721] p-6 flex flex-col justify-between"
        >
          <div className="flex items-start justify-between gap-4">
            <div>
              <div className="flex items-center gap-2 text-xs font-medium">
                {isConnected ? (
                  <ShieldCheck className="w-4 h-4 text-[#22C55E]" />
                ) : isConnecting ? (
                  <Loader2 className="w-4 h-4 text-[#FBBF24] animate-spin" />
                ) : (
                  <ShieldOff className="w-4 h-4 text-[#78887F]" />
                )}
                <span
                  className={
                    isConnected
                      ? 'text-[#22C55E]'
                      : isConnecting
                        ? 'text-[#FBBF24]'
                        : 'text-[#8A9990]'
                  }
                >
                  {isConnected
                    ? 'Tunnel Connected'
                    : isConnecting
                      ? 'Establishing Handshake'
                      : 'Tunnel Disconnected'}
                </span>
              </div>
              <h2 className="text-xl font-semibold text-[#ECF2EE] mt-2 tracking-tight">
                {activeNode.name}
              </h2>
              <p className="text-xs text-[#8A9990] mt-1 font-mono tabular-nums">
                {activeNode.protocol} · {activeNode.security} · {activeNode.host}:{activeNode.port}
              </p>
            </div>

            <div className="text-right shrink-0">
              <div className="text-xs text-[#78887F]">Session</div>
              <div className="text-sm font-mono tabular-nums text-[#ECF2EE] mt-0.5">
                {formatDuration(sessionSeconds)}
              </div>
            </div>
          </div>

          {/* Central Tactile Power Toggle */}
          <div className="py-7 flex items-center justify-center">
            <button
              type="button"
              onClick={onToggleConnect}
              disabled={isConnecting}
              aria-label={isConnected ? 'Disconnect VPN tunnel' : 'Connect VPN tunnel'}
              className={`group relative w-32 h-32 rounded-full flex flex-col items-center justify-center transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#22C55E] focus-visible:ring-offset-2 focus-visible:ring-offset-[#101512] ${
                isConnected
                  ? 'bg-[#22C55E]/15 border-2 border-[#22C55E] text-[#22C55E] shadow-[0_0_32px_rgba(34,197,94,0.18)] hover:bg-[#22C55E]/20'
                  : isConnecting
                    ? 'bg-[#FBBF24]/10 border-2 border-[#FBBF24]/60 text-[#FBBF24] cursor-wait'
                    : 'bg-[#151C18] border-2 border-[#27352D] text-[#ECF2EE] hover:border-[#22C55E]/70 hover:text-[#22C55E]'
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
          <div className="space-y-3 pt-4 border-t border-[#1A231E]">
            <div className="flex items-center justify-between gap-4">
              <span className="text-xs text-[#8A9990] whitespace-nowrap">Routing Mode</span>
              <div className="grid grid-cols-3 gap-1 p-1 rounded-lg bg-[#0A0D0B] border border-[#1B241F]">
                {(['rule', 'global', 'direct'] as RoutingMode[]).map((mode) => (
                  <button
                    key={mode}
                    type="button"
                    onClick={() => onChangeRoutingMode(mode)}
                    className={`px-3 py-1 rounded-md text-xs font-medium capitalize transition-colors whitespace-nowrap ${
                      routingMode === mode
                        ? 'bg-[#22C55E] text-[#05140A] font-semibold'
                        : 'text-[#8A9990] hover:text-[#ECF2EE]'
                    }`}
                  >
                    {mode}
                  </button>
                ))}
              </div>
            </div>

            <div className="flex items-center justify-between gap-4">
              <span className="text-xs text-[#8A9990] whitespace-nowrap">Interface</span>
              <div className="grid grid-cols-2 gap-1 p-1 rounded-lg bg-[#0A0D0B] border border-[#1B241F]">
                <button
                  type="button"
                  onClick={() => onChangeInterfaceMode('tun')}
                  className={`px-3 py-1 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
                    interfaceMode === 'tun'
                      ? 'bg-[#19241E] text-[#22C55E] border border-[#283B30]'
                      : 'text-[#8A9990] hover:text-[#ECF2EE]'
                  }`}
                >
                  TUN (ghost0)
                </button>
                <button
                  type="button"
                  onClick={() => onChangeInterfaceMode('system-proxy')}
                  className={`px-3 py-1 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
                    interfaceMode === 'system-proxy'
                      ? 'bg-[#19241E] text-[#22C55E] border border-[#283B30]'
                      : 'text-[#8A9990] hover:text-[#ECF2EE]'
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
          className="xl:col-span-7 rounded-xl bg-[#101512] border border-[#1D2721] p-6 flex flex-col justify-between"
        >
          <div className="flex flex-wrap items-start justify-between gap-4 mb-4">
            <div>
              <div className="flex items-center gap-2 text-xs text-[#8A9990]">
                <Activity className="w-3.5 h-3.5 text-[#22C55E]" />
                <span>Interface Throughput</span>
              </div>
              <div className="flex flex-wrap items-baseline gap-6 mt-2">
                <div className="flex items-baseline gap-2">
                  <ArrowDown className="w-3.5 h-3.5 text-[#22C55E]" />
                  <span className="text-2xl font-semibold font-mono tabular-nums text-[#ECF2EE]">
                    {downloadMbps.toFixed(1)}
                  </span>
                  <span className="text-xs text-[#8A9990]">Mbps down</span>
                </div>
                <div className="flex items-baseline gap-2">
                  <ArrowUp className="w-3.5 h-3.5 text-[#7D8E84]" />
                  <span className="text-2xl font-semibold font-mono tabular-nums text-[#ECF2EE]">
                    {uploadMbps.toFixed(1)}
                  </span>
                  <span className="text-xs text-[#8A9990]">Mbps up</span>
                </div>
              </div>
            </div>

            <div className="flex items-center gap-6 text-xs font-mono tabular-nums">
              <div>
                <div className="text-[#78887F]">Session RX</div>
                <div className="text-[#ECF2EE] mt-0.5">{formatDataSize(totalDownMb)}</div>
              </div>
              <div>
                <div className="text-[#78887F]">Session TX</div>
                <div className="text-[#ECF2EE] mt-0.5">{formatDataSize(totalUpMb)}</div>
              </div>
            </div>
          </div>

          <ThroughputChart data={trafficHistory} isConnected={isConnected} />

          {/* Clean Unboxed Interface Summary Strip */}
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 pt-4 mt-4 border-t border-[#1A231E] text-xs">
            <div>
              <div className="text-[#78887F]">Engine</div>
              <div className="font-mono text-[#ECF2EE] mt-0.5">{coreEngine}</div>
            </div>
            <div>
              <div className="text-[#78887F]">Local Bind</div>
              <div className="font-mono tabular-nums text-[#ECF2EE] mt-0.5">
                {interfaceMode === 'tun' ? '10.8.0.2 (ghost0)' : `127.0.0.1:${socksPort}`}
              </div>
            </div>
            <div>
              <div className="text-[#78887F]">Exit IPv4</div>
              <div className="font-mono tabular-nums text-[#ECF2EE] mt-0.5">
                {isConnected ? activeNode.host : 'Local ISP'}
              </div>
            </div>
            <div>
              <div className="text-[#78887F]">Node Latency</div>
              <div
                className={`font-mono tabular-nums mt-0.5 ${
                  getLatencyTone(activeNode.latencyMs).color
                }`}
              >
                {getLatencyTone(activeNode.latencyMs).text}
              </div>
            </div>
          </div>
        </section>
      </div>

      {/* Server Nodes Section */}
      <section
        aria-label="Server nodes"
        className="rounded-xl bg-[#101512] border border-[#1D2721] overflow-hidden"
      >
        {/* Toolbar */}
        <div className="p-5 border-b border-[#1A231E] flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
          <div className="flex flex-wrap items-center gap-3">
            <h3 className="text-base font-semibold text-[#ECF2EE] mr-2">Server Nodes</h3>

            {/* Subscription Filter */}
            <div className="flex items-center gap-1 p-1 rounded-lg bg-[#0A0D0B] border border-[#1B241F]">
              <button
                type="button"
                onClick={() => setSelectedSubFilter('all')}
                className={`px-3 py-1 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
                  selectedSubFilter === 'all'
                    ? 'bg-[#19241E] text-[#ECF2EE]'
                    : 'text-[#8A9990] hover:text-[#ECF2EE]'
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
                      ? 'bg-[#19241E] text-[#ECF2EE]'
                      : 'text-[#8A9990] hover:text-[#ECF2EE]'
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
              <SlidersHorizontal className="w-3.5 h-3.5 text-[#78887F]" />
              <select
                aria-label="Filter by protocol"
                value={protocolFilter}
                onChange={(e) => setProtocolFilter(e.target.value as 'all' | NodeProtocol)}
                className="px-2.5 py-1.5 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs text-[#ECF2EE] focus:outline-none focus:border-[#22C55E]"
              >
                <option value="all">All Protocols</option>
                <option value="VLESS">VLESS</option>
                <option value="Hysteria2">Hysteria2</option>
                <option value="WireGuard">WireGuard</option>
                <option value="Trojan">Trojan</option>
              </select>
            </div>

            {/* Search */}
            <div className="relative">
              <Search className="w-3.5 h-3.5 text-[#78887F] absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search city, country, IP..."
                aria-label="Search server nodes"
                className="w-52 pl-8 pr-3 py-1.5 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs text-[#ECF2EE] placeholder-[#5A6A61] focus:outline-none focus:border-[#22C55E]"
              />
            </div>

            {/* Ping All */}
            <button
              type="button"
              onClick={onPingAll}
              disabled={isPingingAll}
              className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-lg bg-[#161F1A] border border-[#233028] text-xs font-medium text-[#ECF2EE] hover:border-[#22C55E]/60 hover:text-[#22C55E] transition-colors disabled:opacity-60 whitespace-nowrap"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isPingingAll ? 'animate-spin text-[#22C55E]' : ''}`} />
              {isPingingAll ? 'Testing...' : 'Ping All'}
            </button>
          </div>
        </div>

        {/* High-Density Node Table */}
        <div className="divide-y divide-[#17201B]">
          {filteredNodes.length === 0 ? (
            <div className="py-12 px-6 text-center">
              <Globe className="w-8 h-8 text-[#4F5E55] mx-auto mb-2" />
              <p className="text-sm font-medium text-[#ECF2EE]">No matching server nodes</p>
              <p className="text-xs text-[#8A9990] mt-1">
                Adjust your search query or protocol filter to view available endpoints.
              </p>
            </div>
          ) : (
            filteredNodes.map((node) => {
              const isSelected = node.id === activeNode.id;
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
                  className={`flex items-center justify-between gap-4 px-5 py-3.5 cursor-pointer transition-colors focus-visible:outline-none focus-visible:bg-[#17221C] ${
                    isSelected
                      ? 'bg-[#142019] hover:bg-[#16251D]'
                      : 'hover:bg-[#141B17]'
                  }`}
                >
                  <div className="flex items-center gap-4 min-w-0">
                    <div
                      className={`w-8 h-8 rounded-md flex items-center justify-center font-mono text-xs font-semibold shrink-0 border ${
                        isSelected
                          ? 'bg-[#22C55E]/15 border-[#22C55E]/50 text-[#22C55E]'
                          : 'bg-[#0C100E] border-[#1E2822] text-[#9AA8A0]'
                      }`}
                    >
                      {node.countryCode}
                    </div>
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span
                          className={`text-sm font-medium truncate ${
                            isSelected ? 'text-[#22C55E]' : 'text-[#ECF2EE]'
                          }`}
                        >
                          {node.name}
                        </span>
                        {isSelected && (
                          <Check className="w-3.5 h-3.5 text-[#22C55E] shrink-0" aria-label="Active node" />
                        )}
                      </div>
                      <div className="text-xs text-[#7D8C84] truncate mt-0.5">
                        {node.protocol} · {node.transport} · {node.security} · {node.city}
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center gap-6 shrink-0">
                    <div className="hidden md:block text-right">
                      <div className="text-xs font-mono tabular-nums text-[#8A9990]">
                        {node.host}:{node.port}
                      </div>
                      <div className="text-[11px] font-mono tabular-nums text-[#67766E]">
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
                      className="min-w-[72px] text-right px-2.5 py-1 rounded-md hover:bg-[#1B2520] transition-colors font-mono text-xs tabular-nums"
                    >
                      {node.pinging ? (
                        <span className="text-[#8A9990]">...</span>
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
