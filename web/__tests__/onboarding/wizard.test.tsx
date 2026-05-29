import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import WelcomePage from "@/app/(setup)/onboarding/welcome/page";
import WorkspacePage from "@/app/(setup)/onboarding/workspace/page";
import ConnectProviderPage from "@/app/(setup)/onboarding/connect-provider/page";
import PickVerticalPage from "@/app/(setup)/onboarding/pick-vertical/page";
import FirstHirePage from "@/app/(setup)/onboarding/first-hire/page";

const pushMock = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, replace: vi.fn(), back: vi.fn() }),
  usePathname: () => "/onboarding/welcome",
}));

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
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

beforeEach(() => {
  installStorage();
  useOnboardingDraft.getState().reset();
  pushMock.mockClear();
  global.fetch = vi.fn();
});

afterEach(() => {
  window.localStorage.clear();
});

describe("Welcome step", () => {
  it("renders the headline and continues to /onboarding/workspace", async () => {
    const user = userEvent.setup();
    render(wrap(<WelcomePage />));
    expect(screen.getByTestId("wiz-shell-hero")).toHaveTextContent("Hire your first teammate.");
    await user.click(screen.getByTestId("onboarding-welcome-continue"));
    expect(pushMock).toHaveBeenCalledWith("/onboarding/workspace");
  });
});

describe("Workspace step", () => {
  it("requires both fields before submitting", async () => {
    const user = userEvent.setup();
    render(wrap(<WorkspacePage />));
    const submit = screen.getByTestId("onboarding-workspace-submit");
    expect(submit).toBeDisabled();
    await user.type(screen.getByTestId("workspace-name"), "Acme");
    expect(submit).toBeDisabled();
    await user.type(screen.getByTestId("company-name"), "Acme Inc.");
    expect(submit).not.toBeDisabled();
  });

  it("posts the provision request and advances on success", async () => {
    const user = userEvent.setup();
    global.fetch = vi.fn(async () =>
      Response.json({ tenant_id: "t-1", workspace_id: "ws-1" }),
    ) as unknown as typeof fetch;

    render(wrap(<WorkspacePage />));
    await user.type(screen.getByTestId("workspace-name"), "Acme");
    await user.type(screen.getByTestId("company-name"), "Acme Inc.");
    await user.click(screen.getByTestId("onboarding-workspace-submit"));

    await waitFor(() => expect(pushMock).toHaveBeenCalledWith("/onboarding/connect-provider"));
    const draft = useOnboardingDraft.getState();
    expect(draft.tenantId).toBe("t-1");
    expect(draft.workspaceId).toBe("ws-1");
  });

  it("surfaces a gateway error and stays on the page", async () => {
    const user = userEvent.setup();
    global.fetch = vi.fn(async () =>
      new Response(JSON.stringify({ error: { message: "tenant exists" } }), {
        status: 409,
        headers: { "content-type": "application/json" },
      }),
    ) as unknown as typeof fetch;

    render(wrap(<WorkspacePage />));
    await user.type(screen.getByTestId("workspace-name"), "Acme");
    await user.type(screen.getByTestId("company-name"), "Acme Inc.");
    await user.click(screen.getByTestId("onboarding-workspace-submit"));

    expect(await screen.findByTestId("onboarding-workspace-error")).toHaveTextContent("tenant exists");
    expect(pushMock).not.toHaveBeenCalledWith("/onboarding/connect-provider");
  });

  it("resumes prior values from the draft", async () => {
    useOnboardingDraft.getState().setWorkspace("Saved Workspace", "Saved Co");
    render(wrap(<WorkspacePage />));
    expect(screen.getByTestId("workspace-name")).toHaveValue("Saved Workspace");
    expect(screen.getByTestId("company-name")).toHaveValue("Saved Co");
  });
});

describe("Connect-provider step", () => {
  it("keeps the v1 provider scope Anthropic-only and preserves the mock skip warning", () => {
    render(wrap(<ConnectProviderPage />));

    expect(screen.getByLabelText("Anthropic API key")).toBeInTheDocument();
    expect(screen.queryByText(/OpenAI/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/Bedrock/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/Vertex/i)).not.toBeInTheDocument();
    expect(screen.getByTestId("onboarding-provider-skip-warning")).toHaveTextContent(
      /Skipping uses a mock provider/i,
    );
  });

  it("rejects keys that don't start with sk-ant-", async () => {
    const user = userEvent.setup();
    render(wrap(<ConnectProviderPage />));
    await user.type(screen.getByTestId("provider-key"), "wrong-prefix");
    await user.click(screen.getByTestId("onboarding-provider-submit"));
    expect(await screen.findByTestId("onboarding-provider-error")).toHaveTextContent(
      "sk-ant-",
    );
  });

  it("skip-for-now advances without calling the gateway", async () => {
    const user = userEvent.setup();
    render(wrap(<ConnectProviderPage />));
    await user.click(screen.getByTestId("onboarding-provider-skip"));
    expect(pushMock).toHaveBeenCalledWith("/onboarding/pick-vertical");
    expect(useOnboardingDraft.getState().providerSkipped).toBe(true);
  });

  it("validates a real key and advances", async () => {
    const user = userEvent.setup();
    global.fetch = vi.fn(async () => Response.json({ validated: true, test_cost: 0.001 })) as unknown as typeof fetch;
    render(wrap(<ConnectProviderPage />));
    await user.type(screen.getByTestId("provider-key"), "sk-ant-test-123");
    await user.click(screen.getByTestId("onboarding-provider-submit"));
    await waitFor(() =>
      expect(pushMock).toHaveBeenCalledWith("/onboarding/pick-vertical"),
    );
    expect(useOnboardingDraft.getState().providerKeyConfigured).toBe(true);
  });
});

describe("Pick-vertical step", () => {
  it("renders all five templates and persists the choice", async () => {
    const user = userEvent.setup();
    global.fetch = vi.fn(async () => Response.json({ ok: true })) as unknown as typeof fetch;
    render(wrap(<PickVerticalPage />));
    // The page now sources cards from `useVerticals()`, which fills the
    // grid from an async fetch effect (falling through to SEED_VERTICALS
    // on failure) rather than a synchronous seed array — so the first
    // tile only appears after the effect resolves.
    expect(await screen.findByTestId("vertical-gtm-engineer")).toBeInTheDocument();
    expect(screen.getByTestId("vertical-marketing-intern")).toBeInTheDocument();
    expect(screen.getByTestId("vertical-sales-sdr")).toBeInTheDocument();
    expect(screen.getByTestId("vertical-customer-support")).toBeInTheDocument();
    expect(screen.getByTestId("vertical-finance-ar")).toBeInTheDocument();

    await user.click(screen.getByTestId("vertical-marketing-intern"));
    // Non-demo install now lands on an InstallReceipt confirmation
    // (showing what got installed) rather than navigating straight to
    // first-hire; its Continue button performs the route.
    await user.click(
      await screen.findByTestId("onboarding-vertical-receipt-continue"),
    );
    await waitFor(() => expect(pushMock).toHaveBeenCalledWith("/onboarding/first-hire"));
    expect(useOnboardingDraft.getState().verticalId).toBe("marketing-intern");
  });
});

describe("First-hire step", () => {
  it("requires both a name and a non-empty description", async () => {
    const user = userEvent.setup();
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));
    const submit = screen.getByTestId("onboarding-first-hire-submit");
    expect(submit).toBeDisabled();
    await user.type(screen.getByTestId("first-hire-name"), "Maya");
    expect(submit).toBeDisabled();
    await user.type(screen.getByTestId("first-hire-prompt"), "marketing intern");
    expect(submit).not.toBeDisabled();
  });

  it("calls Genesis and redirects through the /onboarding/done celebration", async () => {
    const user = userEvent.setup();
    useOnboardingDraft.getState().setVertical("marketing-intern");
    global.fetch = vi.fn(async () =>
      Response.json({ worker_id: "wkr_maya_1", worker_name: "Maya" }),
    ) as unknown as typeof fetch;

    render(wrap(<FirstHirePage />));
    await user.type(screen.getByTestId("first-hire-name"), "Maya");
    await user.type(
      screen.getByTestId("first-hire-prompt"),
      "marketing intern who drafts blog posts",
    );
    await user.click(screen.getByTestId("onboarding-first-hire-submit"));
    // The wizard now routes through `/onboarding/done` so the
    // "meet your first teammate" celebration beat lands before the
    // worker profile. The done page reads `?worker=` and `?name=`
    // from the URL and auto-advances to `/workers/<id>` after 6s.
    await waitFor(() =>
      expect(pushMock).toHaveBeenCalledWith(
        "/onboarding/done?worker=wkr_maya_1&name=Maya",
      ),
    );
  });

  it("blocks submit when no vertical has been chosen", async () => {
    const user = userEvent.setup();
    render(wrap(<FirstHirePage />));
    await user.type(screen.getByTestId("first-hire-name"), "Maya");
    await user.type(screen.getByTestId("first-hire-prompt"), "Someone");
    await user.click(screen.getByTestId("onboarding-first-hire-submit"));
    expect(await screen.findByTestId("onboarding-first-hire-error")).toHaveTextContent(
      "template",
    );
  });
});
