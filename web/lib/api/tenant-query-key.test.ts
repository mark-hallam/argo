import { afterEach, describe, expect, it } from "vitest";
import {
  ANON_TENANT,
  getActiveTenantId,
  setActiveTenantId,
  tenantKey,
  tenantQueryKeyHashFn,
} from "./tenant-query-key";
import { clearTenantOverride, setTenantOverride } from "./tenant";

afterEach(() => {
  setActiveTenantId(null);
  clearTenantOverride();
});

describe("tenantKey", () => {
  it("scopes the same logical query to different keys per tenant", () => {
    setActiveTenantId("tenant_a");
    const a = tenantKey("skills");
    setActiveTenantId("tenant_b");
    const b = tenantKey("skills");

    expect(a).toEqual(["t", "tenant_a", "skills"]);
    expect(b).toEqual(["t", "tenant_b", "skills"]);
    expect(a).not.toEqual(b);
  });

  it("is stable for the same tenant + parts", () => {
    setActiveTenantId("tenant_a");
    expect(tenantKey("worker-memory", "w1")).toEqual(
      tenantKey("worker-memory", "w1"),
    );
  });

  it("falls back to the anon namespace when no tenant is resolved", () => {
    setActiveTenantId(null);
    expect(getActiveTenantId()).toBe(ANON_TENANT);
    expect(tenantKey("team")).toEqual(["t", ANON_TENANT, "team"]);
  });

  it("lets a workspace override take precedence over the session tenant", () => {
    setActiveTenantId("tenant_session");
    setTenantOverride("tenant_pinned");
    expect(getActiveTenantId()).toBe("tenant_pinned");
    expect(tenantKey("approvals")).toEqual(["t", "tenant_pinned", "approvals"]);
  });
});

describe("tenantQueryKeyHashFn", () => {
  it("hashes a bare global key differently per tenant (no cross-tenant alias)", () => {
    setActiveTenantId("tenant_a");
    const hashA = tenantQueryKeyHashFn(["skills"]);
    setActiveTenantId("tenant_b");
    const hashB = tenantQueryKeyHashFn(["skills"]);

    expect(hashA).not.toBe(hashB);
    expect(hashA).toContain("tenant_a");
    expect(hashB).toContain("tenant_b");
  });

  it("is stable for the same tenant + key (cache hits work)", () => {
    setActiveTenantId("tenant_a");
    expect(tenantQueryKeyHashFn(["team"])).toBe(tenantQueryKeyHashFn(["team"]));
  });

  it("does not double-scope an already tenant-scoped key", () => {
    setActiveTenantId("tenant_a");
    // A key built via tenantKey() is already ["t", tenant, ...]; hashing it
    // must not prepend a second tenant segment.
    const built = tenantKey("worker-memory", "w1");
    setActiveTenantId("tenant_b");
    // Re-hashing the explicitly-scoped key keeps its original tenant_a scope
    // regardless of the now-active tenant_b.
    expect(tenantQueryKeyHashFn(built)).toBe(
      JSON.stringify(["t", "tenant_a", "worker-memory", "w1"]),
    );
  });

  it("stable-stringifies object key parts (sorted keys) like the default hash", () => {
    setActiveTenantId("tenant_a");
    const h1 = tenantQueryKeyHashFn(["q", { b: 2, a: 1 }]);
    const h2 = tenantQueryKeyHashFn(["q", { a: 1, b: 2 }]);
    expect(h1).toBe(h2);
  });

  it("anon and a real tenant never collide for the same key", () => {
    setActiveTenantId(null);
    const anon = tenantQueryKeyHashFn(["audit"]);
    setActiveTenantId("tenant_a");
    const real = tenantQueryKeyHashFn(["audit"]);
    expect(anon).not.toBe(real);
  });
});
