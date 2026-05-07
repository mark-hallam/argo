import { useState } from "react";
import type { AppSettings } from "../types";

interface Props {
  settings: AppSettings;
  onSave: (next: AppSettings) => Promise<void>;
  onClose: () => void;
}

export function Settings({ settings, onSave, onClose }: Props) {
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await onSave({
        gateway_url: draft.gateway_url.trim(),
        bearer_token: draft.bearer_token.trim(),
      });
      onClose();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="app">
      <header>
        <h1>Settings</h1>
        <button className="link" onClick={onClose}>
          Done
        </button>
      </header>
      <main>
        {error ? <div className="error">{error}</div> : null}
        <form onSubmit={onSubmit}>
          <label htmlFor="gateway">Gateway URL</label>
          <input
            id="gateway"
            type="url"
            inputMode="url"
            autoCapitalize="off"
            autoCorrect="off"
            spellCheck={false}
            value={draft.gateway_url}
            onChange={(e) => setDraft({ ...draft, gateway_url: e.target.value })}
            placeholder="http://localhost:7090"
          />

          <label htmlFor="bearer">Bearer token</label>
          <input
            id="bearer"
            type="password"
            autoCapitalize="off"
            autoCorrect="off"
            spellCheck={false}
            value={draft.bearer_token}
            onChange={(e) => setDraft({ ...draft, bearer_token: e.target.value })}
            placeholder="paste from `argo cred get ARGO_WEBINBOX_BEARER_TOKEN`"
          />

          <div className="actions" style={{ marginTop: 16 }}>
            <button type="submit" className="primary" disabled={busy}>
              {busy ? "Saving…" : "Save"}
            </button>
          </div>
        </form>
      </main>
    </div>
  );
}
