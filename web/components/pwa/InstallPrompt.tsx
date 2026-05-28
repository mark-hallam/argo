"use client";

import { useCallback, useEffect, useState } from "react";

const DISMISS_KEY = "skycot-pwa-install-dismissed-at";
const DISMISS_TTL_DAYS = 30;

/**
 * Minimal subset of BeforeInstallPromptEvent. Standard DOM lib does
 * not include it (it's Chromium-specific). We only need prompt() +
 * the userChoice resolution.
 */
type BeforeInstallPromptEvent = Event & {
  prompt: () => Promise<void>;
  userChoice: Promise<{ outcome: "accepted" | "dismissed" }>;
};

function dismissIsActive(): boolean {
  if (typeof window === "undefined") return false;
  try {
    const raw = window.localStorage.getItem(DISMISS_KEY);
    if (!raw) return false;
    const ts = Number(raw);
    if (!Number.isFinite(ts)) return false;
    const ageDays = (Date.now() - ts) / (1000 * 60 * 60 * 24);
    return ageDays < DISMISS_TTL_DAYS;
  } catch {
    return false;
  }
}

function isLocalhostOrigin(): boolean {
  if (typeof window === "undefined") return false;
  const host = window.location.hostname;
  return host === "localhost" || host === "127.0.0.1" || host.endsWith(".local");
}

function hasHideQueryParam(): boolean {
  if (typeof window === "undefined") return false;
  try {
    return new URL(window.location.href).searchParams.has("hideInstallPrompt");
  } catch {
    return false;
  }
}

/**
 * Bottom banner that listens for `beforeinstallprompt` and offers
 * Install + Dismiss. Dismiss persists for 30 days via localStorage.
 * Hidden when already installed (display-mode: standalone) or when
 * the event never fires (iOS Safari, unsupported browsers).
 */
export function InstallPrompt(): React.ReactElement | null {
  const [event, setEvent] = useState<BeforeInstallPromptEvent | null>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (typeof window === "undefined") return;
    // Suppression channels (any one wins; never registers listeners):
    //   1. NEXT_PUBLIC_HIDE_PWA_PROMPT=1 (wave-6 demo-record env var)
    //   2. NEXT_PUBLIC_SKYCOT_DEMO_RECORDING=1 (recording-mode flag,
    //      distinct from NEXT_PUBLIC_SKYCOT_DEMO_MODE which gates
    //      sample-data fallback)
    //   3. ?hideInstallPrompt=1 in the URL (ad-hoc operator override)
    //   4. hostname is localhost / 127.0.0.1 / *.local (dev origins
    //      never need the install affordance and the banner ruins
    //      every screenshot during a recording walk)
    //   5. 30-day dismissal still active in localStorage
    //   6. PWA already installed (matchMedia display-mode: standalone)
    if (process.env.NEXT_PUBLIC_HIDE_PWA_PROMPT === "1") return;
    if (process.env.NEXT_PUBLIC_SKYCOT_DEMO_RECORDING === "1") return;
    if (hasHideQueryParam()) return;
    if (isLocalhostOrigin()) return;
    if (dismissIsActive()) return;
    // Already installed (matchMedia is true once the page is launched standalone).
    if (window.matchMedia?.("(display-mode: standalone)").matches) return;

    const onPrompt = (e: Event) => {
      e.preventDefault();
      setEvent(e as BeforeInstallPromptEvent);
      setVisible(true);
    };
    const onInstalled = () => {
      setVisible(false);
      setEvent(null);
    };

    window.addEventListener("beforeinstallprompt", onPrompt);
    window.addEventListener("appinstalled", onInstalled);
    return () => {
      window.removeEventListener("beforeinstallprompt", onPrompt);
      window.removeEventListener("appinstalled", onInstalled);
    };
  }, []);

  const onInstall = useCallback(async () => {
    if (!event) return;
    try {
      await event.prompt();
      await event.userChoice;
    } catch {
      /* user gesture lost or denied; just hide */
    } finally {
      setVisible(false);
      setEvent(null);
    }
  }, [event]);

  const onDismiss = useCallback(() => {
    try {
      window.localStorage.setItem(DISMISS_KEY, String(Date.now()));
    } catch {
      /* private-mode storage failure — fall through, just hide */
    }
    setVisible(false);
  }, []);

  if (!visible || !event) return null;

  return (
    <div
      role="region"
      aria-label="Install SKYCOT"
      data-testid="pwa-install-banner"
      className="skycot-install-banner"
    >
      <div className="skycot-install-banner__inner">
        <div className="skycot-install-banner__copy">
          <strong className="skycot-install-banner__title">Install SKYCOT</strong>
          <span className="skycot-install-banner__subtitle">
            Get it on your homescreen for one-tap access.
          </span>
        </div>
        <div className="skycot-install-banner__actions">
          <button
            type="button"
            onClick={onDismiss}
            className="skycot-install-banner__btn skycot-install-banner__btn--ghost"
          >
            Not now
          </button>
          <button
            type="button"
            onClick={onInstall}
            className="skycot-install-banner__btn skycot-install-banner__btn--primary"
          >
            Install
          </button>
        </div>
      </div>
      <style jsx>{`
        .skycot-install-banner {
          position: fixed;
          left: 12px;
          right: 12px;
          bottom: 12px;
          z-index: 9999;
          background: white;
          border: 1px solid rgba(139, 92, 246, 0.18);
          border-radius: 14px;
          box-shadow: 0 12px 32px -8px rgba(30, 27, 75, 0.25);
          padding: 12px 14px;
          font-family: "Inter", sans-serif;
        }
        @media (min-width: 640px) {
          .skycot-install-banner {
            left: auto;
            right: 20px;
            bottom: 20px;
            max-width: 420px;
          }
        }
        .skycot-install-banner__inner {
          display: flex;
          align-items: center;
          gap: 12px;
        }
        .skycot-install-banner__copy {
          display: flex;
          flex-direction: column;
          flex: 1 1 auto;
          min-width: 0;
        }
        .skycot-install-banner__title {
          font-weight: 700;
          color: var(--foreground);
          font-size: 14px;
        }
        .skycot-install-banner__subtitle {
          font-size: 12px;
          color: var(--muted-foreground);
          margin-top: 2px;
        }
        .skycot-install-banner__actions {
          display: flex;
          gap: 8px;
          flex-shrink: 0;
        }
        .skycot-install-banner__btn {
          padding: 8px 14px;
          border-radius: 999px;
          border: 0;
          font-weight: 600;
          font-size: 13px;
          cursor: pointer;
          font-family: inherit;
        }
        .skycot-install-banner__btn--ghost {
          background: transparent;
          color: var(--muted-foreground);
        }
        .skycot-install-banner__btn--primary {
          background: linear-gradient(135deg, var(--primary), var(--gradient-end));
          color: white;
          box-shadow: 0 6px 16px -4px rgba(139, 92, 246, 0.45);
        }
      `}</style>
    </div>
  );
}
