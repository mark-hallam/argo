/**
 * Unit-level test for the demo-mode short-circuit added in
 * `fix-onboarding-provision-demo-mode-bypass`.
 *
 * Asserts the three wizard mutations (`provision`, `validateProviderKey`,
 * `firstHire`) and the inline error scrubber behave correctly when
 * `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`:
 *
 * - No gateway call leaves the SPA (we'd see a fetch attempt to
 *   `http://127.0.0.1:1` and a NetworkError).
 * - Synthetic ids are deterministic per workspace name (provision) and
 *   stable enough to redirect cleanly (first-hire).
 * - The error scrubber never emits `X-Argo-Principal` even when the
 *   inbound error carries it.
 *
 * This is the CI gate. The Playwright spec
 * `tests/e2e/onboarding-demo-mode-step-2-continue.spec.ts` exercises
 * the full wizard walk but only runs when the dev server was started
 * with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` (the env var inlines at build).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const ORIGINAL_ENV = process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;

beforeEach(() => {
  process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = "1";
  vi.resetModules();
});

afterEach(() => {
  if (ORIGINAL_ENV === undefined) {
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;
  } else {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = ORIGINAL_ENV;
  }
});

describe("demo-mode short-circuit", () => {
  it("provision returns deterministic synthetic ids and writes sessionStorage", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch");
    const { provision, DEMO_TENANT_STORAGE_KEY, DEMO_WORKSPACE_STORAGE_KEY } =
      await import("@/lib/onboarding/use-provision");

    const first = await provision({
      workspaceName: "Acmeco Growth",
      companyName: "Acmeco Inc.",
    });
    const second = await provision({
      workspaceName: "Acmeco Growth",
      companyName: "Different Inc.",
    });

    expect(fetchSpy).not.toHaveBeenCalled();
    expect(first.tenantId).toBe("demo-tenant-acmeco-growth");
    expect(first.workspaceId).toBe("demo-workspace-acmeco-growth");
    expect(second.tenantId).toBe(first.tenantId);
    expect(window.sessionStorage.getItem(DEMO_TENANT_STORAGE_KEY)).toBe(first.tenantId);
    expect(window.sessionStorage.getItem(DEMO_WORKSPACE_STORAGE_KEY)).toBe(
      first.workspaceId,
    );

    fetchSpy.mockRestore();
  });

  it("validateProviderKey returns mockProvider=true with no gateway call (no $0.001 lie)", async () => {
    // F2 contract (post fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect):
    // demo mode short-circuits validation but marks the result as a mock
    // — `testCost: 0` not `0.001`, and `mockProvider: true` so the page
    // renders honest "Mock provider connected" copy.
    const fetchSpy = vi.spyOn(globalThis, "fetch");
    const { validateProviderKey } = await import("@/lib/onboarding/use-provider-key");

    const res = await validateProviderKey({ apiKey: "sk-ant-anything" });

    expect(fetchSpy).not.toHaveBeenCalled();
    expect(res.validated).toBe(true);
    expect(res.testCost).toBe(0);
    expect(res.mockProvider).toBe(true);

    fetchSpy.mockRestore();
  });

  it("firstHire synthesises a new worker keyed off the user's typed name (no sample-roster reuse)", async () => {
    // F1 contract (post fix-demo-first-hire-discards-prompt-and-
    // redirects-to-sample-worker): demo mode short-circuits hiring
    // and synthesises a *new* worker (fresh UUIDv7) keyed off the
    // user's typed name + description, then appends it to the demo
    // roster. The prior behaviour reused fixed sample-roster ids
    // (Mariah/Priya/Jamal/Nori) and silently discarded the user's
    // typed name — the camera saw the wrong name on /workers/<id>.
    const fetchSpy = vi.spyOn(globalThis, "fetch");
    const { firstHire } = await import("@/lib/onboarding/use-first-hire");
    const { _clearDemoRosterForTests, listDemoWorkers } = await import(
      "@/lib/api/demo-roster-store"
    );
    _clearDemoRosterForTests();

    // Demo mode always hires synchronously (never `needs_review`), so narrow
    // each outcome to the `hired` variant before reading the worker fields.
    type Hired = Extract<
      Awaited<ReturnType<typeof firstHire>>,
      { kind: "hired" }
    >;
    const assertHired = (outcome: Awaited<ReturnType<typeof firstHire>>): Hired => {
      expect(outcome.kind).toBe("hired");
      if (outcome.kind !== "hired") throw new Error("expected hired outcome");
      return outcome;
    };

    const marketing = assertHired(
      await firstHire({
        name: "Maya",
        description: "a marketing intern named Maya",
        verticalId: "marketing-intern",
        tenantId: null,
      }),
    );
    const recruiter = assertHired(
      await firstHire({
        name: "Penny",
        description: "a recruiter named Penny",
        verticalId: "recruiter",
        tenantId: null,
      }),
    );
    const unknown = assertHired(
      await firstHire({
        name: "Ada",
        description: "a brand-new role we don't have a sample for",
        verticalId: "made-up-vertical",
        tenantId: null,
      }),
    );

    expect(fetchSpy).not.toHaveBeenCalled();
    // Returned name matches the user's typed input — not the
    // sample-roster persona name.
    expect(marketing.workerName).toBe("Maya");
    expect(recruiter.workerName).toBe("Penny");
    expect(unknown.workerName).toBe("Ada");
    // Worker ids are fresh UUIDv7s — not the canonical sample-roster
    // ids and not `demo-worker-*` synthetics.
    const uuidV7 =
      /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
    expect(marketing.workerId).toMatch(uuidV7);
    expect(recruiter.workerId).toMatch(uuidV7);
    expect(unknown.workerId).toMatch(uuidV7);
    expect(marketing.workerId).not.toBe("019e6785-928f-7281-bf8d-e062bc277478");
    expect(recruiter.workerId).not.toBe("019e6785-928d-78b1-a6b2-136d9ed263ce");
    expect(unknown.workerId).not.toBe("019e6785-928a-7270-a766-f945b6cd7161");
    expect(marketing.workerId).not.toMatch(/^demo-worker-/);
    expect(marketing.pending).toBe(false);
    // Each hire is persisted to the session-local roster so /team
    // and /workers/<id>/overview can resolve it.
    expect(listDemoWorkers()).toHaveLength(3);

    fetchSpy.mockRestore();
  });
});

describe("scrubOnboardingErrorMessage", () => {
  it("replaces X-Argo-Principal leaks with friendly demo-mode copy", async () => {
    const { scrubOnboardingErrorMessage } = await import(
      "@/app/(setup)/onboarding/_scrub-error"
    );
    const result = scrubOnboardingErrorMessage(
      new Error("X-Argo-Principal header must carry a verified human:<email> value"),
    );
    expect(result.wasProtocolLeak).toBe(true);
    expect(result.message).not.toContain("X-Argo-Principal");
    expect(result.message).not.toContain("principal");
  });

  it("replaces other internal-protocol substrings (principal, bearer)", async () => {
    const { scrubOnboardingErrorMessage } = await import(
      "@/app/(setup)/onboarding/_scrub-error"
    );
    expect(scrubOnboardingErrorMessage(new Error("missing principal")).wasProtocolLeak).toBe(
      true,
    );
    expect(
      scrubOnboardingErrorMessage(new Error("bearer token required")).wasProtocolLeak,
    ).toBe(true);
    expect(
      scrubOnboardingErrorMessage(new Error("authorization header missing")).wasProtocolLeak,
    ).toBe(true);
  });

  it("passes through benign messages unchanged", async () => {
    const { scrubOnboardingErrorMessage } = await import(
      "@/app/(setup)/onboarding/_scrub-error"
    );
    const result = scrubOnboardingErrorMessage(new Error("Workspace name is required"));
    expect(result.wasProtocolLeak).toBe(false);
    expect(result.message).toBe("Workspace name is required");
  });

  it("falls back to a generic message for non-Error input", async () => {
    const { scrubOnboardingErrorMessage } = await import(
      "@/app/(setup)/onboarding/_scrub-error"
    );
    const result = scrubOnboardingErrorMessage(null);
    expect(result.wasProtocolLeak).toBe(false);
    expect(result.message.length).toBeGreaterThan(0);
  });
});
