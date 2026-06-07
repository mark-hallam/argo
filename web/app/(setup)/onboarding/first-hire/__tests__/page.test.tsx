/**
 * Acceptance test for
 * `fix-onboarding-first-hire-add-explicit-name-field-honest-parse`.
 *
 * Pins the F13 fix: the first-hire wizard now has a dedicated `name`
 * input alongside the description textarea. The page must:
 *
 *  1. Render both a name input and a description textarea.
 *  2. Disable submit until both inputs are non-empty.
 *  3. POST `{name, description, vertical_id}` to the gateway.
 *  4. Drop the "give them a name" copy from the description label,
 *     since name now has its own input.
 *  5. Persist firstHireName + firstHirePrompt across remounts via the
 *     Zustand draft store (re-pin from the prior persistence test).
 */

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import FirstHirePage from "../page";

const pushMock = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, replace: vi.fn(), back: vi.fn() }),
  usePathname: () => "/onboarding/first-hire",
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
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
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
  vi.restoreAllMocks();
});

describe("OnboardingFirstHirePage — explicit name field (F13)", () => {
  it("renders both a name input and a description textarea", () => {
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));
    expect(screen.getByTestId("first-hire-name")).toBeInTheDocument();
    expect(screen.getByTestId("first-hire-prompt")).toBeInTheDocument();
  });

  it("disables submit until both name and description are non-empty", async () => {
    const user = userEvent.setup();
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));
    const submit = screen.getByTestId("onboarding-first-hire-submit");
    expect(submit).toBeDisabled();
    await user.type(screen.getByTestId("first-hire-name"), "Maya");
    expect(submit).toBeDisabled();
    await user.type(
      screen.getByTestId("first-hire-prompt"),
      "marketing intern who drafts blog posts",
    );
    expect(submit).not.toBeDisabled();
  });

  it("posts {name, description, vertical_id} as separate fields", async () => {
    const user = userEvent.setup();
    useOnboardingDraft.getState().setVertical("marketing-intern");
    const fetchMock = vi.fn(async () =>
      Response.json({ worker_id: "wkr_maya_1", worker_name: "Maya" }),
    ) as unknown as typeof fetch;
    global.fetch = fetchMock;

    render(wrap(<FirstHirePage />));
    await user.type(screen.getByTestId("first-hire-name"), "Maya");
    await user.type(
      screen.getByTestId("first-hire-prompt"),
      "drafts blog posts and runs weekly competitive scans",
    );
    await user.click(screen.getByTestId("onboarding-first-hire-submit"));

    // Non-demo hire now routes through the /onboarding/done celebration
    // (which auto-advances to /workers/<id> after 6s) rather than jumping
    // straight to the worker profile; the worker id + name ride in the
    // URL since the draft is reset before the redirect.
    await waitFor(() =>
      expect(pushMock).toHaveBeenCalledWith(
        "/onboarding/done?worker=wkr_maya_1&name=Maya",
      ),
    );

    const firstHireCall = (
      fetchMock as unknown as { mock: { calls: unknown[][] } }
    ).mock.calls.find((call) => {
      const url = call[0];
      return (
        typeof url === "string" &&
        url.includes("/api/v1/onboarding/first-hire")
      );
    });
    expect(firstHireCall).toBeDefined();
    const init = firstHireCall![1] as RequestInit;
    const body = JSON.parse(init.body as string) as Record<string, unknown>;
    expect(body.name).toBe("Maya");
    expect(body.description).toBe(
      "drafts blog posts and runs weekly competitive scans",
    );
    expect(body.vertical_id).toBe("marketing-intern");
  });

  it("description copy no longer claims to accept a name", () => {
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));
    // The hero subcopy used to say "Give them a name and a role". With
    // the dedicated name input the description label is for role +
    // tasks only — the "name" claim is gone from the surrounding copy.
    expect(document.body.textContent ?? "").not.toMatch(
      /give them a.*name/i,
    );
  });

  it("hydrates name + description from the draft store on mount", () => {
    useOnboardingDraft.getState().setVertical("marketing-intern");
    useOnboardingDraft.getState().setFirstHireName("Maya");
    useOnboardingDraft
      .getState()
      .setFirstHirePrompt("Half-typed prompt from a prior visit");

    render(wrap(<FirstHirePage />));
    expect(screen.getByTestId("first-hire-name")).toHaveValue("Maya");
    expect(screen.getByTestId("first-hire-prompt")).toHaveValue(
      "Half-typed prompt from a prior visit",
    );
  });

  it("writes every name keystroke into the draft store (no debounce)", async () => {
    const user = userEvent.setup();
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));

    await user.type(screen.getByTestId("first-hire-name"), "Marcus");

    await waitFor(() => {
      expect(useOnboardingDraft.getState().firstHireName).toBe("Marcus");
    });
  });
});

// ---------------------------------------------------------------------------
// DEC-025 #1 — human-review-before-activation on the onboarding first hire.
//
// The gateway runs Genesis on the first POST (no `confirmed_contract_digest`),
// derives the truthful capability contract, and returns HTTP 422
// `genesis_gate_failed` holding the hire in `needs_review`. The wizard must
// render the SAME RoleContractReview screen the /genesis studio uses, then —
// only after the operator confirms — re-POST with the gateway-returned digest
// echoed back. A `blocked` gate is NOT confirmable.
// ---------------------------------------------------------------------------

function parseBody(init?: RequestInit): unknown {
  return typeof init?.body === "string" ? JSON.parse(init.body) : undefined;
}

const GATEWAY_DIGEST =
  "a90ed0d6f1c24e8b9a0d3f5e7c1b2a4d6e8f0a2c4e6b8d0f2a4c6e8b0d2f4a6c";

function gate422Body(state: "needs_review" | "blocked") {
  return {
    error: {
      code: "genesis_gate_failed",
      message:
        state === "blocked"
          ? "teammate did not clear the Genesis safety gate (blocked)"
          : "teammate did not clear the Genesis safety gate (needs_review)",
    },
    role_contract: {
      role_name: "Maya",
      does: "Drafts marketing copy and runs weekly competitive scans.",
      actions: [
        {
          action: "draft_blog_post",
          description: "Write a blog post draft for review.",
          requires_approval: false,
          risk: "low",
          executor: "human",
        },
        {
          action: "send_email",
          description: "Email the draft to the list.",
          requires_approval: true,
          risk: "medium",
          executor: "human",
        },
      ],
      approval_required: ["send_email"],
      autonomous: ["draft_blog_post"],
      escalation: "Hands off anything ambiguous to you.",
      will_not: ["Send without approval"],
      draft_integrations: [],
    },
    contract_digest: GATEWAY_DIGEST,
    gate: {
      state,
      eval_passed: 10,
      eval_total: 10,
      comprehension_passed: 5,
      comprehension_total: 5,
      safety_verdict: state === "blocked" ? "block" : "allow",
      reasons: ["role contract awaiting operator confirmation"],
    },
  };
}

interface FirstHirePostBody {
  confirmed_contract_digest?: string;
  [k: string]: unknown;
}

/**
 * Build a fetch mock that:
 *  - answers /api/auth/session + /api/v1/auth/csrf,
 *  - on the FIRST first-hire POST → 422 `gate422Body(gateState)`,
 *  - on a first-hire POST that CARRIES a `confirmed_contract_digest` →
 *    201 `{ worker_id, worker_name }` IFF it equals the gateway digest,
 *  - answers the overview read-back with 200.
 * Records every first-hire POST body in `firstHirePosts`.
 */
function installGateFetch(gateState: "needs_review" | "blocked") {
  const firstHirePosts: FirstHirePostBody[] = [];
  global.fetch = vi.fn(async (input, init) => {
    const url = String(input);
    const method = init?.method ?? "GET";
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
      const body = (parseBody(init) ?? {}) as FirstHirePostBody;
      firstHirePosts.push(body);
      if (typeof body.confirmed_contract_digest === "string") {
        // Confirm call: only succeeds when the echoed digest matches the
        // gateway-issued one — a forged/derived digest is hard-blocked.
        if (body.confirmed_contract_digest === GATEWAY_DIGEST) {
          return Response.json(
            { worker_id: "wkr_maya_confirmed", worker_name: "Maya" },
            { status: 201 },
          );
        }
        return Response.json(gate422Body("needs_review"), { status: 422 });
      }
      // Initial call (no digest): gateway holds it for review / blocks it.
      return Response.json(gate422Body(gateState), { status: 422 });
    }
    if (
      url.endsWith("/api/v1/workers/wkr_maya_confirmed/overview") &&
      method === "GET"
    ) {
      return Response.json({ ok: true }, { status: 200 });
    }
    throw new Error(`unexpected fetch ${method} ${url}`);
  }) as unknown as typeof fetch;
  return firstHirePosts;
}

async function fillAndSubmitForm() {
  const user = userEvent.setup();
  useOnboardingDraft.getState().setProvision("tenant_onboarding_1", "workspace_1");
  useOnboardingDraft.getState().setVertical("marketing-intern");
  render(wrap(<FirstHirePage />));
  await user.type(screen.getByTestId("first-hire-name"), "Maya");
  await user.type(
    screen.getByTestId("first-hire-prompt"),
    "marketing intern who drafts blog posts",
  );
  await user.click(screen.getByTestId("onboarding-first-hire-submit"));
  return user;
}

describe("OnboardingFirstHirePage — Genesis review→confirm gate", () => {
  it("renders the role-contract review when the gateway holds the hire in needs_review", async () => {
    installGateFetch("needs_review");

    await fillAndSubmitForm();

    // The review screen (RoleContractReview, reused from /genesis) appears,
    // listing the truthful bound actions the gateway returned.
    await screen.findByTestId(
      "genesis-role-contract-review",
      {},
      { timeout: 4_000 },
    );
    const actions = screen.getAllByTestId("genesis-contract-action");
    expect(actions.length).toBe(2);
    // It did NOT redirect — the buyer stays on the wizard to review.
    expect(pushMock).not.toHaveBeenCalled();
  });

  it("confirm echoes the gateway-returned digest on the second POST", async () => {
    const posts = installGateFetch("needs_review");

    const user = await fillAndSubmitForm();

    await screen.findByTestId(
      "genesis-role-contract-review",
      {},
      { timeout: 4_000 },
    );
    // Operator must tick the confirm checkbox before the button enables.
    await user.click(screen.getByTestId("genesis-confirm-checkbox"));
    await user.click(screen.getByTestId("genesis-confirm-hire-button"));

    // A SECOND first-hire POST fires, carrying the EXACT gateway digest.
    await waitFor(() => {
      expect(posts.length).toBe(2);
    });
    const confirmBody = posts[1];
    expect(confirmBody.confirmed_contract_digest).toBe(GATEWAY_DIGEST);
    // The initial call must NOT have carried a digest (no auto-confirm).
    expect(posts[0].confirmed_contract_digest).toBeUndefined();
    // And the confirmed hire redirects to the celebration page.
    await waitFor(
      () => {
        expect(pushMock).toHaveBeenCalledWith(
          "/onboarding/done?worker=wkr_maya_confirmed&name=Maya",
        );
      },
      { timeout: 4_000 },
    );
  });

  it("a blocked gate is not confirmable — surfaces an honest error, no second hire is possible", async () => {
    const posts = installGateFetch("blocked");

    await fillAndSubmitForm();

    // A hard `blocked` gate is NOT a confirmable review (SAFETY GUARDRAIL #3):
    // the wizard surfaces the gateway's honest block message in the error
    // banner and NEVER renders a confirmable review screen.
    const banner = await screen.findByTestId(
      "onboarding-first-hire-error",
      {},
      { timeout: 4_000 },
    );
    expect(banner).toHaveTextContent(/blocked/i);
    expect(screen.queryByTestId("onboarding-first-hire-review")).toBeNull();
    expect(screen.queryByTestId("genesis-role-contract-review")).toBeNull();

    // Only the initial (no-digest) POST was ever made — a blocked teammate can
    // never be confirmed, so no second POST and no digest was ever echoed.
    expect(posts.length).toBe(1);
    expect(
      posts.some((p) => typeof p.confirmed_contract_digest === "string"),
    ).toBe(false);
    expect(pushMock).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// FIX 3 — the needs_review screen is DERIVED from the mutation's cached data
// (`firstHire.data`), not a separate `useState`. Driving the flow inside
// <StrictMode> (which double-invokes renders + mount/unmount effects, the
// exact condition behind the dev "needs a 2nd click" quirk) must still land
// the review panel on the FIRST submit — no second click, no lost payload.
// ---------------------------------------------------------------------------
describe("OnboardingFirstHirePage — review is derived from query state (FIX 3)", () => {
  it("renders the review panel on the first submit even under StrictMode double-invoke", async () => {
    installGateFetch("needs_review");

    const user = userEvent.setup();
    useOnboardingDraft.getState().setProvision("tenant_onboarding_1", "workspace_1");
    useOnboardingDraft.getState().setVertical("marketing-intern");

    const client = new QueryClient({
      defaultOptions: {
        queries: { retry: false, gcTime: 0 },
        mutations: { retry: false },
      },
    });
    render(
      <StrictMode>
        <QueryClientProvider client={client}>
          <FirstHirePage />
        </QueryClientProvider>
      </StrictMode>,
    );

    await user.type(screen.getByTestId("first-hire-name"), "Maya");
    await user.type(
      screen.getByTestId("first-hire-prompt"),
      "marketing intern who drafts blog posts",
    );
    await user.click(screen.getByTestId("onboarding-first-hire-submit"));

    // The review screen appears from a single submit — proving the payload
    // is read off `firstHire.data` and not a `setReview` that a remount could
    // drop. The "Start over" affordance (now `firstHire.reset()`) is present.
    await screen.findByTestId(
      "genesis-role-contract-review",
      {},
      { timeout: 4_000 },
    );
    expect(
      screen.getByTestId("onboarding-first-hire-review-start-over"),
    ).toBeInTheDocument();
    expect(pushMock).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// FIX 1 — input validation on the first-hire form. Whitespace-only,
// zero-width-only, and over-long inputs are blocked BEFORE any POST.
// ---------------------------------------------------------------------------
describe("OnboardingFirstHirePage — input validation (FIX 1)", () => {
  function setInputs(name: string, prompt: string) {
    const nameInput = screen.getByTestId("first-hire-name") as HTMLInputElement;
    const promptInput = screen.getByTestId(
      "first-hire-prompt",
    ) as HTMLTextAreaElement;
    // Direct change events bypass the inputs' maxLength clamp so the SCHEMA
    // bound (the real defence) is what the over-long case exercises.
    fireEvent.change(nameInput, { target: { value: name } });
    fireEvent.change(promptInput, { target: { value: prompt } });
  }

  it("blocks a zero-width-only name (no POST, error shown)", async () => {
    const postSpy = vi.fn();
    global.fetch = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/api/v1/onboarding/first-hire")) postSpy();
      return Response.json({});
    }) as unknown as typeof fetch;

    useOnboardingDraft.getState().setProvision("tenant_1", "workspace_1");
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));

    setInputs("​", "a real description of the role");
    fireEvent.submit(screen.getByTestId("onboarding-first-hire-form"));

    await screen.findByTestId("onboarding-first-hire-error");
    expect(postSpy).not.toHaveBeenCalled();
  });

  it("blocks an over-long name (> 80 chars)", async () => {
    const postSpy = vi.fn();
    global.fetch = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/api/v1/onboarding/first-hire")) postSpy();
      return Response.json({});
    }) as unknown as typeof fetch;

    useOnboardingDraft.getState().setProvision("tenant_1", "workspace_1");
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));

    setInputs("n".repeat(81), "a real description of the role");
    fireEvent.submit(screen.getByTestId("onboarding-first-hire-form"));

    await screen.findByTestId("onboarding-first-hire-error");
    expect(postSpy).not.toHaveBeenCalled();
  });

  it("the name input + description textarea carry maxLength stops", () => {
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));
    expect(screen.getByTestId("first-hire-name")).toHaveAttribute(
      "maxlength",
      "80",
    );
    expect(screen.getByTestId("first-hire-prompt")).toHaveAttribute(
      "maxlength",
      "2000",
    );
  });
});

// ---------------------------------------------------------------------------
// FIX 2 — the form's native Enter-submit re-entry guard. A second submit
// fired while the first hire is in flight must NOT produce a second POST.
// ---------------------------------------------------------------------------
describe("OnboardingFirstHirePage — submit re-entry guard (FIX 2)", () => {
  it("a second form submit while the first hire is pending fires only one POST", async () => {
    const user = userEvent.setup();
    let resolveHire: ((value: Response) => void) | undefined;
    const firstHirePosts: unknown[] = [];

    global.fetch = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      const method = init?.method ?? "GET";
      if (url === "/api/auth/session") {
        return Response.json({
          gatewayToken: "t",
          principal: "human:onboarding@example.test",
        });
      }
      if (url.endsWith("/api/v1/auth/csrf")) {
        return Response.json({ token: "csrf" });
      }
      if (url.endsWith("/api/v1/onboarding/first-hire") && method === "POST") {
        firstHirePosts.push(parseBody(init));
        // Hang the FIRST hire POST so `firstHire.isPending` stays true while we
        // fire a second submit; the guard must drop the second one.
        return new Promise<Response>((resolve) => {
          resolveHire = resolve;
        });
      }
      if (url.endsWith("/overview")) return Response.json({ ok: true });
      throw new Error(`unexpected fetch ${method} ${url}`);
    }) as unknown as typeof fetch;

    useOnboardingDraft.getState().setProvision("tenant_1", "workspace_1");
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));

    await user.type(screen.getByTestId("first-hire-name"), "Maya");
    await user.type(
      screen.getByTestId("first-hire-prompt"),
      "marketing intern who drafts blog posts",
    );

    const form = screen.getByTestId("onboarding-first-hire-form");
    // First submit kicks off the (hanging) POST.
    fireEvent.submit(form);
    await waitFor(() => expect(firstHirePosts.length).toBe(1));
    // Native Enter-submit again while in-flight — the disabled button would
    // not have stopped this; the `if (firstHire.isPending) return` guard must.
    fireEvent.submit(form);
    fireEvent.submit(form);

    // Give any errant second POST a tick to land, then assert only one fired.
    await new Promise((r) => setTimeout(r, 50));
    expect(firstHirePosts.length).toBe(1);

    // Release the hanging request so the test doesn't leak a pending promise.
    resolveHire?.(
      Response.json({ worker_id: "wkr_maya", worker_name: "Maya" }, { status: 201 }),
    );
  });
});
