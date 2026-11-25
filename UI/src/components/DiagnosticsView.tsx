import React, { useMemo, useState } from 'react';
import { Copy, Pause, Play, Search, ShieldCheck, Trash2 } from 'lucide-react';
import { ConnectionStatus, DiagnosticLog, LogLevel, ServerNode } from '../types/vpn';
import { GhostApi } from '../services/api';

interface DiagnosticsViewProps {
  logs: DiagnosticLog[];
  isPaused: boolean;
  onTogglePause: () => void;
  onClearLogs: () => void;
  onCopyLogs: () => void;
  status: ConnectionStatus;
  activeNode?: ServerNode | null;
  dnsPrimary: string;
}

export const DiagnosticsView: React.FC<DiagnosticsViewProps> = ({
  logs,
  isPaused,
  onTogglePause,
  onClearLogs,
  onCopyLogs,
  status,
  activeNode,
  dnsPrimary,
}) => {
  const [levelFilter, setLevelFilter] = useState<'all' | LogLevel>('all');
  const [query, setQuery] = useState('');
  const [leakChecking, setLeakChecking] = useState(false);
  const [leakCheckedAt, setLeakCheckedAt] = useState<string>('Not run yet');
  const [doctorChecks, setDoctorChecks] = useState<Array<{ id: string; name: string; passed: boolean; fix?: string | null }>>([]);

  const filteredLogs = useMemo(() => {
    const q = query.trim().toLowerCase();
    return logs.filter((entry) => {
      if (levelFilter !== 'all' && entry.level !== levelFilter) return false;
      if (!q) return true;
      return (
        entry.component.toLowerCase().includes(q) ||
        entry.message.toLowerCase().includes(q)
      );
    });
  }, [logs, levelFilter, query]);

  const handleRunLeakCheck = async () => {
    setLeakChecking(true);
    try {
      const res = await GhostApi.runDoctor();
      if (res && res.checks) {
        setDoctorChecks(res.checks);
      }
    } catch (e) {
      console.warn('Doctor run failed:', e);
    }
    setLeakChecking(false);
    const now = new Date();
    setLeakCheckedAt(now.toTimeString().split(' ')[0]);
  };

  const isConnected = status === 'connected';

  return (
    <div className="space-y-6">
      {/* DNS & Leak Verification Panel */}
      <section className="rounded-xl bg-[#1A1C20] border border-[#292D34] p-6">
        <div className="flex flex-wrap items-center justify-between gap-4 pb-4 border-b border-[#272A30]">
          <div>
            <h2 className="text-base font-semibold text-[#F1F3F5]">DNS & Leak Verification</h2>
            <p className="text-xs text-[#959CA6] mt-0.5">
              Inspect active outbound IPv4, WebRTC, and encrypted DNS resolver posture
            </p>
          </div>
          <button
            type="button"
            onClick={handleRunLeakCheck}
            disabled={leakChecking}
            className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-[#22252A] border border-[#30353D] text-xs font-medium text-[#F1F3F5] hover:border-[#22C55E]/60 hover:text-[#22C55E] transition-colors whitespace-nowrap"
          >
            <ShieldCheck className="w-3.5 h-3.5 text-[#22C55E]" />
            {leakChecking ? 'Probing Resolvers...' : 'Run Leak Check'}
          </button>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6 pt-4 text-xs">
          <div>
            <div className="text-[#7E8691]">Public Egress IP</div>
            <div className="font-mono tabular-nums text-[#F1F3F5] mt-1">
              {isConnected && activeNode ? activeNode.host : 'Not Connected'}
            </div>
          </div>
          <div>
            <div className="text-[#7E8691]">DNS Resolver</div>
            <div className="font-mono text-[#F1F3F5] mt-1 truncate">{dnsPrimary}</div>
          </div>
          <div>
            <div className="text-[#7E8691]">IPv6 Leak Protection</div>
            <div className="font-medium text-[#22C55E] mt-1">
              AAAA Suppressed
            </div>
          </div>
          <div>
            <div className="text-[#7E8691]">Last Verification</div>
            <div className="font-mono tabular-nums text-[#F1F3F5] mt-1">{leakCheckedAt}</div>
          </div>
        </div>

        {doctorChecks.length > 0 && (
          <div className="mt-4 pt-4 border-t border-[#272A30]">
            <div className="text-xs font-semibold text-[#F1F3F5] mb-2">Host Doctor Checks:</div>
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-2">
              {doctorChecks.map((chk) => (
                <div
                  key={chk.id}
                  className={`p-2 rounded border text-xs flex items-center justify-between ${
                    chk.passed
                      ? 'bg-[#17251C] border-[#22C55E]/30 text-[#4ADE80]'
                      : 'bg-[#1C1212] border-[#EF4444]/30 text-[#FCA5A5]'
                  }`}
                >
                  <span className="font-medium">{chk.name}</span>
                  <span className="font-mono text-[10px] uppercase font-bold">{chk.passed ? 'PASS' : 'FAIL'}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </section>

      {/* Core Kernel & Router Logs */}
      <section className="rounded-xl bg-[#1A1C20] border border-[#292D34] overflow-hidden">
        <div className="p-5 border-b border-[#272A30] flex flex-col lg:flex-row lg:items-center justify-between gap-4">
          <div className="flex flex-wrap items-center gap-3">
            <h3 className="text-base font-semibold text-[#F1F3F5]">Core Journal</h3>
            <div className="flex items-center gap-1 p-1 rounded-lg bg-[#131518] border border-[#272A30]">
              {(['all', 'info', 'warn', 'error'] as const).map((lvl) => (
                <button
                  key={lvl}
                  type="button"
                  onClick={() => setLevelFilter(lvl)}
                  className={`px-3 py-1 rounded-md text-xs font-medium uppercase transition-colors ${
                    levelFilter === lvl
                      ? 'bg-[#252930] text-[#22C55E]'
                      : 'text-[#959CA6] hover:text-[#F1F3F5]'
                  }`}
                >
                  {lvl}
                </button>
              ))}
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2.5">
            <div className="relative">
              <Search className="w-3.5 h-3.5 text-[#7E8691] absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
              <input
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Filter component or host..."
                aria-label="Filter log entries"
                className="w-52 pl-8 pr-3 py-1.5 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs text-[#F1F3F5] placeholder-[#606772] focus:outline-none focus:border-[#22C55E]"
              />
            </div>

            <button
              type="button"
              onClick={onTogglePause}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs text-[#F1F3F5] hover:border-[#22C55E]/60 transition-colors"
            >
              {isPaused ? <Play className="w-3.5 h-3.5 text-[#22C55E]" /> : <Pause className="w-3.5 h-3.5" />}
              {isPaused ? 'Resume' : 'Pause'}
            </button>

            <button
              type="button"
              onClick={onCopyLogs}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs text-[#F1F3F5] hover:border-[#22C55E]/60 transition-colors"
            >
              <Copy className="w-3.5 h-3.5" />
              Copy
            </button>

            <button
              type="button"
              onClick={onClearLogs}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs text-[#9EA5AE] hover:text-[#F87171] hover:border-[#F87171]/40 transition-colors"
            >
              <Trash2 className="w-3.5 h-3.5" />
              Clear
            </button>
          </div>
        </div>

        <div className="bg-[#101214] p-4 font-mono text-xs space-y-1.5 max-h-[420px] overflow-y-auto">
          {filteredLogs.length === 0 ? (
            <div className="py-10 text-center text-[#767E89]">
              No journal entries match the current filter.
            </div>
          ) : (
            filteredLogs.map((entry) => {
              const levelColor =
                entry.level === 'error'
                  ? 'text-[#F87171]'
                  : entry.level === 'warn'
                    ? 'text-[#FBBF24]'
                    : 'text-[#22C55E]';
              return (
                <div
                  key={entry.id}
                  className="grid grid-cols-12 gap-3 py-1 px-2 rounded hover:bg-[#1A1C20] leading-relaxed"
                >
                  <span className="col-span-3 sm:col-span-2 text-[#717883] tabular-nums">
                    {entry.timestamp}
                  </span>
                  <span className={`col-span-2 sm:col-span-1 uppercase font-semibold ${levelColor}`}>
                    {entry.level}
                  </span>
                  <span className="col-span-7 sm:col-span-2 text-[#98A0AA] truncate">
                    {entry.component}
                  </span>
                  <span className="col-span-12 sm:col-span-7 text-[#E1E4E8] break-all">
                    {entry.message}
                  </span>
                </div>
              );
            })
          )}
        </div>
      </section>
    </div>
  );
};
