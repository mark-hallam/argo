import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  ONBOARDING_DRAFT_STORAGE_KEY,
  useOnboardingDraft,
} from "@/lib/onboarding/use-onboarding-draft";

function installMemoryStorage() {
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

beforeEach(() => {
  installMemoryStorage();
  useOnboardingDraft.getState().reset();
});

afterEach(() => {
  window.localStorage.clear();
});

describe("useOnboardingDraft store", () => {
  it("starts at step 1 with empty fields", () => {
    const s = useOnboardingDraft.getState();
    expect(s.step).toBe(1);
    expect(s.workspaceName).toBe("");
    expect(s.companyName).toBe("");
    expect(s.providerKeyConfigured).toBe(false);
    expect(s.providerSkipped).toBe(false);
    expect(s.verticalId).toBeNull();
    expect(s.firstHirePrompt).toBe("");
  });

  it("persists progress through the steps", () => {
    const s = useOnboardingDraft.getState();
    s.setStep(2);
    s.setWorkspace("Acme Growth", "Acme Inc.");
    s.setProvision("tenant-1", "ws-1");
    s.setProvider(true, false);
    s.setVertical("marketing-intern");
    s.setFirstHirePrompt("Maya, marketing intern");

    const after = useOnboardingDraft.getState();
    expect(after.step).toBe(2);
    expect(after.workspaceName).toBe("Acme Growth");
    expect(after.companyName).toBe("Acme Inc.");
    expect(after.tenantId).toBe("tenant-1");
    expect(after.workspaceId).toBe("ws-1");
    expect(after.providerKeyConfigured).toBe(true);
    expect(after.verticalId).toBe("marketing-intern");
    expect(after.firstHirePrompt).toBe("Maya, marketing intern");
  });

  it("writes the persisted snapshot under the v1 storage key", () => {
    useOnboardingDraft.getState().setWorkspace("Acme", "Acme Inc.");
    const raw = window.localStorage.getItem(ONBOARDING_DRAFT_STORAGE_KEY);
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw as string);
    expect(parsed.version).toBe(1);
    expect(parsed.state.workspaceName).toBe("Acme");
  });

  it("reset() clears progress", () => {
    const s = useOnboardingDraft.getState();
    s.setWorkspace("Acme", "Acme Inc.");
    s.setStep(3);
    s.reset();
    const after = useOnboardingDraft.getState();
    expect(after.step).toBe(1);
    expect(after.workspaceName).toBe("");
  });
});
