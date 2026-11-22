import React, { useState } from 'react';
import { Plus, Trash2 } from 'lucide-react';
import { RoutingRule, RuleAction } from '../types/vpn';

interface RoutingViewProps {
  rules: RoutingRule[];
  onToggleRule: (id: string) => void;
  onChangeRuleAction: (id: string, action: RuleAction) => void;
  onDeleteRule: (id: string) => void;
  onAddRule: (rule: Omit<RoutingRule, 'id'>) => void;
}

export const RoutingView: React.FC<RoutingViewProps> = ({
  rules,
  onToggleRule,
  onChangeRuleAction,
  onDeleteRule,
  onAddRule,
}) => {
  const [newType, setNewType] = useState<RoutingRule['type']>('domain');
  const [newPattern, setNewPattern] = useState('');
  const [newAction, setNewAction] = useState<RuleAction>('proxy');
  const [newNote, setNewNote] = useState('');
  const [filterType, setFilterType] = useState<'all' | RoutingRule['type']>('all');

  const handleCreate = (e: React.FormEvent) => {
    e.preventDefault();
    const pattern = newPattern.trim();
    if (!pattern) return;
    onAddRule({
      type: newType,
      pattern,
      action: newAction,
      enabled: true,
      note: newNote.trim() || `Custom ${newType} routing rule`,
    });
    setNewPattern('');
    setNewNote('');
  };

  const visibleRules = rules.filter((r) => filterType === 'all' || r.type === filterType);

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold text-[#ECF2EE]">Split Tunneling & Rules</h2>
          <p className="text-xs text-[#8A9990] mt-0.5">
            Evaluate Linux process paths, domains, and CIDR subnets in top-down priority order
          </p>
        </div>

        <div className="flex items-center gap-1 p-1 rounded-lg bg-[#101512] border border-[#1D2721]">
          {(['all', 'process', 'domain', 'geosite', 'geoip', 'cidr'] as const).map((t) => (
            <button
              key={t}
              type="button"
              onClick={() => setFilterType(t)}
              className={`px-3 py-1 rounded-md text-xs font-medium capitalize transition-colors whitespace-nowrap ${
                filterType === t
                  ? 'bg-[#19241E] text-[#22C55E]'
                  : 'text-[#8A9990] hover:text-[#ECF2EE]'
              }`}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      {/* Add Rule Bar */}
      <form
        onSubmit={handleCreate}
        className="rounded-xl bg-[#101512] border border-[#1D2721] p-5 grid grid-cols-1 md:grid-cols-12 gap-3 items-end"
      >
        <div className="md:col-span-2">
          <label className="block text-xs text-[#8A9990] mb-1.5" htmlFor="rule-type">
            Match Type
          </label>
          <select
            id="rule-type"
            value={newType}
            onChange={(e) => setNewType(e.target.value as RoutingRule['type'])}
            className="w-full px-3 py-2 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs text-[#ECF2EE] focus:outline-none focus:border-[#22C55E]"
          >
            <option value="domain">Domain</option>
            <option value="process">Linux Binary</option>
            <option value="cidr">IP / CIDR</option>
            <option value="geosite">GeoSite</option>
            <option value="geoip">GeoIP</option>
          </select>
        </div>

        <div className="md:col-span-4">
          <label className="block text-xs text-[#8A9990] mb-1.5" htmlFor="rule-pattern">
            Pattern / Binary Path
          </label>
          <input
            id="rule-pattern"
            type="text"
            value={newPattern}
            onChange={(e) => setNewPattern(e.target.value)}
            placeholder={
              newType === 'process'
                ? '/usr/bin/code'
                : newType === 'cidr'
                  ? '192.168.10.0/24'
                  : 'domain:kernel.org'
            }
            className="w-full px-3 py-2 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs font-mono text-[#ECF2EE] placeholder-[#5A6A61] focus:outline-none focus:border-[#22C55E]"
          />
        </div>

        <div className="md:col-span-3">
          <label className="block text-xs text-[#8A9990] mb-1.5" htmlFor="rule-note">
            Description
          </label>
          <input
            id="rule-note"
            type="text"
            value={newNote}
            onChange={(e) => setNewNote(e.target.value)}
            placeholder="Optional note"
            className="w-full px-3 py-2 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs text-[#ECF2EE] placeholder-[#5A6A61] focus:outline-none focus:border-[#22C55E]"
          />
        </div>

        <div className="md:col-span-2">
          <label className="block text-xs text-[#8A9990] mb-1.5" htmlFor="rule-action">
            Action
          </label>
          <select
            id="rule-action"
            value={newAction}
            onChange={(e) => setNewAction(e.target.value as RuleAction)}
            className="w-full px-3 py-2 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs text-[#ECF2EE] focus:outline-none focus:border-[#22C55E]"
          >
            <option value="proxy">Proxy</option>
            <option value="direct">Direct</option>
            <option value="block">Block</option>
          </select>
        </div>

        <div className="md:col-span-1">
          <button
            type="submit"
            className="w-full inline-flex items-center justify-center gap-1.5 py-2 px-3 rounded-lg bg-[#22C55E] text-[#05140A] text-xs font-semibold hover:bg-[#16A34A] transition-colors whitespace-nowrap"
          >
            <Plus className="w-3.5 h-3.5" />
            Add
          </button>
        </div>
      </form>

      {/* Rules List */}
      <section className="rounded-xl bg-[#101512] border border-[#1D2721] divide-y divide-[#17201B] overflow-hidden">
        {visibleRules.map((rule) => (
          <div
            key={rule.id}
            className={`flex flex-col sm:flex-row sm:items-center justify-between gap-4 px-5 py-3.5 transition-colors ${
              rule.enabled ? 'hover:bg-[#141B17]' : 'opacity-55 bg-[#0C100E]'
            }`}
          >
            <div className="flex items-start sm:items-center gap-4 min-w-0">
              <button
                type="button"
                role="switch"
                aria-checked={rule.enabled}
                aria-label={`Toggle rule ${rule.pattern}`}
                onClick={() => onToggleRule(rule.id)}
                className={`relative w-9 h-5 rounded-full transition-colors shrink-0 mt-0.5 sm:mt-0 ${
                  rule.enabled ? 'bg-[#22C55E]' : 'bg-[#243029]'
                }`}
              >
                <span
                  className={`block w-3.5 h-3.5 rounded-full bg-[#090C0A] transition-transform duration-150 ${
                    rule.enabled ? 'translate-x-4.5' : 'translate-x-1'
                  }`}
                />
              </button>

              <div className="min-w-0">
                <div className="flex items-center gap-2.5">
                  <span className="text-xs font-mono text-[#ECF2EE] truncate">
                    {rule.pattern}
                  </span>
                  <span className="text-xs text-[#6E7E75] uppercase shrink-0">
                    {rule.type}
                  </span>
                </div>
                <p className="text-xs text-[#8A9990] mt-0.5 truncate">{rule.note}</p>
              </div>
            </div>

            <div className="flex items-center gap-3 shrink-0 self-end sm:self-center">
              <div className="grid grid-cols-3 gap-1 p-1 rounded-lg bg-[#0A0D0B] border border-[#1B241F]">
                {(['proxy', 'direct', 'block'] as RuleAction[]).map((action) => {
                  const active = rule.action === action;
                  let activeClass = 'bg-[#1A241E] text-[#ECF2EE]';
                  if (active && action === 'proxy') activeClass = 'bg-[#22C55E]/20 text-[#22C55E]';
                  if (active && action === 'direct') activeClass = 'bg-[#1D2822] text-[#D1DCD5]';
                  if (active && action === 'block') activeClass = 'bg-[#F87171]/15 text-[#F87171]';
                  return (
                    <button
                      key={action}
                      type="button"
                      onClick={() => onChangeRuleAction(rule.id, action)}
                      className={`px-2.5 py-1 rounded text-xs font-medium capitalize transition-colors ${
                        active ? activeClass : 'text-[#78887F] hover:text-[#ECF2EE]'
                      }`}
                    >
                      {action}
                    </button>
                  );
                })}
              </div>

              <button
                type="button"
                onClick={() => onDeleteRule(rule.id)}
                aria-label={`Remove rule ${rule.pattern}`}
                className="p-2 rounded-lg text-[#78887F] hover:text-[#F87171] hover:bg-[#1F1717] transition-colors"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            </div>
          </div>
        ))}
      </section>
    </div>
  );
};
