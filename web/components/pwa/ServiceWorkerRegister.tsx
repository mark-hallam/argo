"use client";

import { useEffect } from "react";

/**
 * Registers /sw.js once on mount. No-op in environments without
 * service-worker support (older Safari, server, jsdom test runner).
 * Intentionally silent on failure — the app still works without a SW.
 */
export function ServiceWorkerRegister(): null {
  useEffect(() => {
    if (typeof window === "undefined") return;
    if (!("serviceWorker" in navigator)) return;
    // Skip registration in dev to avoid stale caching while iterating.
    if (process.env.NODE_ENV !== "production") return;

    const register = () => {
      navigator.serviceWorker.register("/sw.js").catch((err) => {
        // eslint-disable-next-line no-console
        console.error("[skycot-pwa] sw register failed", err);
      });
    };

    if (document.readyState === "complete") {
      register();
    } else {
      window.addEventListener("load", register, { once: true });
    }
  }, []);

  return null;
}
