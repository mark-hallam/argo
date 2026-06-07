import { beforeEach, describe, expect, it, vi } from "vitest";
import type { QueryClient } from "@tanstack/react-query";
import { purgeClientTenantState } from "../purge-client-state";

/** Minimal Map-backed Storage so the test never depends on the
 *  environment's flaky localStorage shim. */
function makeStorage(): Storage {
  const m = new Map<string, string>();
  return {
    get length() {
      return m.size;
    },
    key: (i: number) => Array.from(m.keys())[i] ?? null,
    getItem: (k: string) => (m.has(k) ? (m.get(k) as string) : null),
    setItem: (k: string, v: string) => {
      m.set(k, String(v));
    },
    removeItem: (k: string) => {
      m.delete(k);
    },
    clear: () => {
      m.clear();
    },
  } as Storage;
}

beforeEach(() => {
  Object.defineProperty(window, "localStorage", { value: makeStorage(), configurable: true });
  Object.defineProperty(window, "sessionStorage", { value: makeStorage(), configurable: true });
});

describe("purgeClientTenantState", () => {
  it("clears the React Query cache", () => {
    const clear = vi.fn();
    const client = { clear } as unknown as QueryClient;
    purgeClientTenantState(client);
    expect(clear).toHaveBeenCalledTimes(1);
  });

  it("removes every skycot: and skycot. namespaced key from local + session storage", () => {
    window.localStorage.setItem("skycot:onboarding-draft-v1", "{}");
    window.localStorage.setItem("skycot.elevated-grants", "[]");
    window.localStorage.setItem("skycot.genesisTenantId", "op-1");
    window.sessionStorage.setItem("skycot.demoRoster", "[]");
    const client = { clear: vi.fn() } as unknown as QueryClient;

    purgeClientTenantState(client);

    expect(window.localStorage.getItem("skycot:onboarding-draft-v1")).toBeNull();
    expect(window.localStorage.getItem("skycot.elevated-grants")).toBeNull();
    expect(window.localStorage.getItem("skycot.genesisTenantId")).toBeNull();
    expect(window.sessionStorage.getItem("skycot.demoRoster")).toBeNull();
  });

  it("leaves device-pref + non-skycot keys intact", () => {
    window.localStorage.setItem("skycot-color-mode", "dark"); // hyphen — device pref
    window.localStorage.setItem("_ga", "x");
    window.localStorage.setItem("skycot.elevated-grants", "[]"); // tenant — purged
    const client = { clear: vi.fn() } as unknown as QueryClient;

    purgeClientTenantState(client);

    expect(window.localStorage.getItem("skycot-color-mode")).toBe("dark");
    expect(window.localStorage.getItem("_ga")).toBe("x");
    expect(window.localStorage.getItem("skycot.elevated-grants")).toBeNull();
  });

  it("never throws when queryClient.clear() throws", () => {
    const client = {
      clear: () => {
        throw new Error("boom");
      },
    } as unknown as QueryClient;
    expect(() => purgeClientTenantState(client)).not.toThrow();
  });
});
