import React, { useState } from 'react';
import { ClipboardPaste, Link2, Loader2, Terminal, X } from 'lucide-react';
import { ServerNode, Subscription } from '../types/vpn';
import { GhostApi } from '../services/api';

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
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handlePasteClipboardSub = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        setSubUrl(text.trim());
        setError(null);
      }
    } catch {
      setError('Unable to read from clipboard. Please paste manually.');
    }
  };

  const handlePasteClipboardUri = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        setRawUri(text.trim());
        setError(null);
      }
    } catch {
      setError('Unable to read from clipboard. Please paste manually.');
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      if (mode === 'subscription') {
        const trimmedUrl = subUrl.trim();
        if (!trimmedUrl.startsWith('http://') && !trimmedUrl.startsWith('https://')) {
          setError('Enter a valid http:// or https:// subscription endpoint.');
          setLoading(false);
          return;
        }

        const newSub = await GhostApi.importSubscription(trimmedUrl, subName.trim() || undefined);
        const allNodes = await GhostApi.listNodes();
        const subNodes = allNodes.filter((n) => n.subscriptionId === newSub.id);
        onAddSubscription(newSub, subNodes);
        setSubName('');
        setSubUrl('');
        onClose();
        return;
      }

      const trimmedUri = rawUri.trim();
      if (!trimmedUri) {
        setError('Please paste a valid server share URI.');
        setLoading(false);
        return;
      }

      const res = await GhostApi.importProfile(trimmedUri);
      if (res && res.node) {
        onAddSingleNode(res.node);
        setRawUri('');
        onClose();
      } else {
        throw new Error('Daemon did not return imported node');
      }
    } catch (err: any) {
      setError(err?.message || 'Failed to import configuration from daemon');
    } finally {
      setLoading(false);
    }
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
                  Profile Name (Optional)
                </label>
                <input
                  id="sub-name"
                  type="text"
                  value={subName}
                  onChange={(e) => setSubName(e.target.value)}
                  placeholder="e.g. My VPN Subscription"
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
                    onClick={handlePasteClipboardSub}
                    className="inline-flex items-center gap-1.5 text-xs text-[#22C55E] hover:text-[#4ADE80] transition-colors"
                  >
                    <ClipboardPaste className="w-3.5 h-3.5" />
                    Paste Clipboard
                  </button>
                </div>
                <input
                  id="sub-url"
                  type="text"
                  required
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
                  Share Link (vless://, vmess://, trojan://, ss://)
                </label>
                <button
                  type="button"
                  onClick={handlePasteClipboardUri}
                  className="inline-flex items-center gap-1.5 text-xs text-[#22C55E] hover:text-[#4ADE80] transition-colors"
                >
                  <ClipboardPaste className="w-3.5 h-3.5" />
                  Paste Clipboard
                </button>
              </div>
              <textarea
                id="raw-uri"
                rows={4}
                required
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
              disabled={loading}
              className="px-4 py-2 rounded-lg text-xs font-medium text-[#98A69E] hover:text-[#ECF2EE] hover:bg-[#18201B] transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading}
              className="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold bg-[#22C55E] text-[#05140A] hover:bg-[#16A34A] transition-colors disabled:opacity-60"
            >
              {loading && <Loader2 className="w-3.5 h-3.5 animate-spin" />}
              {mode === 'subscription' ? 'Fetch & Import' : 'Add Node'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
