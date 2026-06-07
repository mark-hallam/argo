/**
 * Idempotency-key pins for the onboarding first hire (FIX 2).
 *
 * Root cause being closed: `firstHire` / `confirmFirstHire` sent NO
 * `idempotency_key`, so a native form Enter-key double-fire (which ignores the
 * disabled submit button) could mint TWO workers. The fix threads a stable
 * UUID-v7 token — the SAME value on the initial submit and the post-review
 * confirm — so the gateway (once it honours the field) de-dupes the pair.
 *
 * These tests assert the CLIENT contract: two rapid `firstHire` calls send an
 * identical `idempotency_key`, and `clearOnboardingIdempotencyKey()` rotates
 * it for the next hire. (The re-entry guard on the page is pinned in the
 * first-hire page test.)
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Mock the gateway client so we capture POST bodies without the
// session/csrf plumbing. `apiGet` (the post-hire overview readback) resolves
// OK so `settleHiredResponse` returns a plain `hired` outcome.
const apiPostMock = vi.fn();
const apiGetMock = vi.fn();

vi.mock("@/lib/api/client", () => ({
  apiPost: (...args: unknown[]) => apiPostMock(...args),
  apiGet: (...args: unknown[]) => apiGetMock(...args),
  // `firstHire` imports ApiError for its instanceof checks; a minimal stand-in
  // with the fields the module reads is enough for the happy path.
  ApiError: class ApiError extends Error {
    status: number;
    code?: string;
    body?: unknown;
    constructor(status: number, message: string, code?: string, body?: unknown) {
      super(message);
      this.status = status;
      this.code = code;
      this.body = body;
    }
  },
}));

// Force the live (non-demo) path so the POST actually fires.
vi.mock("@/lib/skycot/demo-mode", () => ({
  isDemoMode: () => false,
  isDemoLive: () => false,
}));

import { firstHire } from "@/lib/onboarding/use-first-hire";
import {
  ONBOARDING_IDEMPOTENCY_STORAGE_KEY,
  clearOnboardingIdempotencyKey,
} from "@/lib/onboarding/onboarding-idempotency-key";

const UUID_V7_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

function installLocalStorage() {
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
      removeItem: (k: string) => data.delete(k),
      setItem: (k: string, v: string) => data.set(k, String(v)),
    },
  });
}

function lastPostBody(): Record<string, unknown> {
  const call = apiPostMock.mock.calls.at(-1);
  return (call?.[1] ?? {}) as Record<string, unknown>;
}

beforeEach(() => {
  installLocalStorage();
  apiPostMock.mockReset();
  apiGetMock.mockReset();
  // Default: gateway accepts the hire (201-shape) and overview reads back OK.
  apiPostMock.mockResolvedValue({ worker_id: "wkr_1", worker_name: "Maya" });
  apiGetMock.mockResolvedValue({ ok: true });
});

afterEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe("onboarding first-hire idempotency key (FIX 2)", () => {
  const req = {
    name: "Maya",
    description: "marketing intern who drafts blog posts",
    verticalId: "marketing-intern",
    tenantId: "tenant_1",
  };

  it("sends an idempotency_key shaped as a UUID v7", async () => {
    await firstHire(req);
    const key = lastPostBody().idempotency_key;
    expect(typeof key).toBe("string");
    expect(key as string).toMatch(UUID_V7_PATTERN);
  });

  it("two rapid first-hire calls send the SAME idempotency_key (double-fire de-dupe)", async () => {
    await Promise.all([firstHire(req), firstHire(req)]);

    const keys = apiPostMock.mock.calls.map(
      (c) => (c[1] as Record<string, unknown>).idempotency_key,
    );
    expect(keys).toHaveLength(2);
    expect(keys[0]).toBeDefined();
    expect(keys[0]).toBe(keys[1]);
  });

  it("clearing the key rotates it for the next hire", async () => {
    await firstHire(req);
    const firstKey = lastPostBody().idempotency_key;
    expect(window.localStorage.getItem(ONBOARDING_IDEMPOTENCY_STORAGE_KEY)).toBe(
      firstKey,
    );

    clearOnboardingIdempotencyKey();
    expect(
      window.localStorage.getItem(ONBOARDING_IDEMPOTENCY_STORAGE_KEY),
    ).toBeNull();

    await firstHire(req);
    const secondKey = lastPostBody().idempotency_key;
    expect(secondKey).toBeDefined();
    expect(secondKey).not.toBe(firstKey);
  });
});
