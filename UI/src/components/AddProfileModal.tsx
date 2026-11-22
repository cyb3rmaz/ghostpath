import React, { useState } from 'react';
import { ClipboardPaste, Link2, Terminal, X } from 'lucide-react';
import { NodeProtocol, ServerNode, Subscription } from '../types/vpn';

interface AddProfileModalProps {
  isOpen: boolean;
  onClose: () => void;
  onAddSubscription: (sub: Subscription, nodes: ServerNode[]) => void;
  onAddSingleNode: (node: ServerNode) => void;
}

export const AddProfileModal: React.FC<AddProfileModalProps> = ({
  isOpen,
  onClose,
  onAddSubscription,
  onAddSingleNode,
}) => {
  const [mode, setMode] = useState<'subscription' | 'uri'>('subscription');
  const [subName, setSubName] = useState('');
  const [subUrl, setSubUrl] = useState('');
  const [rawUri, setRawUri] = useState('');
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleInsertSampleUri = () => {
    setRawUri('vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@195.181.162.88:443?type=tcp&security=reality&flow=xtls-rprx-vision#Stockholm%20SE-01');
    setError(null);
  };

  const handleInsertSampleSub = () => {
    setSubName('Baltic Edge Relay');
    setSubUrl('https://sub.baltic-mesh.io/v2/profile/linux-x86_64?token=49ac72e1');
    setError(null);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (mode === 'subscription') {
      const trimmedUrl = subUrl.trim();
      if (!trimmedUrl.startsWith('http://') && !trimmedUrl.startsWith('https://')) {
        setError('Enter a valid http:// or https:// subscription endpoint.');
        return;
      }
      const id = `sub-${Date.now()}`;
      const name = subName.trim() || new URL(trimmedUrl).hostname;
      const newSub: Subscription = {
        id,
        name,
        url: trimmedUrl,
        usedGb: 0,
        totalGb: 1000,
        expiresAt: '2027-11-22',
        lastUpdated: 'Just now',
        autoUpdateHours: 12,
      };
      const generatedNodes: ServerNode[] = [
        {
          id: `node-${Date.now()}-1`,
          name: `${name.split(' ')[0]} Tallinn EE-01`,
          countryCode: 'EE',
          city: 'Tallinn',
          country: 'Estonia',
          protocol: 'VLESS',
          transport: 'TCP · Vision',
          security: 'Reality',
          host: '185.158.248.31',
          port: 443,
          latencyMs: 36,
          loadPercent: 18,
          subscriptionId: id,
        },
        {
          id: `node-${Date.now()}-2`,
          name: `${name.split(' ')[0]} Riga LV-01`,
          countryCode: 'LV',
          city: 'Riga',
          country: 'Latvia',
          protocol: 'Hysteria2',
          transport: 'QUIC · BBR',
          security: 'TLS 1.3',
          host: '91.219.236.14',
          port: 8443,
          latencyMs: 41,
          loadPercent: 22,
          subscriptionId: id,
        },
      ];
      onAddSubscription(newSub, generatedNodes);
      setSubName('');
      setSubUrl('');
      onClose();
      return;
    }

    const trimmedUri = rawUri.trim();
    const supportedPrefixes = ['vless://', 'hy2://', 'hysteria2://', 'trojan://', 'wg://'];
    const matchedPrefix = supportedPrefixes.find((p) => trimmedUri.toLowerCase().startsWith(p));
    if (!matchedPrefix) {
      setError('URI must start with vless://, hy2://, trojan://, or wg://');
      return;
    }

    let protocol: NodeProtocol = 'VLESS';
    if (matchedPrefix.startsWith('hy')) protocol = 'Hysteria2';
    else if (matchedPrefix.startsWith('trojan')) protocol = 'Trojan';
    else if (matchedPrefix.startsWith('wg')) protocol = 'WireGuard';

    const hashIndex = trimmedUri.indexOf('#');
    const parsedLabel =
      hashIndex !== -1 ? decodeURIComponent(trimmedUri.slice(hashIndex + 1)).trim() : '';

    const hostMatch = trimmedUri.match(/@([^:/?#]+):(\d+)/);
    const host = hostMatch ? hostMatch[1] : '195.181.162.88';
    const port = hostMatch ? Number(hostMatch[2]) : 443;

    const newNode: ServerNode = {
      id: `node-${Date.now()}`,
      name: parsedLabel || `${protocol} Custom Node`,
      countryCode: 'SE',
      city: parsedLabel ? parsedLabel.split(' ')[0] : 'Stockholm',
      country: 'Custom Endpoint',
      protocol,
      transport: protocol === 'Hysteria2' ? 'QUIC · BBR' : protocol === 'WireGuard' ? 'UDP · Kernel' : 'TCP · Vision',
      security: protocol === 'VLESS' ? 'Reality' : protocol === 'WireGuard' ? 'Noise_IK' : 'TLS 1.3',
      host,
      port,
      latencyMs: 33,
      loadPercent: 15,
      subscriptionId: 'sub-personal',
    };

    onAddSingleNode(newNode);
    setRawUri('');
    onClose();
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-[2px] p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="import-modal-title"
    >
      <div className="w-full max-w-lg rounded-xl bg-[#101512] border border-[#212D26] shadow-2xl overflow-hidden">
        <div className="flex items-center justify-between px-6 py-4 border-b border-[#1B241F]">
          <div>
            <h2 id="import-modal-title" className="text-base font-semibold text-[#ECF2EE]">
              Import Configuration
            </h2>
            <p className="text-xs text-[#8A9990] mt-0.5">
              Subscribe to a remote profile URL or import a single node URI
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-2 rounded-lg text-[#8A9990] hover:text-[#ECF2EE] hover:bg-[#18201B] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#22C55E]"
            aria-label="Close import modal"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="px-6 pt-4">
          <div className="grid grid-cols-2 gap-1 p-1 rounded-lg bg-[#0A0D0B] border border-[#1B241F]">
            <button
              type="button"
              onClick={() => {
                setMode('subscription');
                setError(null);
              }}
              className={`flex items-center justify-center gap-2 py-2 px-3 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
                mode === 'subscription'
                  ? 'bg-[#17201B] text-[#ECF2EE] border border-[#26352C]'
                  : 'text-[#8A9990] hover:text-[#ECF2EE]'
              }`}
            >
              <Link2 className="w-3.5 h-3.5" />
              Subscription URL
            </button>
            <button
              type="button"
              onClick={() => {
                setMode('uri');
                setError(null);
              }}
              className={`flex items-center justify-center gap-2 py-2 px-3 rounded-md text-xs font-medium transition-colors whitespace-nowrap ${
                mode === 'uri'
                  ? 'bg-[#17201B] text-[#ECF2EE] border border-[#26352C]'
                  : 'text-[#8A9990] hover:text-[#ECF2EE]'
              }`}
            >
              <Terminal className="w-3.5 h-3.5" />
              Node Share URI
            </button>
          </div>
        </div>

        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          {mode === 'subscription' ? (
            <>
              <div>
                <label className="block text-xs font-medium text-[#B3C0B8] mb-1.5" htmlFor="sub-name">
                  Profile Name
                </label>
                <input
                  id="sub-name"
                  type="text"
                  value={subName}
                  onChange={(e) => setSubName(e.target.value)}
                  placeholder="e.g. Baltic Edge Relay"
                  className="w-full px-3.5 py-2.5 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-sm text-[#ECF2EE] placeholder-[#5A6A61] focus:outline-none focus:border-[#22C55E]"
                />
              </div>
              <div>
                <div className="flex items-center justify-between mb-1.5">
                  <label className="block text-xs font-medium text-[#B3C0B8]" htmlFor="sub-url">
                    Subscription Endpoint
                  </label>
                  <button
                    type="button"
                    onClick={handleInsertSampleSub}
                    className="inline-flex items-center gap-1.5 text-xs text-[#22C55E] hover:text-[#4ADE80] transition-colors"
                  >
                    <ClipboardPaste className="w-3.5 h-3.5" />
                    Paste Sample URL
                  </button>
                </div>
                <input
                  id="sub-url"
                  type="text"
                  value={subUrl}
                  onChange={(e) => setSubUrl(e.target.value)}
                  placeholder="https://example.com/api/v1/subscribe?token=..."
                  className="w-full px-3.5 py-2.5 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs font-mono text-[#ECF2EE] placeholder-[#5A6A61] focus:outline-none focus:border-[#22C55E]"
                />
              </div>
            </>
          ) : (
            <div>
              <div className="flex items-center justify-between mb-1.5">
                <label className="block text-xs font-medium text-[#B3C0B8]" htmlFor="raw-uri">
                  Share Link (VLESS, Hysteria2, Trojan, WireGuard)
                </label>
                <button
                  type="button"
                  onClick={handleInsertSampleUri}
                  className="inline-flex items-center gap-1.5 text-xs text-[#22C55E] hover:text-[#4ADE80] transition-colors"
                >
                  <ClipboardPaste className="w-3.5 h-3.5" />
                  Paste Sample URI
                </button>
              </div>
              <textarea
                id="raw-uri"
                rows={4}
                value={rawUri}
                onChange={(e) => setRawUri(e.target.value)}
                placeholder="vless://uuid@host:443?security=reality#NodeName"
                className="w-full px-3.5 py-2.5 rounded-lg bg-[#0A0D0B] border border-[#1E2822] text-xs font-mono text-[#ECF2EE] placeholder-[#5A6A61] focus:outline-none focus:border-[#22C55E] resize-none"
              />
            </div>
          )}

          {error && (
            <p className="text-xs text-[#F87171] font-medium" role="alert">
              {error}
            </p>
          )}

          <div className="flex items-center justify-end gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-lg text-xs font-medium text-[#98A69E] hover:text-[#ECF2EE] hover:bg-[#18201B] transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-4 py-2 rounded-lg text-xs font-semibold bg-[#22C55E] text-[#05140A] hover:bg-[#16A34A] transition-colors"
            >
              {mode === 'subscription' ? 'Fetch & Import' : 'Add Node'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
