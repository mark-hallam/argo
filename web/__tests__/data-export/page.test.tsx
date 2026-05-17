import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import DataExportPage from "@/app/(app)/settings/data-export/page";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
}));

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function mockFetch(handler: (url: string, init?: RequestInit) => Promise<Response>) {
  vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
    const url = typeof input === "string" ? input : input.toString();
    return handler(url, init);
  });
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("DataExportPage", () => {
  beforeEach(() => {
    // Stub URL helpers and anchor click — happy-dom doesn't trigger downloads.
    URL.createObjectURL = vi.fn(() => "blob://stub");
    URL.revokeObjectURL = vi.fn();
  });

  it("renders the initial Request export CTA", async () => {
    mockFetch(async (url) => {
      if (url.includes("/api/auth/session")) return jsonResponse({ gatewayToken: "t" });
      return new Response("", { status: 404 });
    });
    render(wrap(<DataExportPage />));
    expect(await screen.findByTestId("data-export-request")).toBeInTheDocument();
  });

  it("ships the user through Request → Ready → Download", async () => {
    mockFetch(async (url, init) => {
      if (url.includes("/api/auth/session")) return jsonResponse({ gatewayToken: "t" });
      if (url.endsWith("/api/v1/me/export") && init?.method === "POST") {
        return jsonResponse(
          {
            job_id: "job-1",
            status: "ready",
            download_url: "/api/v1/me/export/download/abc123",
            size_bytes: 42,
            expires_at: 99,
          },
          202,
        );
      }
      if (url.includes("/api/v1/me/export/download/abc123")) {
        return jsonResponse({
          content_type: "application/gzip",
          filename: "my-skycot-export.tar.gz",
          size_bytes: 4,
          body_b64: btoa("tarz"),
        });
      }
      return new Response("not found", { status: 404 });
    });

    render(wrap(<DataExportPage />));
    const requestBtn = await screen.findByTestId("data-export-request");
    await userEvent.click(requestBtn);

    const downloadBtn = await screen.findByTestId("data-export-download");
    await userEvent.click(downloadBtn);
    await waitFor(() => {
      expect(URL.createObjectURL).toHaveBeenCalled();
    });
  });

  it("surfaces a rate-limit response", async () => {
    mockFetch(async (url, init) => {
      if (url.includes("/api/auth/session")) return jsonResponse({ gatewayToken: "t" });
      if (url.endsWith("/api/v1/me/export") && init?.method === "POST") {
        return jsonResponse(
          {
            error: {
              code: "rate_limited",
              message: "one data export per 24 hours",
              retry_after_secs: 86_400,
            },
          },
          429,
        );
      }
      return new Response("", { status: 404 });
    });
    render(wrap(<DataExportPage />));
    const requestBtn = await screen.findByTestId("data-export-request");
    await userEvent.click(requestBtn);
    expect(await screen.findByTestId("data-export-rate-limited")).toBeInTheDocument();
  });

  it("surfaces a generic error on 500", async () => {
    mockFetch(async (url, init) => {
      if (url.includes("/api/auth/session")) return jsonResponse({ gatewayToken: "t" });
      if (url.endsWith("/api/v1/me/export") && init?.method === "POST") {
        return new Response("boom", { status: 500 });
      }
      return new Response("", { status: 404 });
    });
    render(wrap(<DataExportPage />));
    const requestBtn = await screen.findByTestId("data-export-request");
    await userEvent.click(requestBtn);
    expect(await screen.findByTestId("data-export-error")).toBeInTheDocument();
  });
});
