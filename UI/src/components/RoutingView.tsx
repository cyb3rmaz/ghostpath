import React, { useEffect, useMemo, useState } from 'react';
import {
  ChevronDown,
  Clock,
  Globe,
  Loader2,
  MapPin,
  Plus,
  RotateCcw,
  ShieldAlert,
  Trash2,
} from 'lucide-react';
import {
  ConnectionStatus,
  GeoCountryPreset,
  GeoSpoofSettings,
  GeoSpoofStatus,
  RoutingRule,
  RuleAction,
  ServerNode,
} from '../types/vpn';
import { INITIAL_GEO_SPOOF_SETTINGS } from '../data/vpnData';

interface RoutingViewProps {
  rules: RoutingRule[];
  onToggleRule: (id: string) => void;
  onChangeRuleAction: (id: string, action: RuleAction) => void;
  onDeleteRule: (id: string) => void;
  onAddRule: (rule: Omit<RoutingRule, 'id'>) => void;
  geoCountries: GeoCountryPreset[];
  geoStatus: GeoSpoofStatus | null;
  geoBusy?: boolean;
  connectionStatus: ConnectionStatus;
  activeNode: ServerNode | null;
  onUpdateGeoSpoof: (patch: Partial<GeoSpoofSettings>) => void;
  onApplyGeoSpoof: (countryCode?: string) => void;
  onRestoreGeoSpoof: () => void;
}

const formatClockForZone = (tz: string, tick: Date): string => {
  try {
    return new Intl.DateTimeFormat('en-GB', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      day: '2-digit',
      month: 'short',
      timeZone: tz,
    }).format(tick);
  } catch {
    return 'Unavailable';
  }
};

export const RoutingView: React.FC<RoutingViewProps> = ({
  rules,
  onToggleRule,
  onChangeRuleAction,
  onDeleteRule,
  onAddRule,
  geoCountries,
  geoStatus,
  geoBusy = false,
  connectionStatus,
  activeNode,
  onUpdateGeoSpoof,
  onApplyGeoSpoof,
  onRestoreGeoSpoof,
}) => {
  const [newType, setNewType] = useState<RoutingRule['type']>('domain');
  const [newPattern, setNewPattern] = useState('');
  const [newAction, setNewAction] = useState<RuleAction>('proxy');
  const [newNote, setNewNote] = useState('');
  const [filterType, setFilterType] = useState<'all' | RoutingRule['type']>('all');
  const [clockNow, setClockNow] = useState(() => new Date());

  useEffect(() => {
    const id = setInterval(() => setClockNow(new Date()), 1000);
    return () => clearInterval(id);
  }, []);

  const geoSettings = geoStatus?.settings ?? INITIAL_GEO_SPOOF_SETTINGS;

  const selectedPreset = useMemo(
    () =>
      geoCountries.find(
        (c) => c.countryCode.toUpperCase() === geoSettings.countryCode.toUpperCase()
      ) ?? null,
    [geoCountries, geoSettings.countryCode]
  );

  const matchedNodePreset = useMemo(() => {
    if (!activeNode) return null;
    const code = activeNode.countryCode?.toUpperCase();
    return (
      geoCountries.find((c) => c.countryCode.toUpperCase() === code) ??
      geoCountries.find(
        (c) => c.countryName.toLowerCase() === activeNode.country?.toLowerCase()
      ) ??
      null
    );
  }, [geoCountries, activeNode]);

  const effectiveTargetPreset =
    geoSettings.mode === 'match-node'
      ? matchedNodePreset ?? selectedPreset
      : selectedPreset;

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
    <div className="space-y-8">
      {/* Geo Location & System Timezone Spoofing */}
      <section className="rounded-xl bg-[#101512] border border-[#1D2721] p-6 space-y-5">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="space-y-1">
            <div className="flex flex-wrap items-center gap-2.5">
              <h2 className="text-lg font-semibold text-[#ECF2EE] flex items-center gap-2">
                <Globe className="w-4 h-4 text-[#22C55E]" />
                Geo Location &amp; System Timezone Spoofing
              </h2>
              {geoStatus ? (
                geoStatus.spoofActive ? (
                  <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[11px] font-medium bg-[#22C55E]/15 text-[#22C55E] border border-[#22C55E]/30">
                    Spoofing Active: {geoStatus.appliedCountryName || geoStatus.appliedCountryCode} (
                    {geoStatus.activeTimezone})
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[11px] font-medium bg-[#18201B] text-[#98A69E] border border-[#243029]">
                    System Default: {geoStatus.activeTimezone}
                  </span>
                )
              ) : (
                <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[11px] font-medium bg-[#1F1A14] text-[#FBBF24] border border-[#F59E0B]/30">
                  Daemon status unavailable
                </span>
              )}
            </div>
            <p className="text-xs text-[#8A9990]">
              Select a country to align Linux OS timezone (/etc/localtime) and desktop GeoClue2 coordinates with your tunnel exit
            </p>
          </div>

          <div className="flex items-center gap-3">
            <span className="text-xs text-[#8A9990]">
              {geoSettings.enabled ? 'Enabled' : 'Disabled'}
            </span>
            <button
              type="button"
              role="switch"
              aria-checked={geoSettings.enabled}
              aria-label="Enable Geo Location and System Timezone Spoofing"
              disabled={geoBusy}
              onClick={() => onUpdateGeoSpoof({ enabled: !geoSettings.enabled })}
              className={`relative w-10 h-5 rounded-full transition-colors shrink-0 ${
                geoSettings.enabled ? 'bg-[#22C55E]' : 'bg-[#243029]'
              } disabled:opacity-60`}
            >
              <span
                className={`block w-3.5 h-3.5 rounded-full bg-[#090C0A] transition-transform duration-150 ${
                  geoSettings.enabled ? 'translate-x-5.5' : 'translate-x-1'
                }`}
              />
            </button>
          </div>
        </div>

        {/* Mode Selector & Country Picker */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-4 items-end">
          <div className="lg:col-span-4">
            <label className="block text-xs text-[#8A9990] mb-1.5">Target Source</label>
            <div className="grid grid-cols-2 gap-1 p-1 rounded-lg bg-[#0A0D0B] border border-[#1E2822]">
              <button
                type="button"
                disabled={geoBusy}
                onClick={() => onUpdateGeoSpoof({ mode: 'match-node' })}
                className={`py-1.5 px-2.5 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
                  geoSettings.mode === 'match-node'
                    ? 'bg-[#19241E] text-[#22C55E]'
                    : 'text-[#8A9990] hover:text-[#ECF2EE]'
                }`}
              >
                Match VPN Server
              </button>
              <button
                type="button"
                disabled={geoBusy}
                onClick={() => onUpdateGeoSpoof({ mode: 'manual' })}
                className={`py-1.5 px-2.5 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
                  geoSettings.mode === 'manual'
                    ? 'bg-[#19241E] text-[#22C55E]'
                    : 'text-[#8A9990] hover:text-[#ECF2EE]'
                }`}
              >
                Choose Country
              </button>
            </div>
          </div>

          <div className="lg:col-span-5">
            <label className="block text-xs text-[#8A9990] mb-1.5" htmlFor="geo-country-select">
              {geoSettings.mode === 'match-node'
                ? 'Fallback Country (when node country is unknown)'
                : 'Spoof Target Country'}
            </label>
            <div className="relative">
              <select
                id="geo-country-select"
                value={geoSettings.countryCode}
                disabled={geoBusy || geoCountries.length === 0}
                onChange={(e) => onUpdateGeoSpoof({ countryCode: e.target.value })}
                className="w-full appearance-none pl-3 pr-9 py-2 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs text-[#ECF2EE] focus:outline-none focus:border-[#22C55E] disabled:opacity-60"
              >
                {geoCountries.length === 0 ? (
                  <option value={geoSettings.countryCode}>
                    Country catalog unavailable (connect to daemon)
                  </option>
                ) : (
                  geoCountries.map((c) => (
                    <option key={c.countryCode} value={c.countryCode}>
                      {c.countryName} ({c.countryCode}) — {c.city} · {c.timezone} ({c.utcOffsetHint})
                    </option>
                  ))
                )}
              </select>
              <ChevronDown className="w-3.5 h-3.5 text-[#8A9990] absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none" />
            </div>
          </div>

          <div className="lg:col-span-3 flex items-center gap-2">
            <button
              type="button"
              disabled={geoBusy || geoCountries.length === 0}
              onClick={() =>
                onApplyGeoSpoof(
                  geoSettings.mode === 'manual' ? geoSettings.countryCode : undefined
                )
              }
              className="flex-1 inline-flex items-center justify-center gap-1.5 py-2 px-3 rounded-lg bg-[#22C55E] text-[#05140A] text-xs font-semibold hover:bg-[#16A34A] transition-colors whitespace-nowrap disabled:opacity-60"
            >
              {geoBusy ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <MapPin className="w-3.5 h-3.5" />
              )}
              Apply Now
            </button>
            <button
              type="button"
              disabled={geoBusy || !geoStatus?.spoofActive}
              onClick={onRestoreGeoSpoof}
              title="Restore original OS timezone & location"
              className="inline-flex items-center justify-center gap-1.5 py-2 px-3 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs font-medium text-[#D2DDD6] hover:text-[#ECF2EE] hover:border-[#22C55E]/40 transition-colors whitespace-nowrap disabled:opacity-45"
            >
              <RotateCcw className="w-3.5 h-3.5" />
              Restore
            </button>
          </div>
        </div>

        {/* Matched Node Helper Banner */}
        {geoSettings.mode === 'match-node' && (
          <div className="px-3.5 py-2.5 rounded-lg bg-[#0A0D0B] border border-[#1A231E] text-xs text-[#98A69E] flex flex-wrap items-center justify-between gap-2">
            <span>
              {activeNode ? (
                matchedNodePreset ? (
                  <>
                    Active node <span className="font-mono text-[#ECF2EE]">{activeNode.name}</span>{' '}
                    maps to{' '}
                    <span className="text-[#22C55E] font-medium">
                      {matchedNodePreset.countryName} ({matchedNodePreset.timezone})
                    </span>
                  </>
                ) : (
                  <>
                    Active node <span className="font-mono text-[#ECF2EE]">{activeNode.name}</span>{' '}
                    has no country metadata; will use fallback{' '}
                    <span className="text-[#ECF2EE] font-medium">
                      {selectedPreset?.countryName || geoSettings.countryCode}
                    </span>
                  </>
                )
              ) : (
                'No active server node selected; will match on connect or use fallback country.'
              )}
            </span>
            <span className="text-[11px] font-mono text-[#6E7E75]">
              Tunnel: {connectionStatus}
            </span>
          </div>
        )}

        {/* Component Toggles */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
          {[
            {
              key: 'spoofTimezone' as const,
              title: 'System Timezone',
              desc: 'Updates /etc/localtime via timedatectl',
              checked: geoSettings.spoofTimezone,
            },
            {
              key: 'spoofGeolocation' as const,
              title: 'GeoClue2 Coordinates',
              desc: 'Sets /etc/geolocation static GPS',
              checked: geoSettings.spoofGeolocation,
            },
            {
              key: 'applyOnConnect' as const,
              title: 'Auto-Restore on Disconnect',
              desc: 'Reverts OS state when tunnel closes',
              checked: geoSettings.applyOnConnect,
            },
            {
              key: 'strictGeoRouting' as const,
              title: 'Strict Geo Routing',
              desc: 'Proxy non-LAN GeoIP/GeoSite traffic',
              checked: geoSettings.strictGeoRouting,
            },
          ].map((opt) => (
            <button
              key={opt.key}
              type="button"
              role="switch"
              aria-checked={opt.checked}
              disabled={geoBusy}
              onClick={() => onUpdateGeoSpoof({ [opt.key]: !opt.checked })}
              className="flex items-start justify-between gap-3 p-3.5 rounded-lg bg-[#0A0D0B] border border-[#1B241F] hover:border-[#26352C] text-left transition-colors"
            >
              <div className="min-w-0">
                <div className="text-xs font-medium text-[#ECF2EE]">{opt.title}</div>
                <div className="text-[11px] text-[#78887F] mt-0.5 leading-snug">{opt.desc}</div>
              </div>
              <span
                className={`relative w-8 h-4 rounded-full transition-colors shrink-0 mt-0.5 ${
                  opt.checked ? 'bg-[#22C55E]' : 'bg-[#243029]'
                }`}
              >
                <span
                  className={`block w-3 h-3 rounded-full bg-[#090C0A] transition-transform duration-150 translate-y-0.5 ${
                    opt.checked ? 'translate-x-4.5' : 'translate-x-0.5'
                  }`}
                />
              </span>
            </button>
          ))}
        </div>

        {/* Real Live OS Telemetry Readout */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 pt-1">
          <div className="rounded-lg bg-[#0A0D0B] border border-[#1A231E] px-3.5 py-2.5">
            <div className="flex items-center gap-1.5 text-[11px] text-[#78887F]">
              <Clock className="w-3 h-3" />
              Active OS Timezone
            </div>
            <div className="text-xs font-mono text-[#ECF2EE] mt-1 truncate">
              {geoStatus ? geoStatus.activeTimezone : 'Unavailable'}
            </div>
            <div className="text-[11px] font-mono text-[#22C55E] mt-0.5">
              {geoStatus
                ? formatClockForZone(geoStatus.activeTimezone, clockNow)
                : effectiveTargetPreset
                  ? `${effectiveTargetPreset.timezone} (${effectiveTargetPreset.utcOffsetHint})`
                  : '—'}
            </div>
          </div>

          <div className="rounded-lg bg-[#0A0D0B] border border-[#1A231E] px-3.5 py-2.5">
            <div className="flex items-center gap-1.5 text-[11px] text-[#78887F]">
              <MapPin className="w-3 h-3" />
              GeoClue Coordinates
            </div>
            <div className="text-xs font-mono text-[#ECF2EE] mt-1 truncate">
              {geoStatus?.activeGeolocation
                ? `${geoStatus.activeGeolocation.latitude.toFixed(4)}, ${geoStatus.activeGeolocation.longitude.toFixed(4)}`
                : 'System default (not overridden)'}
            </div>
            <div className="text-[11px] text-[#78887F] mt-0.5 truncate">
              {effectiveTargetPreset
                ? `Target: ${effectiveTargetPreset.city}, ${effectiveTargetPreset.countryCode}`
                : 'No target selected'}
            </div>
          </div>

          <div className="rounded-lg bg-[#0A0D0B] border border-[#1A231E] px-3.5 py-2.5">
            <div className="flex items-center gap-1.5 text-[11px] text-[#78887F]">
              <RotateCcw className="w-3 h-3" />
              Original Backup
            </div>
            <div className="text-xs font-mono text-[#ECF2EE] mt-1 truncate">
              {geoStatus?.backedUpTimezone || 'None saved yet'}
            </div>
            <div className="text-[11px] text-[#78887F] mt-0.5">
              Protected by UndoJournal
            </div>
          </div>

          <div className="rounded-lg bg-[#0A0D0B] border border-[#1A231E] px-3.5 py-2.5">
            <div className="flex items-center gap-1.5 text-[11px] text-[#78887F]">
              <ShieldAlert className="w-3 h-3" />
              Linux Subsystems
            </div>
            <div className="text-xs text-[#ECF2EE] mt-1 truncate">
              {geoStatus
                ? geoStatus.timedatectlAvailable
                  ? 'timedatectl ready'
                  : '/etc/localtime fallback'
                : 'Unavailable'}
            </div>
            <div className="text-[11px] text-[#78887F] mt-0.5 truncate">
              {geoStatus
                ? geoStatus.geoclueAvailable
                  ? 'GeoClue2 ready'
                  : 'GeoClue2 absent (TZ only)'
                : 'Start ghostpathd service'}
            </div>
          </div>
        </div>
      </section>

      {/* Split Tunneling & Rules */}
      <div className="space-y-6">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <h2 className="text-lg font-semibold text-[#ECF2EE]">Split Tunneling &amp; Rules</h2>
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
            <div className="relative">
              <select
                id="rule-type"
                value={newType}
                onChange={(e) => setNewType(e.target.value as RoutingRule['type'])}
                className="w-full appearance-none pl-3 pr-8 py-2 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs text-[#ECF2EE] focus:outline-none focus:border-[#22C55E]"
              >
                <option value="domain">Domain</option>
                <option value="process">Linux Binary</option>
                <option value="cidr">IP / CIDR</option>
                <option value="geosite">GeoSite</option>
                <option value="geoip">GeoIP</option>
              </select>
              <ChevronDown className="w-3.5 h-3.5 text-[#8A9990] absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
            </div>
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
            <div className="relative">
              <select
                id="rule-action"
                value={newAction}
                onChange={(e) => setNewAction(e.target.value as RuleAction)}
                className="w-full appearance-none pl-3 pr-8 py-2 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs text-[#ECF2EE] focus:outline-none focus:border-[#22C55E]"
              >
                <option value="proxy">Proxy</option>
                <option value="direct">Direct</option>
                <option value="block">Block</option>
              </select>
              <ChevronDown className="w-3.5 h-3.5 text-[#8A9990] absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
            </div>
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
          {visibleRules.length === 0 ? (
            <div className="px-5 py-8 text-center text-xs text-[#78887F]">
              No routing rules match this filter.
            </div>
          ) : (
            visibleRules.map((rule) => (
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
                      if (active && action === 'proxy')
                        activeClass = 'bg-[#22C55E]/20 text-[#22C55E]';
                      if (active && action === 'direct')
                        activeClass = 'bg-[#1D2822] text-[#D1DCD5]';
                      if (active && action === 'block')
                        activeClass = 'bg-[#F87171]/15 text-[#F87171]';
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
            ))
          )}
        </section>
      </div>
    </div>
  );
};
