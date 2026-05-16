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
