"use client";

import { useEffect } from "react";
import { resolveGatewayUrl } from "@/lib/config/gateway-url";

type ErrorReportingConfig =
  | { enabled: false; mode?: string }
  | { enabled: true; mode: "sentry"; dsn: string }
  | { enabled: true; mode: "otlp"; otlp_endpoint: string };

type Fetcher = typeof fetch;

let activeConfig: ErrorReportingConfig | null = null;
let configLoaded = false;
let listenersInstalled = false;
let errorHandler: ((event: ErrorEvent) => void) | null = null;
let rejectionHandler: ((event: PromiseRejectionEvent) => void) | null = null;
let transportFetch: Fetcher = fetch;

// Cooldown + recursion guards. F16 (UX audit 2026-05-27-0415) traced a runaway
// fetch loop to the reporter's own `send()` failing on CORS, the throw
// firing `unhandledrejection`, the handler refiring `reportBrowserError`,
// and so on at <100ms intervals. The cooldown caps reports to one per
// 30s after the first failure; `reportingInFlight` blocks the recursive
// case where the report's own fetch throws before the cooldown is set.
let lastFailureAt = 0;
let reportingInFlight = false;
const FAILURE_COOLDOWN_MS = 30_000;

export function ErrorReportingInit({ source = "skycot-web" }: { source?: string }) {
  // Empty deps — the listener attaches once per session, not once per
  // navigation. `source` is a stable string in practice, but pinning to
  // `[]` removes any chance of re-entry on layout transitions.
  useEffect(() => {
    void initBrowserErrorReporting(source);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return null;
}

export async function initBrowserErrorReporting(
  source = "skycot-web",
  fetcher: Fetcher = fetch,
): Promise<ErrorReportingConfig | null> {
  if (typeof window === "undefined") return null;
  if (configLoaded) return activeConfig;
  configLoaded = true;
  transportFetch = fetcher;
  const cfg = await loadConfig(fetcher);
  activeConfig = cfg;
  if (cfg?.enabled && !listenersInstalled) {
    errorHandler = (event) => {
      void reportBrowserError(source, event.error ?? event.message ?? "browser error");
    };
    rejectionHandler = (event) => {
      void reportBrowserError(source, event.reason ?? "unhandled promise rejection");
    };
    window.addEventListener("error", errorHandler);
    window.addEventListener("unhandledrejection", rejectionHandler);
    listenersInstalled = true;
  }
  return cfg;
}

async function loadConfig(fetcher: Fetcher): Promise<ErrorReportingConfig | null> {
  const base = resolveGatewayUrl();
  try {
    const res = await fetcher(`${base}/api/v1/meta/error-reporting`, {
      credentials: "omit",
      headers: { Accept: "application/json" },
    });
    if (!res.ok) return null;
    return (await res.json()) as ErrorReportingConfig;
  } catch {
    return null;
  }
}

async function reportBrowserError(source: string, reason: unknown): Promise<void> {
  const cfg = activeConfig;
  if (!cfg?.enabled) return;
  if (reportingInFlight) return;
  if (Date.now() - lastFailureAt < FAILURE_COOLDOWN_MS) return;
  reportingInFlight = true;
  try {
    const message = scrub(messageFrom(reason));
    if (cfg.mode === "sentry") {
      await postSentry(cfg.dsn, source, message, reason);
    } else if (cfg.mode === "otlp") {
      await postOtlp(cfg.otlp_endpoint, source, message);
    }
  } catch {
    // Swallow. Re-throwing would refire `window.error` /
    // `unhandledrejection` and recurse — the F16 positive-feedback loop.
    lastFailureAt = Date.now();
  } finally {
    reportingInFlight = false;
  }
}

function messageFrom(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  try {
    return JSON.stringify(reason);
  } catch {
    return "unknown browser error";
  }
}

async function postSentry(
  dsn: string,
  source: string,
  message: string,
  reason: unknown,
): Promise<void> {
  const target = sentryEnvelopeUrl(dsn);
  if (!target) return;
  const eventId = eventIdHex();
  const exceptionValue = reason instanceof Error ? scrub(reason.stack ?? reason.message) : message;
  const envelope = [
    JSON.stringify({ event_id: eventId, sent_at: new Date().toISOString() }),
    JSON.stringify({ type: "event" }),
    JSON.stringify({
      event_id: eventId,
      platform: "javascript",
      level: "error",
      message,
      tags: { source },
      exception: { values: [{ type: "BrowserError", value: exceptionValue }] },
    }),
  ].join("\n");
  await send(target, envelope, "application/x-sentry-envelope");
}

async function postOtlp(endpoint: string, source: string, message: string): Promise<void> {
  const base = endpoint.endsWith("/v1/logs") ? endpoint : `${endpoint.replace(/\/$/, "")}/v1/logs`;
  const body = {
    resourceLogs: [
      {
        resource: { attributes: [{ key: "service.name", value: { stringValue: source } }] },
        scopeLogs: [
          {
            scope: { name: "skycot.browser.error-reporting" },
            logRecords: [
              {
                timeUnixNano: String(Date.now() * 1_000_000),
                severityText: "ERROR",
                body: { stringValue: message },
              },
            ],
          },
        ],
      },
    ],
  };
  await send(base, JSON.stringify(body), "application/json");
}

async function send(url: string, body: string, contentType: string): Promise<void> {
  if (navigator.sendBeacon) {
    const blob = new Blob([body], { type: contentType });
    if (navigator.sendBeacon(url, blob)) return;
  }
  await transportFetch(url, { method: "POST", body, keepalive: true, headers: { "Content-Type": contentType } });
}

function sentryEnvelopeUrl(dsn: string): string | null {
  try {
    const parsed = new URL(dsn);
    const key = parsed.username;
    const project = parsed.pathname.split("/").filter(Boolean).at(-1);
    if (!key || !project) return null;
    return `${parsed.origin}/api/${project}/envelope/?sentry_key=${key}&sentry_version=7`;
  } catch {
    return null;
  }
}

function eventIdHex(): string {
  const id = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}${Math.random()}`;
  return id.replace(/[^a-fA-F0-9]/g, "").padEnd(32, "0").slice(0, 32);
}

function scrub(input: string): string {
  return input
    .replace(/[A-Z0-9._%+\-]+@[A-Z0-9.\-]+\.[A-Z]{2,}/gi, "<redacted:email>")
    .replace(/eyJ[A-Za-z0-9_-]{20,}(?:\.[A-Za-z0-9_-]+){0,2}/g, "<redacted:jwt>")
    .replace(/sk-[A-Za-z0-9]{20,}/g, "<redacted:provider-key>")
    .replace(/\+?[0-9][0-9 ()\-\.]{7,}[0-9]/g, "<redacted:phone>");
}

export function __resetErrorReportingForTests() {
  if (errorHandler) window.removeEventListener("error", errorHandler);
  if (rejectionHandler) window.removeEventListener("unhandledrejection", rejectionHandler);
  activeConfig = null;
  configLoaded = false;
  listenersInstalled = false;
  errorHandler = null;
  rejectionHandler = null;
  transportFetch = fetch;
  lastFailureAt = 0;
  reportingInFlight = false;
}
