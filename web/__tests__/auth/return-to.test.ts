import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  INTERNAL_PATH_PREFIXES,
  buildSigninUrlWithReturnTo,
  captureCurrentLocationAsReturnTo,
  consumeReturnToPending,
  parseReturnTo,
  safeReturnToOrHome,
  storeReturnToPending,
} from "@/lib/auth/return-to";

describe("safeReturnToOrHome — whitelist enforcement (load-bearing)", () => {
  it("returns '/' for null / undefined / empty", () => {
    expect(safeReturnToOrHome(null)).toBe("/");
    expect(safeReturnToOrHome(undefined)).toBe("/");
    expect(safeReturnToOrHome("")).toBe("/");
  });

  it("returns '/' for absolute http(s) URLs", () => {
    expect(safeReturnToOrHome("https://evil.com/")).toBe("/");
    expect(safeReturnToOrHome("http://evil.com/app/x")).toBe("/");
    expect(safeReturnToOrHome("HTTPS://EVIL.COM/")).toBe("/");
  });

  it("returns '/' for javascript: and data: schemes", () => {
    expect(safeReturnToOrHome("javascript:alert(1)")).toBe("/");
    expect(safeReturnToOrHome("data:text/html,<script>alert(1)</script>")).toBe("/");
    expect(safeReturnToOrHome("JaVaScRiPt:alert(1)")).toBe("/");
  });

  it("returns '/' for protocol-relative URLs", () => {
    expect(safeReturnToOrHome("//evil.com/app/x")).toBe("/");
    expect(safeReturnToOrHome("//evil.com")).toBe("/");
  });

  it("returns '/' for paths not starting with '/'", () => {
    expect(safeReturnToOrHome("app/dashboard")).toBe("/");
    expect(safeReturnToOrHome("evil")).toBe("/");
  });

  it("returns '/' for whitelisted-prefix-as-substring (not at start)", () => {
    expect(safeReturnToOrHome("/marketing/app/x")).toBe("/");
    expect(safeReturnToOrHome("/x/workflows/y")).toBe("/");
  });

  it("returns '/' for non-whitelisted /landing, /docs, /help, etc.", () => {
    expect(safeReturnToOrHome("/landing")).toBe("/");
    expect(safeReturnToOrHome("/docs/getting-started")).toBe("/");
    expect(safeReturnToOrHome("/help")).toBe("/");
    expect(safeReturnToOrHome("/auth/signin")).toBe("/");
  });

  it("returns '/' for the bare root '/'", () => {
    expect(safeReturnToOrHome("/")).toBe("/");
  });

  it("accepts each whitelisted prefix and round-trips query+hash", () => {
    for (const prefix of INTERNAL_PATH_PREFIXES) {
      const path = `${prefix}deep/page`;
      expect(safeReturnToOrHome(path)).toBe(path);
    }
    expect(safeReturnToOrHome("/app/workers?cohort=peta")).toBe(
      "/app/workers?cohort=peta",
    );
    expect(safeReturnToOrHome("/workflows/skycot/build#step3")).toBe(
      "/workflows/skycot/build#step3",
    );
    expect(safeReturnToOrHome("/approvals")).toBe("/approvals");
  });
});

describe("parseReturnTo", () => {
  it("returns null for null/undefined params", () => {
    expect(parseReturnTo(null)).toBeNull();
    expect(parseReturnTo(undefined)).toBeNull();
  });

  it("reads returnTo from URLSearchParams", () => {
    const params = new URLSearchParams("returnTo=%2Fapp%2Ffoo&x=1");
    expect(parseReturnTo(params)).toBe("/app/foo");
  });

  it("returns null when key is absent", () => {
    const params = new URLSearchParams("x=1");
    expect(parseReturnTo(params)).toBeNull();
  });

  it("works with the Next.js searchParams.get(...) shape", () => {
    const shim = { get: (n: string) => (n === "returnTo" ? "/audit" : null) };
    expect(parseReturnTo(shim)).toBe("/audit");
  });
});

describe("captureCurrentLocationAsReturnTo", () => {
  const originalWindow = globalThis.window;

  afterEach(() => {
    if (originalWindow === undefined) {
      // jsdom restores window between files; nothing to do.
      return;
    }
  });

  it("returns '/' under SSR (no window)", () => {
    const w = (globalThis as { window?: unknown }).window;
    (globalThis as { window?: unknown }).window = undefined;
    try {
      expect(captureCurrentLocationAsReturnTo()).toBe("/");
    } finally {
      (globalThis as { window?: unknown }).window = w;
    }
  });

  it("snapshots pathname + search in the browser", () => {
    Object.defineProperty(window, "location", {
      configurable: true,
      value: { pathname: "/app/workers", search: "?cohort=peta" },
    });
    expect(captureCurrentLocationAsReturnTo()).toBe("/app/workers?cohort=peta");
  });

  it("returns '/' when current path is itself an auth surface", () => {
    Object.defineProperty(window, "location", {
      configurable: true,
      value: { pathname: "/auth/signin", search: "" },
    });
    expect(captureCurrentLocationAsReturnTo()).toBe("/");
  });
});

describe("buildSigninUrlWithReturnTo", () => {
  it("returns bare /auth/signin for home, empty, and auth paths", () => {
    expect(buildSigninUrlWithReturnTo("/")).toBe("/auth/signin");
    expect(buildSigninUrlWithReturnTo("")).toBe("/auth/signin");
    expect(buildSigninUrlWithReturnTo("/auth/verify")).toBe("/auth/signin");
  });

  it("encodes the current path into the returnTo query", () => {
    expect(buildSigninUrlWithReturnTo("/app/workers?cohort=peta")).toBe(
      "/auth/signin?returnTo=%2Fapp%2Fworkers%3Fcohort%3Dpeta",
    );
  });
});

describe("storeReturnToPending / consumeReturnToPending", () => {
  // happy-dom doesn't expose Web Storage on `window` by default
  // (vitest's setup mounts no Storage shim), so we wire a small
  // in-memory replacement scoped to this suite.
  let backing: Map<string, string>;

  beforeEach(() => {
    backing = new Map();
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      value: {
        getItem: (k: string) => (backing.has(k) ? backing.get(k)! : null),
        setItem: (k: string, v: string) => {
          backing.set(k, String(v));
        },
        removeItem: (k: string) => {
          backing.delete(k);
        },
        clear: () => {
          backing.clear();
        },
      },
    });
  });

  afterEach(() => {
    backing.clear();
    vi.useRealTimers();
  });

  it("stores then consumes a whitelisted path", () => {
    storeReturnToPending("/app/workers?cohort=peta");
    expect(consumeReturnToPending()).toBe("/app/workers?cohort=peta");
    // Consume is destructive — second read returns null.
    expect(consumeReturnToPending()).toBeNull();
  });

  it("refuses to store a non-whitelisted path", () => {
    storeReturnToPending("https://evil.com/");
    expect(consumeReturnToPending()).toBeNull();
    storeReturnToPending("/landing");
    expect(consumeReturnToPending()).toBeNull();
  });

  it("returns null when storage is empty", () => {
    expect(consumeReturnToPending()).toBeNull();
  });

  it("returns null and clears stale entries after the TTL", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-22T12:00:00Z"));
    storeReturnToPending("/audit/events");
    vi.setSystemTime(new Date("2026-05-22T12:16:00Z")); // 16 min later
    expect(consumeReturnToPending()).toBeNull();
  });

  it("re-validates on read (tampered localStorage folds to null)", () => {
    window.localStorage.setItem(
      "skycot:return-to-pending",
      JSON.stringify({ p: "https://evil.com/", t: Date.now() }),
    );
    expect(consumeReturnToPending()).toBeNull();
  });
});

