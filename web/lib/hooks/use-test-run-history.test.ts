import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  applyCap,
  MAX_ENTRIES,
  useTestRunHistory,
  type TestRunHistoryEntry,
} from "./use-test-run-history";

// Node 25 ships a built-in localStorage that shadows happy-dom's; replace
// with a deterministic in-memory shim per test
// (matches lib/hire/celebration-storage.test.ts).
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

function makeEntry(seq: number): TestRunHistoryEntry {
  return {
    id: `run-${seq}`,
    startedAt: seq * 1000,
    endedAt: seq * 1000 + 500,
    status: "complete",
    steps: [{ id: "s1", name: "Trigger", kind: "trigger" }],
    stepStates: { s1: { status: "success", durationMs: 100 } },
    summary: { ok: 1, fail: 0, durationMs: 500 },
  };
}

describe("useTestRunHistory — FIFO cap at 10", () => {
  beforeEach(() => {
    installMemoryStorage();
  });

  afterEach(() => {
    window.localStorage.clear();
  });

  it("evicts the oldest 2 after 12 records, keeping the 10 most-recent", () => {
    const { result } = renderHook(() => useTestRunHistory("support/triage"));

    act(() => {
      for (let i = 1; i <= 12; i += 1) {
        result.current.record(makeEntry(i));
      }
    });

    expect(result.current.entries).toHaveLength(MAX_ENTRIES);
    // Newest sits at index 0, oldest survivor at index 9.
    expect(result.current.entries[0].id).toBe("run-12");
    expect(result.current.entries[MAX_ENTRIES - 1].id).toBe("run-3");
    // Runs 1 and 2 were evicted.
    expect(result.current.entries.find((e) => e.id === "run-1")).toBeUndefined();
    expect(result.current.entries.find((e) => e.id === "run-2")).toBeUndefined();
  });

  it("rehydrates from localStorage when remounted under the same workflowId", () => {
    const { result, unmount } = renderHook(() =>
      useTestRunHistory("support/triage"),
    );
    act(() => {
      result.current.record(makeEntry(1));
      result.current.record(makeEntry(2));
    });
    unmount();
    const remount = renderHook(() => useTestRunHistory("support/triage"));
    expect(remount.result.current.entries.map((e) => e.id)).toEqual([
      "run-2",
      "run-1",
    ]);
  });

  it("isolates entries by workflowId", () => {
    const a = renderHook(() => useTestRunHistory("support/triage"));
    const b = renderHook(() => useTestRunHistory("billing/dispute"));
    act(() => {
      a.result.current.record(makeEntry(1));
    });
    expect(a.result.current.entries).toHaveLength(1);
    expect(b.result.current.entries).toHaveLength(0);
  });

  it("clear empties the entries and the storage backing", () => {
    const { result } = renderHook(() => useTestRunHistory("support/triage"));
    act(() => {
      result.current.record(makeEntry(1));
      result.current.clear();
    });
    expect(result.current.entries).toEqual([]);
    const stored = window.localStorage.getItem(
      "skycot:test-run-history:v1:support/triage",
    );
    expect(stored).toBe(JSON.stringify({ version: 1, entries: [] }));
  });
});

describe("applyCap (pure helper)", () => {
  it("returns the input unchanged when ≤ MAX_ENTRIES", () => {
    const entries = [makeEntry(1), makeEntry(2)];
    expect(applyCap(entries)).toEqual(entries);
  });

  it("drops the tail so length === MAX_ENTRIES", () => {
    const entries = Array.from({ length: 14 }, (_v, i) => makeEntry(i + 1));
    const capped = applyCap(entries);
    expect(capped).toHaveLength(MAX_ENTRIES);
    expect(capped[0].id).toBe("run-1");
    expect(capped[MAX_ENTRIES - 1].id).toBe(`run-${MAX_ENTRIES}`);
  });
});
