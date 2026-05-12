import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { HIRE_DRAFT_STORAGE_KEY } from "./draft-types";
import { useHireDraft } from "./use-hire-draft";

// See draft-store.test.ts for why we replace `window.localStorage` per
// test instead of relying on the runtime-provided implementation.
function installMemoryStorage(): Storage {
  const data = new Map<string, string>();
  const storage: Storage = {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (k) => (data.has(k) ? (data.get(k) as string) : null),
    key: (i) => Array.from(data.keys())[i] ?? null,
    removeItem: (k) => {
      data.delete(k);
    },
    setItem: (k, v) => {
      data.set(k, String(v));
    },
  };
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: storage,
  });
  return storage;
}

beforeEach(() => {
  installMemoryStorage();
});

afterEach(() => {
  window.localStorage.clear();
});

describe("useHireDraft", () => {
  it("hydrates state from a persisted draft on mount", async () => {
    window.localStorage.setItem(
      HIRE_DRAFT_STORAGE_KEY,
      JSON.stringify({ version: 1, step: 3, problem: "rehydrate me", ts: Date.now() }),
    );
    const { result } = renderHook(() => useHireDraft());
    expect(result.current.isLoading).toBe(true);
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.state?.step).toBe(3);
    expect(result.current.state?.problem).toBe("rehydrate me");
  });

  it("persists updates to localStorage", async () => {
    const { result } = renderHook(() => useHireDraft());
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    await act(async () => {
      await result.current.update({ step: 2, problem: "draft body" });
    });
    expect(result.current.state?.step).toBe(2);
    expect(result.current.state?.problem).toBe("draft body");
    const raw = window.localStorage.getItem(HIRE_DRAFT_STORAGE_KEY);
    expect(raw).not.toBeNull();
    expect(JSON.parse(raw as string).step).toBe(2);
  });

  it("clear() removes the entry and resets state", async () => {
    window.localStorage.setItem(
      HIRE_DRAFT_STORAGE_KEY,
      JSON.stringify({ version: 1, step: 4, ts: Date.now() }),
    );
    const { result } = renderHook(() => useHireDraft());
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.state?.step).toBe(4);
    await act(async () => {
      await result.current.clear();
    });
    expect(result.current.state).toBeNull();
    expect(window.localStorage.getItem(HIRE_DRAFT_STORAGE_KEY)).toBeNull();
  });

  it("merges patches shallowly so unrelated fields survive", async () => {
    const { result } = renderHook(() => useHireDraft());
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    await act(async () => {
      await result.current.update({ step: 2, problem: "kept" });
    });
    await act(async () => {
      await result.current.update({ step: 3 });
    });
    expect(result.current.state?.step).toBe(3);
    expect(result.current.state?.problem).toBe("kept");
  });
});
