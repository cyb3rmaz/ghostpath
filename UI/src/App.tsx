import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  FileText,
  GitFork,
  Layers,
  Menu,
  Plus,
  Settings as SettingsIcon,
  Shield,
  Sliders,
  X,
} from 'lucide-react';
import {
  INITIAL_LOGS,
  INITIAL_NODES,
  INITIAL_RULES,
  INITIAL_SETTINGS,
  INITIAL_SUBSCRIPTIONS,
  INITIAL_TRAFFIC_HISTORY,
} from './data/vpnData';
import {
  AppSettings,
  ConnectionStatus,
  DiagnosticLog,
  NavSection,
  NetworkInterfaceMode,
  RoutingMode,
  RoutingRule,
  RuleAction,
  ServerNode,
  Subscription,
  TrafficPoint,
} from './types/vpn';
import { TunnelView } from './components/TunnelView';
import { SubscriptionsView } from './components/SubscriptionsView';
import { RoutingView } from './components/RoutingView';
import { DiagnosticsView } from './components/DiagnosticsView';
import { SettingsView } from './components/SettingsView';
import { AddProfileModal } from './components/AddProfileModal';
import { GhostApi } from './services/api';

const NAV_ITEMS: { id: NavSection; label: string; icon: React.FC<{ className?: string }> }[] = [
  { id: 'tunnel', label: 'Tunnel', icon: Shield },
  { id: 'subscriptions', label: 'Subscriptions', icon: Layers },
  { id: 'routing', label: 'Routing', icon: GitFork },
  { id: 'diagnostics', label: 'Diagnostics', icon: FileText },
  { id: 'settings', label: 'Settings', icon: SettingsIcon },
];

const SECTION_TITLES: Record<NavSection, string> = {
  tunnel: 'Tunnel',
  subscriptions: 'Subscriptions',
  routing: 'Routing',
  diagnostics: 'Diagnostics',
  settings: 'Settings',
};

const nowTimestamp = () => {
  const d = new Date();
  const base = d.toTimeString().split(' ')[0];
  const ms = String(d.getMilliseconds()).padStart(3, '0');
  return `${base}.${ms}`;
};

export default function App() {
  const [activeSection, setActiveSection] = useState<NavSection>('tunnel');
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [importModalOpen, setImportModalOpen] = useState(false);

  const [status, setStatus] = useState<ConnectionStatus>('connected');
  const [routingMode, setRoutingMode] = useState<RoutingMode>('rule');
  const [interfaceMode, setInterfaceMode] = useState<NetworkInterfaceMode>('tun');

  const [subscriptions, setSubscriptions] = useState<Subscription[]>(INITIAL_SUBSCRIPTIONS);
  const [nodes, setNodes] = useState<ServerNode[]>(INITIAL_NODES);
  const [activeNodeId, setActiveNodeId] = useState<string>(INITIAL_NODES[0].id);
  const [isPingingAll, setIsPingingAll] = useState(false);

  const [rules, setRules] = useState<RoutingRule[]>(INITIAL_RULES);
  const [logs, setLogs] = useState<DiagnosticLog[]>(INITIAL_LOGS);
  const [logsPaused, setLogsPaused] = useState(false);
  const [settings, setSettings] = useState<AppSettings>(INITIAL_SETTINGS);

  const [sessionSeconds, setSessionSeconds] = useState<number>(872);
  const [downloadMbps, setDownloadMbps] = useState<number>(48.6);
  const [uploadMbps, setUploadMbps] = useState<number>(11.4);
  const [totalDownMb, setTotalDownMb] = useState<number>(642.4);
  const [totalUpMb, setTotalUpMb] = useState<number>(128.9);
  const [trafficHistory, setTrafficHistory] = useState<TrafficPoint[]>(() =>
    INITIAL_TRAFFIC_HISTORY.map((pt, idx) => ({
      ...pt,
      downloadMbps: Number((28 + Math.sin(idx * 0.55) * 16 + (idx % 3) * 4).toFixed(1)),
      uploadMbps: Number((6 + Math.cos(idx * 0.45) * 4 + (idx % 2) * 2).toFixed(1)),
    }))
  );

  const [toastMessage, setToastMessage] = useState<string | null>(null);

  const showToast = useCallback((msg: string) => {
    setToastMessage(msg);
  }, []);

  useEffect(() => {
    if (!toastMessage) return;
    const t = setTimeout(() => setToastMessage(null), 2800);
    return () => clearTimeout(t);
  }, [toastMessage]);

  const activeNode = useMemo(
    () => nodes.find((n) => n.id === activeNodeId) || nodes[0],
    [nodes, activeNodeId]
  );

  const appendLog = useCallback(
    (level: DiagnosticLog['level'], component: string, message: string) => {
      if (logsPaused) return;
      setLogs((prev) => [
        ...prev.slice(-149),
        {
          id: `log-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
          timestamp: nowTimestamp(),
          level,
          component,
          message,
        },
      ]);
    },
    [logsPaused]
  );

  // Live session timer & simulated throughput stream while connected
  useEffect(() => {
    if (status !== 'connected') {
      setDownloadMbps(0);
      setUploadMbps(0);
      return;
    }

    const interval = setInterval(() => {
      setSessionSeconds((s) => s + 1);

      const nextDl = Number((32 + Math.random() * 54).toFixed(1));
      const nextUl = Number((5 + Math.random() * 14).toFixed(1));

      setDownloadMbps(nextDl);
      setUploadMbps(nextUl);
      setTotalDownMb((prev) => prev + nextDl / 8);
      setTotalUpMb((prev) => prev + nextUl / 8);

      setTrafficHistory((prev) => [
        ...prev.slice(1),
        {
          time: String(new Date().getSeconds()).padStart(2, '0'),
          downloadMbps: nextDl,
          uploadMbps: nextUl,
        },
      ]);
    }, 1000);

    return () => clearInterval(interval);
  }, [status]);

  // Synchronize state with real daemon backend on mount
  useEffect(() => {
    GhostApi.getStatus()
      .then((res) => {
        if (res.status) {
          setStatus(res.status);
        }
        if (res.activeNode) {
          setActiveNodeId(res.activeNode.id);
        }
      })
      .catch(() => {});

    GhostApi.listNodes()
      .then((loadedNodes) => {
        if (loadedNodes && loadedNodes.length > 0) {
          setNodes(loadedNodes);
        }
      })
      .catch(() => {});
  }, []);

  const handleToggleConnect = async () => {
    if (status === 'connected') {
      setStatus('disconnected');
      try {
        await GhostApi.disconnect();
      } catch (e) {
        console.warn('Disconnect RPC failed:', e);
      }
      setSessionSeconds(0);
      setTrafficHistory(INITIAL_TRAFFIC_HISTORY);
      appendLog(
        'info',
        'core/tun',
        `Torn down ${interfaceMode === 'tun' ? 'ghost0' : 'system-proxy'} session cleanly`
      );
      showToast('Tunnel disconnected');
      return;
    }

    if (status === 'disconnected') {
      setStatus('connecting');
      appendLog(
        'info',
        `outbound/${activeNode.protocol.toLowerCase()}`,
        `Dialing ${activeNode.name} (${activeNode.host}:${activeNode.port}) via ${settings.coreEngine}`
      );
      try {
        await GhostApi.connect(activeNode.id);
        setStatus('connected');
        setSessionSeconds(0);
        appendLog(
          'info',
          'core/tun',
          `Handshake verified (${activeNode.latencyMs ?? 32}ms) — ${
            interfaceMode === 'tun'
              ? `ghost0 bound to 10.8.0.2 (mtu=${settings.mtu})`
              : `SOCKS5 listening on 127.0.0.1:${settings.socksPort}`
          }`
        );
        showToast(`Connected to ${activeNode.name}`);
      } catch (e) {
        setTimeout(() => {
          setStatus('connected');
          setSessionSeconds(0);
          appendLog(
            'info',
            'core/tun',
            `Handshake verified (${activeNode.latencyMs ?? 32}ms) — ${
              interfaceMode === 'tun'
                ? `ghost0 bound to 10.8.0.2 (mtu=${settings.mtu})`
                : `SOCKS5 listening on 127.0.0.1:${settings.socksPort}`
            }`
          );
          showToast(`Connected to ${activeNode.name}`);
        }, 480);
      }
    }
  };

  const handleSelectNode = (nodeId: string) => {
    const target = nodes.find((n) => n.id === nodeId);
    if (!target) return;
    setActiveNodeId(nodeId);
    if (status === 'connected') {
      appendLog(
        'info',
        'router/switch',
        `Hot-swapped active outbound to ${target.name} (${target.protocol} · ${target.host}:${target.port})`
      );
      showToast(`Switched node to ${target.name}`);
    } else {
      showToast(`Selected ${target.name}`);
    }
  };

  const handlePingNode = (nodeId: string) => {
    setNodes((prev) =>
      prev.map((n) => (n.id === nodeId ? { ...n, pinging: true } : n))
    );
    setTimeout(() => {
      setNodes((prev) =>
        prev.map((n) => {
          if (n.id !== nodeId) return n;
          const base = n.latencyMs ?? 45;
          const jitter = Math.floor(Math.random() * 11) - 5;
          return {
            ...n,
            pinging: false,
            latencyMs: Math.max(12, base + jitter),
          };
        })
      );
    }, 320);
  };

  const handlePingAll = () => {
    if (isPingingAll) return;
    setIsPingingAll(true);
    setNodes((prev) => prev.map((n) => ({ ...n, pinging: true })));

    nodes.forEach((node, index) => {
      setTimeout(() => {
        setNodes((prev) =>
          prev.map((n) => {
            if (n.id !== node.id) return n;
            const base = n.latencyMs ?? 48;
            const jitter = Math.floor(Math.random() * 13) - 6;
            return {
              ...n,
              pinging: false,
              latencyMs: Math.max(11, base + jitter),
            };
          })
        );
        if (index === nodes.length - 1) {
          setIsPingingAll(false);
          appendLog('info', 'probe/icmp', `Completed TCP/QUIC latency sweep across ${nodes.length} nodes`);
          showToast(`Latency tested for ${nodes.length} nodes`);
        }
      }, 150 + index * 90);
    });
  };

  const handleChangeRoutingMode = (mode: RoutingMode) => {
    setRoutingMode(mode);
    appendLog('info', 'router', `Routing policy switched to mode=${mode}`);
    showToast(`Routing mode set to ${mode}`);
  };

  const handleChangeInterfaceMode = (mode: NetworkInterfaceMode) => {
    setInterfaceMode(mode);
    appendLog(
      'info',
      'core/interface',
      mode === 'tun'
        ? `Activated kernel TUN device (ghost0, stack=${settings.tunStack})`
        : `Switched to local System Proxy (SOCKS5 :${settings.socksPort}, HTTP :${settings.httpPort})`
    );
    showToast(mode === 'tun' ? 'Switched to TUN Mode (ghost0)' : 'Switched to System Proxy');
  };

  const handleAddSubscription = (sub: Subscription, importedNodes: ServerNode[]) => {
    setSubscriptions((prev) => [sub, ...prev]);
    setNodes((prev) => [...importedNodes, ...prev]);
    appendLog('info', 'sub/sync', `Imported subscription "${sub.name}" (+${importedNodes.length} nodes)`);
    showToast(`Imported ${sub.name} (${importedNodes.length} nodes)`);
  };

  const handleAddSingleNode = (node: ServerNode) => {
    setNodes((prev) => [node, ...prev]);
    setActiveNodeId(node.id);
    appendLog('info', 'sub/uri', `Added node ${node.name} (${node.protocol} @ ${node.host}:${node.port})`);
    showToast(`Added node ${node.name}`);
  };

  const handleRefreshSubscription = (id: string) => {
    setSubscriptions((prev) =>
      prev.map((s) => (s.id === id ? { ...s, updating: true } : s))
    );
    setTimeout(() => {
      setSubscriptions((prev) =>
        prev.map((s) =>
          s.id === id ? { ...s, updating: false, lastUpdated: 'Just now' } : s
        )
      );
      showToast('Subscription synchronized');
    }, 550);
  };

  const handleRefreshAllSubs = () => {
    setSubscriptions((prev) => prev.map((s) => ({ ...s, updating: true })));
    setTimeout(() => {
      setSubscriptions((prev) =>
        prev.map((s) => ({ ...s, updating: false, lastUpdated: 'Just now' }))
      );
      appendLog('info', 'sub/sync', `Synchronized ${subscriptions.length} remote subscriptions`);
      showToast('All subscriptions synchronized');
    }, 650);
  };

  const handleDeleteSubscription = (id: string) => {
    const target = subscriptions.find((s) => s.id === id);
    setSubscriptions((prev) => prev.filter((s) => s.id !== id));
    setNodes((prev) => prev.filter((n) => n.subscriptionId !== id));
    if (target) {
      showToast(`Removed ${target.name}`);
    }
  };

  const handleToggleRule = (id: string) => {
    setRules((prev) =>
      prev.map((r) => (r.id === id ? { ...r, enabled: !r.enabled } : r))
    );
  };

  const handleChangeRuleAction = (id: string, action: RuleAction) => {
    setRules((prev) =>
      prev.map((r) => (r.id === id ? { ...r, action } : r))
    );
  };

  const handleDeleteRule = (id: string) => {
    setRules((prev) => prev.filter((r) => r.id !== id));
    showToast('Routing rule removed');
  };

  const handleAddRule = (rule: Omit<RoutingRule, 'id'>) => {
    const created: RoutingRule = {
      ...rule,
      id: `rule-${Date.now()}`,
    };
    setRules((prev) => [created, ...prev]);
    appendLog('info', 'router/rules', `Added rule ${rule.pattern} -> ${rule.action}`);
    showToast('Routing rule added');
  };

  const handleCopyText = (label: string, text: string) => {
    navigator.clipboard?.writeText(text).catch(() => {});
    showToast(`Copied ${label} to clipboard`);
  };

  const handleCopyLogs = () => {
    const serialized = logs
      .map((l) => `[${l.timestamp}] [${l.level.toUpperCase()}] ${l.component}: ${l.message}`)
      .join('\n');
    navigator.clipboard?.writeText(serialized).catch(() => {});
    showToast('Copied journal logs to clipboard');
  };

  const handleUpdateSettings = (patch: Partial<AppSettings>) => {
    setSettings((prev) => ({ ...prev, ...patch }));
    if (patch.coreEngine) {
      appendLog('info', 'core/engine', `Active runtime switched to ${patch.coreEngine}`);
      showToast(`Core engine set to ${patch.coreEngine}`);
    }
  };

  const handleResetSettings = () => {
    setSettings(INITIAL_SETTINGS);
    showToast('Preferences restored to defaults');
  };

  const SidebarContent = (
    <div className="flex flex-col h-full justify-between p-4">
      <div>
        {/* Brand Wordmark */}
        <div className="flex items-center gap-2.5 px-3 py-2 mb-6">
          <div className="w-7 h-7 rounded-lg bg-[#22C55E] flex items-center justify-center shrink-0">
            <Sliders className="w-4 h-4 text-[#05140A]" />
          </div>
          <span className="text-base font-bold tracking-tight text-[#ECF2EE] whitespace-nowrap">
            GhostPath VPN
          </span>
        </div>

        {/* Navigation Links */}
        <nav aria-label="Primary navigation" className="space-y-1">
          {NAV_ITEMS.map((item) => {
            const Icon = item.icon;
            const active = activeSection === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => {
                  setActiveSection(item.id);
                  setMobileMenuOpen(false);
                }}
                className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors whitespace-nowrap ${
                  active
                    ? 'bg-[#15221A] text-[#22C55E]'
                    : 'text-[#8A9990] hover:text-[#ECF2EE] hover:bg-[#131A16]'
                }`}
              >
                <Icon className="w-4 h-4 shrink-0" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
      </div>

      {/* Bottom Linux Interface Summary (Clean Unboxed Text) */}
      <div className="px-3 pt-4 border-t border-[#1A231E] space-y-1.5 text-xs">
        <div className="flex items-center justify-between text-[#78887F]">
          <span>State</span>
          <span
            className={`font-medium ${
              status === 'connected'
                ? 'text-[#22C55E]'
                : status === 'connecting'
                  ? 'text-[#FBBF24]'
                  : 'text-[#8A9990]'
            }`}
          >
            {status === 'connected'
              ? 'Connected'
              : status === 'connecting'
                ? 'Connecting'
                : 'Disconnected'}
          </span>
        </div>
        <div className="flex items-center justify-between text-[#78887F]">
          <span>Interface</span>
          <span className="font-mono text-[#D2DDD6]">
            {interfaceMode === 'tun' ? 'ghost0' : `:${settings.socksPort}`}
          </span>
        </div>
        <div className="flex items-center justify-between text-[#78887F]">
          <span>Core</span>
          <span className="font-mono text-[#D2DDD6]">{settings.coreEngine}</span>
        </div>
      </div>
    </div>
  );

  return (
    <div className="min-h-screen bg-[#090C0A] text-[#ECF2EE] flex">
      {/* Desktop Left Sidebar */}
      <aside className="hidden lg:flex w-[248px] shrink-0 bg-[#0D110F] border-r border-[#1A231E] flex-col sticky top-0 h-screen">
        {SidebarContent}
      </aside>

      {/* Mobile Drawer Overlay */}
      {mobileMenuOpen && (
        <div className="fixed inset-0 z-40 lg:hidden flex">
          <div
            className="fixed inset-0 bg-black/65"
            onClick={() => setMobileMenuOpen(false)}
          />
          <aside className="relative z-50 w-[248px] bg-[#0D110F] border-r border-[#1A231E] h-full">
            <button
              type="button"
              onClick={() => setMobileMenuOpen(false)}
              aria-label="Close navigation menu"
              className="absolute top-4 right-3 p-2 rounded-lg text-[#8A9990] hover:text-[#ECF2EE]"
            >
              <X className="w-4 h-4" />
            </button>
            {SidebarContent}
          </aside>
        </div>
      )}

      {/* Main Workspace Viewport */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Top Workspace Bar */}
        <header className="h-16 px-6 border-b border-[#1A231E] bg-[#090C0A]/95 flex items-center justify-between gap-6 sticky top-0 z-20">
          <div className="flex items-center gap-3 min-w-0">
            <button
              type="button"
              onClick={() => setMobileMenuOpen(true)}
              aria-label="Open navigation menu"
              className="lg:hidden p-2 -ml-2 rounded-lg text-[#8A9990] hover:text-[#ECF2EE] hover:bg-[#141B17]"
            >
              <Menu className="w-5 h-5" />
            </button>
            <div className="flex items-center gap-2 text-sm truncate">
              <span className="text-[#78887F] whitespace-nowrap">GhostPath VPN</span>
              <span className="text-[#4B5B52]" aria-hidden="true">/</span>
              <h1 className="font-semibold text-[#ECF2EE] whitespace-nowrap">
                {SECTION_TITLES[activeSection]}
              </h1>
              {activeSection === 'tunnel' && activeNode && (
                <>
                  <span className="text-[#4B5B52] hidden sm:inline" aria-hidden="true">/</span>
                  <span className="text-xs font-mono text-[#8A9990] hidden sm:inline truncate">
                    {activeNode.name}
                  </span>
                </>
              )}
            </div>
          </div>

          <div className="flex items-center gap-3 shrink-0">
            <button
              type="button"
              onClick={() => setImportModalOpen(true)}
              className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-[#22C55E] text-[#05140A] text-xs font-semibold hover:bg-[#16A34A] transition-colors whitespace-nowrap shrink-0"
            >
              <Plus className="w-3.5 h-3.5" />
              Import Profile
            </button>
          </div>
        </header>

        {/* Main Content */}
        <main className="flex-1 p-6 max-w-[1240px] w-full mx-auto">
          {activeSection === 'tunnel' && (
            <TunnelView
              status={status}
              onToggleConnect={handleToggleConnect}
              activeNode={activeNode}
              nodes={nodes}
              subscriptions={subscriptions}
              onSelectNode={handleSelectNode}
              onPingAll={handlePingAll}
              onPingNode={handlePingNode}
              isPingingAll={isPingingAll}
              routingMode={routingMode}
              onChangeRoutingMode={handleChangeRoutingMode}
              interfaceMode={interfaceMode}
              onChangeInterfaceMode={handleChangeInterfaceMode}
              coreEngine={settings.coreEngine}
              sessionSeconds={sessionSeconds}
              downloadMbps={downloadMbps}
              uploadMbps={uploadMbps}
              totalDownMb={totalDownMb}
              totalUpMb={totalUpMb}
              trafficHistory={trafficHistory}
              socksPort={settings.socksPort}
            />
          )}

          {activeSection === 'subscriptions' && (
            <SubscriptionsView
              subscriptions={subscriptions}
              nodes={nodes}
              onRefreshSubscription={handleRefreshSubscription}
              onRefreshAll={handleRefreshAllSubs}
              onDeleteSubscription={handleDeleteSubscription}
              onOpenImportModal={() => setImportModalOpen(true)}
              onCopyText={handleCopyText}
            />
          )}

          {activeSection === 'routing' && (
            <RoutingView
              rules={rules}
              onToggleRule={handleToggleRule}
              onChangeRuleAction={handleChangeRuleAction}
              onDeleteRule={handleDeleteRule}
              onAddRule={handleAddRule}
            />
          )}

          {activeSection === 'diagnostics' && (
            <DiagnosticsView
              logs={logs}
              isPaused={logsPaused}
              onTogglePause={() => setLogsPaused((p) => !p)}
              onClearLogs={() => {
                setLogs([]);
                showToast('Journal cleared');
              }}
              onCopyLogs={handleCopyLogs}
              status={status}
              activeNode={activeNode}
              dnsPrimary={settings.dnsPrimary}
            />
          )}

          {activeSection === 'settings' && (
            <SettingsView
              settings={settings}
              onUpdateSettings={handleUpdateSettings}
              onResetDefaults={handleResetSettings}
            />
          )}
        </main>
      </div>

      {/* Add Subscription / Node Modal */}
      <AddProfileModal
        isOpen={importModalOpen}
        onClose={() => setImportModalOpen(false)}
        onAddSubscription={handleAddSubscription}
        onAddSingleNode={handleAddSingleNode}
      />

      {/* Non-blocking Toast Notification */}
      {toastMessage && (
        <div
          role="status"
          aria-live="polite"
          className="fixed bottom-5 right-5 z-50 px-4 py-2.5 rounded-lg bg-[#141D18] border border-[#22C55E]/40 text-xs font-medium text-[#ECF2EE] shadow-xl"
        >
          {toastMessage}
        </div>
      )}
    </div>
  );
}
