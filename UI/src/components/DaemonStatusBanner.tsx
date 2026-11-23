import React, { useState } from 'react';
import { AlertTriangle, AlertCircle, CheckCircle2, Copy, RefreshCw } from 'lucide-react';

export interface DaemonHealth {
  state: 'running' | 'not_running' | 'not_installed' | 'permission_denied' | 'version_mismatch';
  message: string;
  fix_command?: string | null;
  details?: string | null;
}

interface DaemonStatusBannerProps {
  health: DaemonHealth | null;
  onRefresh: () => void;
}

export const DaemonStatusBanner: React.FC<DaemonStatusBannerProps> = ({ health, onRefresh }) => {
  const [copied, setCopied] = useState(false);

  if (!health || health.state === 'running') {
    return null;
  }

  const isSevere = health.state === 'not_installed' || health.state === 'permission_denied';

  const handleCopy = (text: string) => {
    navigator.clipboard?.writeText(text).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div
      className={`mx-4 mt-3 mb-1 p-3.5 rounded-lg border text-sm flex flex-col md:flex-row md:items-center justify-between gap-3 ${
        isSevere
          ? 'bg-[#1C0D0D] border-[#7F1D1D]/60 text-[#FECACA]'
          : 'bg-[#1C160C] border-[#B45309]/50 text-[#FDE68A]'
      }`}
    >
      <div className="flex items-start gap-3">
        {isSevere ? (
          <AlertCircle className="w-5 h-5 text-[#EF4444] shrink-0 mt-0.5" />
        ) : (
          <AlertTriangle className="w-5 h-5 text-[#F59E0B] shrink-0 mt-0.5" />
        )}
        <div>
          <div className="font-semibold">{health.message}</div>
          {health.details && <div className="text-xs opacity-85 mt-0.5">{health.details}</div>}
          {health.fix_command && (
            <div className="mt-2 flex items-center gap-2">
              <code className="text-xs bg-black/40 px-2 py-1 rounded border border-white/10 font-mono text-[#4ADE80]">
                {health.fix_command}
              </code>
              <button
                type="button"
                onClick={() => handleCopy(health.fix_command!)}
                className="text-xs p-1 rounded hover:bg-white/10 text-white/70 hover:text-white transition-colors"
                title="Copy command"
              >
                {copied ? <CheckCircle2 className="w-3.5 h-3.5 text-[#22C55E]" /> : <Copy className="w-3.5 h-3.5" />}
              </button>
            </div>
          )}
        </div>
      </div>

      <div className="flex items-center gap-2 shrink-0 self-end md:self-center">
        <button
          type="button"
          onClick={onRefresh}
          className="flex items-center gap-1.5 text-xs font-medium px-3 py-1.5 rounded bg-white/10 hover:bg-white/15 text-white transition-colors"
        >
          <RefreshCw className="w-3.5 h-3.5" />
          <span>Retry Status</span>
        </button>
      </div>
    </div>
  );
};
