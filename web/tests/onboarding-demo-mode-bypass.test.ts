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

  it("validateProviderKey returns synthetic validated result with no gateway call", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch");
    const { validateProviderKey } = await import("@/lib/onboarding/use-provider-key");

    const res = await validateProviderKey({ apiKey: "sk-ant-anything" });

    expect(fetchSpy).not.toHaveBeenCalled();
    expect(res.validated).toBe(true);
    expect(res.testCost).toBe(0.001);

    fetchSpy.mockRestore();
  });

  it("firstHire returns a synthetic worker id keyed off the vertical", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch");
    const { firstHire } = await import("@/lib/onboarding/use-first-hire");

    const res = await firstHire({
      description: "a marketing intern named Maya",
      verticalId: "marketing-intern",
      tenantId: null,
    });

    expect(fetchSpy).not.toHaveBeenCalled();
    expect(res.workerId.startsWith("demo-worker-marketing-intern-")).toBe(true);
    expect(res.pending).toBe(false);
    expect(res.workerName.length).toBeGreaterThan(0);

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
