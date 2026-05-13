import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  CELEBRATIONS_STORAGE_KEY,
  CELEBRATION_TTL_MS,
  clearCelebration,
  getCelebrations,
  pruneExpired,
  setCelebration,
} from "./celebration-storage";

// Node 25 ships a built-in localStorage that shadows happy-dom's; replace
// with a deterministic in-memory shim per test (matches draft-store.test.ts).
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
  vi.restoreAllMocks();
});

afterEach(() => {
  window.localStorage.clear();
});

describe("celebration-storage", () => {
  it("round-trips a single celebration via setCelebration / getCelebrations", () => {
    const ok = setCelebration("wkr_01", { workerName: "Mariah" });
    expect(ok).toBe(true);
    const list = getCelebrations();
    expect(list).toHaveLength(1);
    expect(list[0].workerId).toBe("wkr_01");
    expect(list[0].workerName).toBe("Mariah");
    expect(list[0].inert).toBe(false);
    expect(typeof list[0].hiredAt).toBe("string");
    expect(typeof list[0].ttl).toBe("string");
  });

  it("drops entries whose ttl has passed", () => {
    setCelebration("wkr_old", { workerName: "Priya" });
    const raw = window.localStorage.getItem(CELEBRATIONS_STORAGE_KEY) as string;
    const parsed = JSON.parse(raw) as Array<Record<string, unknown>>;
    parsed[0].ttl = new Date(Date.now() - 60_000).toISOString();
    window.localStorage.setItem(CELEBRATIONS_STORAGE_KEY, JSON.stringify(parsed));

    const list = getCelebrations();
    expect(list).toHaveLength(0);
    // expiry also rewrites storage so the next read is cheap
    const after = window.localStorage.getItem(CELEBRATIONS_STORAGE_KEY);
    expect(after).toBe("[]");
  });

  it("pruneExpired removes only expired entries", () => {
    setCelebration("wkr_fresh", { workerName: "Mariah" });
    setCelebration("wkr_stale", { workerName: "Priya" });
    const raw = window.localStorage.getItem(CELEBRATIONS_STORAGE_KEY) as string;
    const parsed = JSON.parse(raw) as Array<Record<string, unknown>>;
    const stale = parsed.find((e) => e.workerId === "wkr_stale") as Record<string, unknown>;
    stale.ttl = new Date(Date.now() - 60_000).toISOString();
    window.localStorage.setItem(CELEBRATIONS_STORAGE_KEY, JSON.stringify(parsed));

    pruneExpired();
    const list = getCelebrations();
    expect(list).toHaveLength(1);
    expect(list[0].workerId).toBe("wkr_fresh");
  });

  it("queues multiple workers by hire timestamp (oldest first)", () => {
    const earlier = new Date(Date.now() - 30_000).toISOString();
    const later = new Date(Date.now() - 10_000).toISOString();
    setCelebration("wkr_a", { workerName: "Anna", hiredAt: later });
    setCelebration("wkr_b", { workerName: "Bea", hiredAt: earlier });
    const list = getCelebrations();
    expect(list.map((e) => e.workerId)).toEqual(["wkr_b", "wkr_a"]);
  });

  it("treats malformed payloads as empty, never throws", () => {
    window.localStorage.setItem(CELEBRATIONS_STORAGE_KEY, "not json{{");
    expect(getCelebrations()).toEqual([]);
    window.localStorage.setItem(CELEBRATIONS_STORAGE_KEY, JSON.stringify({ not: "an array" }));
    expect(getCelebrations()).toEqual([]);
    window.localStorage.setItem(
      CELEBRATIONS_STORAGE_KEY,
      JSON.stringify([{ workerId: 1, workerName: 2 }]),
    );
    expect(getCelebrations()).toEqual([]);
  });

  it("clearCelebration removes the target entry only", () => {
    setCelebration("wkr_a", { workerName: "Anna" });
    setCelebration("wkr_b", { workerName: "Bea" });
    clearCelebration("wkr_a");
    const list = getCelebrations();
    expect(list).toHaveLength(1);
    expect(list[0].workerId).toBe("wkr_b");
  });

  it("re-calling setCelebration for the same workerId replaces, doesn't duplicate", () => {
    setCelebration("wkr_a", { workerName: "Anna" });
    setCelebration("wkr_a", { workerName: "Anna", inert: true });
    const list = getCelebrations();
    expect(list).toHaveLength(1);
    expect(list[0].inert).toBe(true);
  });

  it("degrades gracefully when window.localStorage is absent (no-throw, returns empty)", () => {
    // Simulate a runtime where localStorage access throws (private mode).
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      get() {
        throw new Error("SecurityError");
      },
    });
    try {
      expect(getCelebrations()).toEqual([]);
      expect(setCelebration("wkr_x", { workerName: "X" })).toBe(false);
      expect(() => clearCelebration("wkr_x")).not.toThrow();
      expect(() => pruneExpired()).not.toThrow();
    } finally {
      // Restore an accessible localStorage so the afterEach cleanup
      // (which calls window.localStorage.clear()) doesn't re-throw.
      installMemoryStorage();
    }
  });

  it("stamps ttl ~24h in the future", () => {
    const before = Date.now();
    setCelebration("wkr_a", { workerName: "Anna" });
    const list = getCelebrations();
    const ttlMs = Date.parse(list[0].ttl);
    expect(ttlMs - before).toBeGreaterThanOrEqual(CELEBRATION_TTL_MS - 1_000);
    expect(ttlMs - before).toBeLessThanOrEqual(CELEBRATION_TTL_MS + 1_000);
  });
});
