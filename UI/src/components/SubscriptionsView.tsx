import React, { useState } from 'react';
import { Copy, Eye, EyeOff, Plus, RefreshCw, Trash2 } from 'lucide-react';
import { ServerNode, Subscription } from '../types/vpn';

interface SubscriptionsViewProps {
  subscriptions: Subscription[];
  nodes: ServerNode[];
  onRefreshSubscription: (id: string) => void;
  onRefreshAll: () => void;
  onDeleteSubscription: (id: string) => void;
  onOpenImportModal: () => void;
  onCopyText: (label: string, text: string) => void;
}

export const SubscriptionsView: React.FC<SubscriptionsViewProps> = ({
  subscriptions,
  nodes,
  onRefreshSubscription,
  onRefreshAll,
  onDeleteSubscription,
  onOpenImportModal,
  onCopyText,
}) => {
  const [revealedIds, setRevealedIds] = useState<Record<string, boolean>>({});

  const toggleReveal = (id: string) => {
    setRevealedIds((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  return (
    <div className="space-y-6">
      {/* Header Toolbar */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold text-[#F1F3F5]">Subscriptions</h2>
          <p className="text-xs text-[#959CA6] mt-0.5">
            Remote YAML / JSON / Base64 node feeds synchronized automatically
          </p>
        </div>
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={onRefreshAll}
            className="inline-flex items-center gap-2 px-3.5 py-2 rounded-lg bg-[#1F2227] border border-[#2A2E35] text-xs font-medium text-[#F1F3F5] hover:border-[#22C55E]/60 transition-colors whitespace-nowrap"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            Sync All
          </button>
          <button
            type="button"
            onClick={onOpenImportModal}
            className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-[#22C55E] text-[#07140B] text-xs font-semibold hover:bg-[#16A34A] transition-colors whitespace-nowrap"
          >
            <Plus className="w-3.5 h-3.5" />
            Add Subscription
          </button>
        </div>
      </div>

      {subscriptions.length === 0 ? (
        <div className="rounded-xl bg-[#1A1C20] border border-[#292D34] p-12 text-center">
          <p className="text-sm font-medium text-[#F1F3F5]">No active subscriptions</p>
          <p className="text-xs text-[#959CA6] mt-1 mb-4">
            Import a subscription URL or share link to populate server nodes.
          </p>
          <button
            type="button"
            onClick={onOpenImportModal}
            className="px-4 py-2 rounded-lg bg-[#22C55E] text-[#07140B] text-xs font-semibold hover:bg-[#16A34A] transition-colors"
          >
            Import First Profile
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {subscriptions.map((sub) => {
            const subNodes = nodes.filter((n) => n.subscriptionId === sub.id);
            const usagePercent = Math.min(100, Math.round((sub.usedGb / sub.totalGb) * 100));
            const isRevealed = !!revealedIds[sub.id];

            return (
              <section
                key={sub.id}
                className="rounded-xl bg-[#1A1C20] border border-[#292D34] p-6 flex flex-col justify-between space-y-5"
              >
                <div className="flex items-start justify-between gap-4">
                  <div className="min-w-0">
                    <h3 className="text-base font-semibold text-[#F1F3F5] truncate">{sub.name}</h3>
                    <p className="text-xs text-[#959CA6] mt-1">
                      {subNodes.length} endpoints · Updated {sub.lastUpdated} · Auto-sync {sub.autoUpdateHours}h
                    </p>
                  </div>

                  <div className="flex items-center gap-1 shrink-0">
                    <button
                      type="button"
                      onClick={() => onRefreshSubscription(sub.id)}
                      disabled={sub.updating}
                      aria-label={`Sync ${sub.name}`}
                      className="p-2 rounded-lg text-[#959CA6] hover:text-[#22C55E] hover:bg-[#24272D] transition-colors"
                    >
                      <RefreshCw className={`w-4 h-4 ${sub.updating ? 'animate-spin text-[#22C55E]' : ''}`} />
                    </button>
                    <button
                      type="button"
                      onClick={() => onDeleteSubscription(sub.id)}
                      aria-label={`Delete ${sub.name}`}
                      className="p-2 rounded-lg text-[#959CA6] hover:text-[#F87171] hover:bg-[#1F1717] transition-colors"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>

                {/* Endpoint URL Bar */}
                <div className="flex items-center justify-between gap-2 px-3 py-2 rounded-lg bg-[#131518] border border-[#272A30]">
                  <span className="text-xs font-mono text-[#9EA5AE] truncate">
                    {isRevealed ? sub.url : `${sub.url.slice(0, 32)}••••••••••••••••••••`}
                  </span>
                  <div className="flex items-center gap-1 shrink-0">
                    <button
                      type="button"
                      onClick={() => toggleReveal(sub.id)}
                      aria-label={isRevealed ? 'Mask subscription URL' : 'Reveal subscription URL'}
                      className="p-1.5 rounded text-[#7E8691] hover:text-[#F1F3F5] transition-colors"
                    >
                      {isRevealed ? <EyeOff className="w-3.5 h-3.5" /> : <Eye className="w-3.5 h-3.5" />}
                    </button>
                    <button
                      type="button"
                      onClick={() => onCopyText(sub.name, sub.url)}
                      aria-label="Copy subscription URL"
                      className="p-1.5 rounded text-[#7E8691] hover:text-[#22C55E] transition-colors"
                    >
                      <Copy className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>

                {/* Bandwidth Quota */}
                <div className="space-y-2">
                  <div className="flex items-center justify-between text-xs font-mono tabular-nums">
                    <span className="text-[#959CA6]">
                      Quota: {sub.usedGb.toFixed(1)} GB / {sub.totalGb} GB
                    </span>
                    <span className="text-[#F1F3F5]">Expires {sub.expiresAt}</span>
                  </div>
                  <div className="w-full h-1.5 rounded-full bg-[#262A30] overflow-hidden">
                    <div
                      className="h-full rounded-full bg-[#22C55E] transition-all duration-300"
                      style={{ width: `${usagePercent}%` }}
                    />
                  </div>
                </div>
              </section>
            );
          })}
        </div>
      )}
    </div>
  );
};
