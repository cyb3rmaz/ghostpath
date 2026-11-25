import React from 'react';
import { ChevronDown } from 'lucide-react';
import { AppSettings, CoreEngine } from '../types/vpn';

interface SettingsViewProps {
  settings: AppSettings;
  onUpdateSettings: (patch: Partial<AppSettings>) => void;
  onResetDefaults: () => void;
}

export const SettingsView: React.FC<SettingsViewProps> = ({
  settings,
  onUpdateSettings,
  onResetDefaults,
}) => {
  return (
    <div className="space-y-6 max-w-4xl">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold text-[#F1F3F5]">Linux Client Preferences</h2>
          <p className="text-xs text-[#959CA6] mt-0.5">
            Configure kernel TUN parameters, systemd daemon behavior, and upstream resolvers
          </p>
        </div>

        <button
          type="button"
          onClick={onResetDefaults}
          className="px-3.5 py-2 rounded-lg bg-[#1A1C20] border border-[#292D34] text-xs font-medium text-[#9EA5AE] hover:text-[#F1F3F5] hover:border-[#22C55E]/50 transition-colors"
        >
          Reset Defaults
        </button>
      </div>

      {/* Core Engine & Daemon */}
      <section className="rounded-xl bg-[#1A1C20] border border-[#292D34] divide-y divide-[#272A30]">
        <div className="p-5 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div>
            <div className="text-sm font-medium text-[#F1F3F5]">Routing Core Engine</div>
            <div className="text-xs text-[#959CA6] mt-0.5">
              Select underlying proxy runtime binary managed by GhostPath VPN daemon
            </div>
          </div>
          <div className="grid grid-cols-2 gap-1 p-1 rounded-lg bg-[#131518] border border-[#272A30] shrink-0">
            {(['sing-box', 'xray-core'] as CoreEngine[]).map((engine) => (
              <button
                key={engine}
                type="button"
                onClick={() => onUpdateSettings({ coreEngine: engine })}
                className={`px-3.5 py-1.5 rounded-md text-xs font-mono transition-colors ${
                  settings.coreEngine === engine
                    ? 'bg-[#22C55E] text-[#07140B] font-semibold'
                    : 'text-[#959CA6] hover:text-[#F1F3F5]'
                }`}
              >
                {engine}
              </button>
            ))}
          </div>
        </div>

        <div className="p-5 flex items-center justify-between gap-4">
          <div>
            <div className="text-sm font-medium text-[#F1F3F5]">Start systemd User Service on Login</div>
            <div className="text-xs text-[#959CA6] mt-0.5">
              Automatically restore last active tunnel profile via ghostpath.service
            </div>
          </div>
          <button
            type="button"
            role="switch"
            aria-checked={settings.launchOnBoot}
            onClick={() => onUpdateSettings({ launchOnBoot: !settings.launchOnBoot })}
            className={`relative w-10 h-5 rounded-full transition-colors shrink-0 ${
              settings.launchOnBoot ? 'bg-[#22C55E]' : 'bg-[#30353D]'
            }`}
          >
            <span
              className={`block w-3.5 h-3.5 rounded-full bg-[#111315] transition-transform duration-150 ${
                settings.launchOnBoot ? 'translate-x-5.5' : 'translate-x-1'
              }`}
            />
          </button>
        </div>

        <div className="p-5 flex items-center justify-between gap-4">
          <div>
            <div className="text-sm font-medium text-[#F1F3F5]">nftables Kill Switch</div>
            <div className="text-xs text-[#959CA6] mt-0.5">
              Drop non-tunnel outbound packets if ghost0 drops unexpectedly
            </div>
          </div>
          <button
            type="button"
            role="switch"
            aria-checked={settings.killSwitch}
            onClick={() => onUpdateSettings({ killSwitch: !settings.killSwitch })}
            className={`relative w-10 h-5 rounded-full transition-colors shrink-0 ${
              settings.killSwitch ? 'bg-[#22C55E]' : 'bg-[#30353D]'
            }`}
          >
            <span
              className={`block w-3.5 h-3.5 rounded-full bg-[#111315] transition-transform duration-150 ${
                settings.killSwitch ? 'translate-x-5.5' : 'translate-x-1'
              }`}
            />
          </button>
        </div>

        <div className="p-5 flex items-center justify-between gap-4">
          <div>
            <div className="text-sm font-medium text-[#F1F3F5]">Allow LAN Connections</div>
            <div className="text-xs text-[#959CA6] mt-0.5">
              Expose local SOCKS5/HTTP listeners on 0.0.0.0 for devices on local subnet
            </div>
          </div>
          <button
            type="button"
            role="switch"
            aria-checked={settings.allowLan}
            onClick={() => onUpdateSettings({ allowLan: !settings.allowLan })}
            className={`relative w-10 h-5 rounded-full transition-colors shrink-0 ${
              settings.allowLan ? 'bg-[#22C55E]' : 'bg-[#30353D]'
            }`}
          >
            <span
              className={`block w-3.5 h-3.5 rounded-full bg-[#111315] transition-transform duration-150 ${
                settings.allowLan ? 'translate-x-5.5' : 'translate-x-1'
              }`}
            />
          </button>
        </div>

        <div className="p-5 flex items-center justify-between gap-4">
          <div>
            <div className="text-sm font-medium text-[#F1F3F5]">Route IPv6 Traffic</div>
            <div className="text-xs text-[#959CA6] mt-0.5">
              Enable dual-stack ghost0 interface; disable to suppress AAAA leak risk
            </div>
          </div>
          <button
            type="button"
            role="switch"
            aria-checked={settings.ipv6Routing}
            onClick={() => onUpdateSettings({ ipv6Routing: !settings.ipv6Routing })}
            className={`relative w-10 h-5 rounded-full transition-colors shrink-0 ${
              settings.ipv6Routing ? 'bg-[#22C55E]' : 'bg-[#30353D]'
            }`}
          >
            <span
              className={`block w-3.5 h-3.5 rounded-full bg-[#111315] transition-transform duration-150 ${
                settings.ipv6Routing ? 'translate-x-5.5' : 'translate-x-1'
              }`}
            />
          </button>
        </div>
      </section>

      {/* Network Interface & DNS Parameters */}
      <section className="rounded-xl bg-[#1A1C20] border border-[#292D34] p-6 space-y-5">
        <h3 className="text-sm font-semibold text-[#F1F3F5]">Interface & DNS Parameters</h3>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <div>
            <label className="block text-xs text-[#959CA6] mb-1.5" htmlFor="tun-stack">
              TUN Stack
            </label>
            <div className="relative">
              <select
                id="tun-stack"
                value={settings.tunStack}
                onChange={(e) =>
                  onUpdateSettings({
                    tunStack: e.target.value as AppSettings['tunStack'],
                  })
                }
                className="w-full appearance-none pl-3 pr-8 py-2 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs font-mono text-[#F1F3F5] focus:outline-none focus:border-[#22C55E]"
              >
                <option value="system">system</option>
                <option value="gvisor">gvisor</option>
                <option value="mixed">mixed</option>
              </select>
              <ChevronDown className="w-3.5 h-3.5 text-[#959CA6] absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
            </div>
          </div>

          <div>
            <label className="block text-xs text-[#959CA6] mb-1.5" htmlFor="tun-mtu">
              MTU Size
            </label>
            <input
              id="tun-mtu"
              type="number"
              min={1200}
              max={9000}
              value={settings.mtu}
              onChange={(e) => onUpdateSettings({ mtu: Number(e.target.value) || 1420 })}
              className="w-full px-3 py-2 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs font-mono tabular-nums text-[#F1F3F5] focus:outline-none focus:border-[#22C55E]"
            />
          </div>

          <div>
            <label className="block text-xs text-[#959CA6] mb-1.5" htmlFor="socks-port">
              SOCKS5 Port
            </label>
            <input
              id="socks-port"
              type="number"
              value={settings.socksPort}
              onChange={(e) => onUpdateSettings({ socksPort: Number(e.target.value) || 10808 })}
              className="w-full px-3 py-2 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs font-mono tabular-nums text-[#F1F3F5] focus:outline-none focus:border-[#22C55E]"
            />
          </div>

          <div>
            <label className="block text-xs text-[#959CA6] mb-1.5" htmlFor="http-port">
              HTTP Proxy Port
            </label>
            <input
              id="http-port"
              type="number"
              value={settings.httpPort}
              onChange={(e) => onUpdateSettings({ httpPort: Number(e.target.value) || 10809 })}
              className="w-full px-3 py-2 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs font-mono tabular-nums text-[#F1F3F5] focus:outline-none focus:border-[#22C55E]"
            />
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <label className="block text-xs text-[#959CA6] mb-1.5" htmlFor="dns-primary">
              Primary Encrypted DNS (DoH)
            </label>
            <input
              id="dns-primary"
              type="text"
              value={settings.dnsPrimary}
              onChange={(e) => onUpdateSettings({ dnsPrimary: e.target.value })}
              className="w-full px-3 py-2 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs font-mono text-[#F1F3F5] focus:outline-none focus:border-[#22C55E]"
            />
          </div>

          <div>
            <label className="block text-xs text-[#959CA6] mb-1.5" htmlFor="dns-fallback">
              Fallback DNS (DoT)
            </label>
            <input
              id="dns-fallback"
              type="text"
              value={settings.dnsFallback}
              onChange={(e) => onUpdateSettings({ dnsFallback: e.target.value })}
              className="w-full px-3 py-2 rounded-lg bg-[#131518] border border-[#2A2E35] text-xs font-mono text-[#F1F3F5] focus:outline-none focus:border-[#22C55E]"
            />
          </div>
        </div>
      </section>
    </div>
  );
};
