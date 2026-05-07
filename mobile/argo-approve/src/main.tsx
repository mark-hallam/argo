import { StrictMode, useCallback, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { ApprovalsList } from "./screens/ApprovalsList";
import { Settings } from "./screens/Settings";
import { loadSettings, saveSettings } from "./store";
import { SETTINGS_DEFAULTS, type AppSettings } from "./types";
import "./styles.css";

type View = "list" | "settings";

function App() {
  const [settings, setSettings] = useState<AppSettings>(SETTINGS_DEFAULTS);
  const [view, setView] = useState<View>("list");
  const [highlight, setHighlight] = useState<string | null>(null);
  const [bootError, setBootError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const loaded = await loadSettings();
        if (cancelled) return;
        setSettings(loaded);
        if (!loaded.bearer_token) setView("settings");
      } catch (e) {
        if (cancelled) return;
        setBootError((e as Error).message);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    (async () => {
      try {
        unlisten = await onOpenUrl((urls) => {
          for (const url of urls) {
            const id = parseApproveUrl(url);
            if (id) {
              setHighlight(id);
              setView("list");
            }
          }
        });
      } catch {
        /* deep-link plugin only available on supported platforms */
      }
    })();
    return () => {
      unlisten?.();
    };
  }, []);

  const onSave = useCallback(async (next: AppSettings) => {
    await saveSettings(next);
    setSettings(next);
  }, []);

  if (bootError) {
    return (
      <div className="app">
        <main>
          <div className="error">Failed to load settings: {bootError}</div>
        </main>
      </div>
    );
  }

  if (view === "settings") {
    return <Settings settings={settings} onSave={onSave} onClose={() => setView("list")} />;
  }
  return (
    <ApprovalsList
      settings={settings}
      highlightApprovalId={highlight}
      onOpenSettings={() => setView("settings")}
    />
  );
}

function parseApproveUrl(raw: string): string | null {
  try {
    const url = new URL(raw);
    if (url.protocol !== "argo:") return null;
    const segments = url.pathname.split("/").filter(Boolean);
    return segments[0] ?? null;
  } catch {
    return null;
  }
}

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
