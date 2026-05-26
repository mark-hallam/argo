import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createElement, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import FirstHirePage from "@/app/(setup)/onboarding/first-hire/page";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";

const pushMock = vi.hoisted(() => vi.fn());

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, replace: vi.fn(), back: vi.fn() }),
  usePathname: () => "/onboarding/first-hire",
}));

interface CapturedFetch {
  body?: unknown;
  method: string;
  url: string;
}

function installStorage() {
  const data = new Map<string, string>();
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: {
      get length() {
        return data.size;
      },
      clear: () => data.clear(),
      getItem: (k: string) => (data.has(k) ? (data.get(k) as string) : null),
      key: (i: number) => Array.from(data.keys())[i] ?? null,
      removeItem: (k: string) => {
        data.delete(k);
      },
      setItem: (k: string, v: string) => {
        data.set(k, String(v));
      },
    },
  });
}

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: {
      mutations: { retry: false },
      queries: { gcTime: 0, retry: false },
    },
  });
  return createElement(QueryClientProvider, { client }, ui);
}

function parseBody(init?: RequestInit): unknown {
  return typeof init?.body === "string" ? JSON.parse(init.body) : undefined;
}

beforeEach(() => {
  installStorage();
  vi.stubEnv("NEXT_PUBLIC_GATEWAY_URL", "http://gw.test");
  pushMock.mockClear();
  useOnboardingDraft.getState().reset();
  useOnboardingDraft.getState().setProvision("tenant_onboarding_1", "workspace_1");
  useOnboardingDraft.getState().setVertical("marketing-intern");
});

afterEach(() => {
  window.localStorage.clear();
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

describe("first-hire read-back verification", () => {
  it("shows a wizard error and does not route when the created worker stays unreadable", async () => {
    const calls: CapturedFetch[] = [];
    global.fetch = vi.fn(async (input, init) => {
      const url = String(input);
      const method = init?.method ?? "GET";
      calls.push({ body: parseBody(init), method, url });

      if (url === "/api/auth/session") {
        return Response.json({
          gatewayToken: "test-token",
          principal: "human:onboarding@example.test",
        });
      }
      if (url.endsWith("/api/v1/auth/csrf")) {
        return Response.json({ token: "csrf-token" });
      }
      if (url.endsWith("/api/v1/onboarding/first-hire") && method === "POST") {
        return Response.json(
          { worker_id: "wkr_orphan", worker_name: "Maya" },
          { status: 201 },
        );
      }
      if (url.endsWith("/api/v1/workers/wkr_orphan/overview") && method === "GET") {
        return Response.json(
          { error: { code: "not_found", message: "worker not found" } },
          { status: 404 },
        );
      }
      if (url.endsWith("/api/v1/audit") && method === "POST") {
        return new Response(null, { status: 204 });
      }
      throw new Error(`unexpected fetch ${method} ${url}`);
    }) as unknown as typeof fetch;

    const user = userEvent.setup();
    render(wrap(createElement(FirstHirePage)));

    await user.type(
      screen.getByTestId("first-hire-prompt"),
      "Maya, marketing intern who drafts blog posts",
    );
    await user.click(screen.getByTestId("onboarding-first-hire-submit"));

    const banner = await screen.findByTestId(
      "onboarding-first-hire-error",
      {},
      { timeout: 4_000 },
    );
    expect(banner).toHaveTextContent(
      "Hiring succeeded but your teammate isn't visible yet. Try refreshing or contact support.",
    );
    expect(pushMock).not.toHaveBeenCalledWith("/workers/wkr_orphan");
    expect(useOnboardingDraft.getState().step).toBe(5);

    const verifyCalls = calls.filter((call) =>
      call.url.endsWith("/api/v1/workers/wkr_orphan/overview"),
    );
    expect(verifyCalls).toHaveLength(3);

    await waitFor(() => {
      const auditCall = calls.find(
        (call) => call.method === "POST" && call.url.endsWith("/api/v1/audit"),
      );
      expect(auditCall?.body).toMatchObject({
        action: "wizard.first_hire_verify_failed",
        metadata: {
          tenant_id: "tenant_onboarding_1",
          worker_id: "wkr_orphan",
        },
        tenant: "tenant_onboarding_1",
      });
    });
  });
});
