import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ElevatedGrant } from "@/lib/redaction/types";

// Mutable session the mocked useSkycotSession reads — lets a test flip
// the signed-in identity between issuing and reading a grant.
const session = { email: null as string | null, tenantId: null as string | null };

vi.mock("@/lib/skycot/use-session", () => ({
  useSkycotSession: () => ({
    status: "authenticated" as const,
    isDemoMode: false,
    email: session.email,
    displayName: "Tester",
    tenantId: session.tenantId,
    workspaceLabel: "WS",
    role: "operator",
  }),
}));

import {
  grantMatchesBinding,
  liveGrantsForSession,
  useElevatedViewGrants,
} from "./use-elevated-view-grant";

// Node 25 ships a built-in localStorage that shadows happy-dom's; replace
// with a deterministic in-memory shim per test.
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

function makeGrant(over: Partial<ElevatedGrant> = {}): ElevatedGrant {
  return {
    grantedAt: new Date().toISOString(),
    expiresAt: new Date(Date.now() + 60 * 60_000).toISOString(),
    categories: ["unredacted-email"],
    reason: "audit review",
    requestId: "req-1",
    principal: "a@example.com",
    tenantId: "tenant_a",
    ...over,
  };
}

beforeEach(() => {
  installMemoryStorage();
  session.email = "a@example.com";
  session.tenantId = "tenant_a";
});

afterEach(() => {
  window.localStorage.clear();
  vi.clearAllMocks();
});

describe("grantMatchesBinding (pure)", () => {
  it("matches when principal AND tenant both match", () => {
    expect(
      grantMatchesBinding(makeGrant(), {
        principal: "a@example.com",
        tenantId: "tenant_a",
      }),
    ).toBe(true);
  });

  it("does NOT match a different principal (grant A under session B)", () => {
    expect(
      grantMatchesBinding(makeGrant({ principal: "a@example.com" }), {
        principal: "b@example.com",
        tenantId: "tenant_a",
      }),
    ).toBe(false);
  });

  it("does NOT match a different tenant (workspace A under workspace B)", () => {
    expect(
      grantMatchesBinding(makeGrant({ tenantId: "tenant_a" }), {
        principal: "a@example.com",
        tenantId: "tenant_b",
      }),
    ).toBe(false);
  });

  it("legacy grant with no principal/tenant never matches a real session", () => {
    expect(
      grantMatchesBinding(makeGrant({ principal: undefined, tenantId: undefined }), {
        principal: "a@example.com",
        tenantId: "tenant_a",
      }),
    ).toBe(false);
  });

  it("a null session identity matches nothing", () => {
    expect(
      grantMatchesBinding(makeGrant(), { principal: null, tenantId: null }),
    ).toBe(false);
  });
});

describe("liveGrantsForSession (pure)", () => {
  const now = Date.now();

  it("keeps only unexpired + same-identity grants", () => {
    const mine = makeGrant({ requestId: "mine" });
    const otherUser = makeGrant({ requestId: "other-user", principal: "b@example.com" });
    const expired = makeGrant({
      requestId: "expired",
      expiresAt: new Date(now - 1000).toISOString(),
    });
    const out = liveGrantsForSession([mine, otherUser, expired], {
      principal: "a@example.com",
      tenantId: "tenant_a",
    }, now);
    expect(out.map((g) => g.requestId)).toEqual(["mine"]);
  });
});

describe("useElevatedViewGrants — bind-on-read", () => {
  it("a grant minted under principal A is not active when the session is principal B", () => {
    // Issue under A.
    const a = renderHook(() => useElevatedViewGrants());
    act(() => {
      a.result.current.issueGrant({
        categories: ["unredacted-email"],
        reason: "review",
        duration: "1h",
        requestId: "grant-A",
      });
    });
    expect(a.result.current.activeGrants).toHaveLength(1);
    expect(a.result.current.activeGrants[0].principal).toBe("a@example.com");

    // Switch identity to B and re-read (grant bytes still in localStorage).
    session.email = "b@example.com";
    session.tenantId = "tenant_b";
    const b = renderHook(() => useElevatedViewGrants());
    expect(b.result.current.activeGrants).toHaveLength(0);

    // The grant is still persisted — proving it's filtered on READ, not deleted.
    expect(window.localStorage.getItem("skycot.elevated-grants")).toContain("grant-A");
  });

  it("the same identity still sees its own grant (cache/grant still works)", () => {
    const { result } = renderHook(() => useElevatedViewGrants());
    act(() => {
      result.current.issueGrant({
        categories: ["unredacted-phone"],
        reason: "review",
        duration: "4h",
      });
    });
    expect(result.current.activeGrants).toHaveLength(1);
    expect(result.current.activeGrants[0].tenantId).toBe("tenant_a");
  });

  it("a different tenant for the same user is also inert (workspace switch)", () => {
    const a = renderHook(() => useElevatedViewGrants());
    act(() => {
      a.result.current.issueGrant({
        categories: ["unredacted-email"],
        reason: "review",
        duration: "1h",
      });
    });
    expect(a.result.current.activeGrants).toHaveLength(1);

    session.tenantId = "tenant_b"; // same email, different workspace
    const b = renderHook(() => useElevatedViewGrants());
    expect(b.result.current.activeGrants).toHaveLength(0);
  });
});
