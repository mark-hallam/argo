import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  HIRE_IDEMPOTENCY_STORAGE_KEY,
  clearIdempotencyKey,
  getOrCreateIdempotencyKey,
  readIdempotencyKey,
  uuidV7,
} from "./idempotency-key";

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

describe("uuidV7", () => {
  it("matches the canonical 8-4-4-4-12 hex layout", () => {
    const id = uuidV7();
    expect(id).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    );
  });

  it("encodes version 7 in the third group", () => {
    for (let i = 0; i < 16; i += 1) {
      expect(uuidV7().charAt(14)).toBe("7");
    }
  });

  it("encodes the RFC 4122 variant 10 in the fourth group", () => {
    for (let i = 0; i < 16; i += 1) {
      const variantNibble = uuidV7().charAt(19);
      expect("89ab").toContain(variantNibble);
    }
  });

  it("produces distinct values across rapid successive calls", () => {
    const ids = new Set(Array.from({ length: 32 }, () => uuidV7()));
    expect(ids.size).toBe(32);
  });

  it("time-sorts: ids generated later are lexicographically >= earlier ones", async () => {
    const a = uuidV7();
    // Advance the clock by ≥1 ms so the 48-bit timestamp prefix
    // increments. Without the wait the random tail can flip ordering.
    await new Promise((r) => setTimeout(r, 2));
    const b = uuidV7();
    expect(b >= a).toBe(true);
  });
});

describe("getOrCreateIdempotencyKey", () => {
  it("generates + persists on first call", () => {
    const k = getOrCreateIdempotencyKey();
    expect(k).not.toBeNull();
    expect(window.localStorage.getItem(HIRE_IDEMPOTENCY_STORAGE_KEY)).toBe(k);
  });

  it("returns the same key on subsequent calls", () => {
    const a = getOrCreateIdempotencyKey();
    const b = getOrCreateIdempotencyKey();
    expect(b).toBe(a);
  });

  it("survives a simulated reload — readIdempotencyKey sees the persisted value", () => {
    const a = getOrCreateIdempotencyKey();
    // No clear between — simulates page reload (hook re-mounts but
    // localStorage carries over).
    expect(readIdempotencyKey()).toBe(a);
  });

  it("regenerates after clear()", () => {
    const a = getOrCreateIdempotencyKey();
    clearIdempotencyKey();
    expect(readIdempotencyKey()).toBeNull();
    const b = getOrCreateIdempotencyKey();
    expect(b).not.toBe(a);
  });
});
