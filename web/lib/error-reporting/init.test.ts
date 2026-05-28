import { afterEach, describe, expect, it, vi } from "vitest";
import { __resetErrorReportingForTests, initBrowserErrorReporting } from "./init";

afterEach(() => {
  __resetErrorReportingForTests();
  vi.restoreAllMocks();
});

describe("browser error reporting", () => {
  it("keeps disabled config as a fetch-only no-op", async () => {
    stubBeacon(false);
    const fetchMock = vi.fn(async () => jsonResponse({ enabled: false, mode: "disabled" }));
    await initBrowserErrorReporting("test", fetchMock as typeof fetch);
    window.dispatchEvent(new ErrorEvent("error", { message: "boom" }));
    await tick();
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("captures global errors through the configured Sentry envelope endpoint", async () => {
    stubBeacon(false);
    const calls: Array<{ url: string; init?: RequestInit }> = [];
    const fetchMock = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), init });
      if (calls.length === 1) {
        return jsonResponse({
          enabled: true,
          mode: "sentry",
          dsn: "https://public@sentry.example/42",
        });
      }
      return jsonResponse({ ok: true });
    });
    await initBrowserErrorReporting("test", fetchMock as typeof fetch);
    window.dispatchEvent(
      new ErrorEvent("error", {
        error: new Error("email alice@example.com key sk-12345678901234567890"),
      }),
    );
    await tick();
    expect(calls[1]?.url).toContain("https://sentry.example/api/42/envelope/");
    const body = String(calls[1]?.init?.body ?? "");
    expect(body).toContain("<redacted:email>");
    expect(body).toContain("<redacted:provider-key>");
    expect(body).not.toContain("alice@example.com");
  });

  it("captures unhandled promise rejections through OTLP logs", async () => {
    stubBeacon(false);
    const calls: Array<{ url: string; init?: RequestInit }> = [];
    const fetchMock = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), init });
      if (calls.length === 1) {
        return jsonResponse({
          enabled: true,
          mode: "otlp",
          otlp_endpoint: "https://otel.example",
        });
      }
      return jsonResponse({ ok: true });
    });
    await initBrowserErrorReporting("test", fetchMock as typeof fetch);
    window.dispatchEvent(
      Object.assign(new Event("unhandledrejection"), {
        reason: "token eyJaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        promise: Promise.resolve(),
      }),
    );
    await tick();
    expect(calls[1]?.url).toBe("https://otel.example/v1/logs");
    expect(String(calls[1]?.init?.body ?? "")).toContain("<redacted:jwt>");
  });

  it("swallows transport CORS failures so the listener does not recurse (F16)", async () => {
    stubBeacon(false);
    const calls: Array<{ url: string }> = [];
    const fetchMock = vi.fn(async (url: string | URL | Request) => {
      calls.push({ url: String(url) });
      if (calls.length === 1) {
        return jsonResponse({
          enabled: true,
          mode: "otlp",
          otlp_endpoint: "https://otel.example",
        });
      }
      // Simulate gateway returning no CORS headers — fetch rejects.
      throw new TypeError("Failed to fetch");
    });
    await initBrowserErrorReporting("test", fetchMock as typeof fetch);
    // Fire a torrent of window.error events. Without the cooldown +
    // in-flight guard, each transport failure would refire window.error
    // and the loop would consume the queue at <100ms intervals.
    for (let i = 0; i < 20; i += 1) {
      window.dispatchEvent(new ErrorEvent("error", { message: `boom-${i}` }));
    }
    await tick();
    await tick();
    // 1 config fetch + at most 1 reporter attempt (cooldown holds the rest).
    expect(fetchMock.mock.calls.length).toBeLessThanOrEqual(2);
  });

  it("re-entry into initBrowserErrorReporting is a no-op (listener attaches once)", async () => {
    stubBeacon(false);
    const fetchMock = vi.fn(async () => jsonResponse({ enabled: false, mode: "disabled" }));
    await initBrowserErrorReporting("test", fetchMock as typeof fetch);
    await initBrowserErrorReporting("test", fetchMock as typeof fetch);
    await initBrowserErrorReporting("test", fetchMock as typeof fetch);
    // Config endpoint hit exactly once across the three mounts — emulates
    // the navigation pattern that previously caused per-mount refetch.
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});

function jsonResponse(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
}

function tick(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

function stubBeacon(value: boolean) {
  Object.defineProperty(navigator, "sendBeacon", {
    configurable: true,
    value: vi.fn(() => value),
  });
}
