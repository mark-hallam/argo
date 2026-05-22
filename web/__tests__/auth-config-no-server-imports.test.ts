import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * Regression test for the `web-auth-config-server-only-split` brief
 * (P0 demo-MP4 blocker, 2026-05-22).
 *
 * `web/auth.config.ts` is bundled into every Client Component that
 * touches the NextAuth context (via the `lib/api/client.ts → @/auth`
 * dynamic-import chain, which Turbopack still traces). If
 * `auth.config.ts` ever imports `next/headers` or calls
 * `cookies()`/`headers()` directly, Turbopack refuses to compile the
 * client SSR chunk and every in-product route 500s on cold dev
 * start (the recorded failure mode from the 2026-05-22 evaluator
 * scenario `spa-cookies-import-app-routes-500.md`).
 *
 * Server-only callbacks belong in `web/auth.ts`, which loads
 * `web/lib/auth/session-cookie.ts` dynamically; that file in turn
 * keeps the `next/headers` import behind a dynamic
 * `await import("next/headers")` inside the function body so the
 * static-import graph that Turbopack inspects never reaches
 * `next/headers` from a client-bundled file.
 *
 * Both `auth.config.ts` AND `session-cookie.ts` are guarded here
 * because either regressing to a static `next/headers` import would
 * re-trip the original 500.
 */

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const WEB_DIR = join(THIS_DIR, "..");

const AUTH_CONFIG_SRC = readFileSync(join(WEB_DIR, "auth.config.ts"), "utf8");
const SESSION_COOKIE_SRC = readFileSync(
  join(WEB_DIR, "lib", "auth", "session-cookie.ts"),
  "utf8",
);

describe("auth.config.ts client-bundle safety", () => {
  it("does not statically import from 'next/headers'", () => {
    expect(AUTH_CONFIG_SRC).not.toMatch(
      /^\s*import[^;]*from\s+["']next\/headers["']/m,
    );
  });

  it("does not call cookies()", () => {
    expect(AUTH_CONFIG_SRC).not.toMatch(/\bcookies\s*\(/);
  });

  it("does not call headers()", () => {
    expect(AUTH_CONFIG_SRC).not.toMatch(/\bheaders\s*\(/);
  });
});

describe("session-cookie.ts dynamic-import guard", () => {
  it("does not statically import from 'next/headers'", () => {
    expect(SESSION_COOKIE_SRC).not.toMatch(
      /^\s*import[^;]*from\s+["']next\/headers["']/m,
    );
  });

  it("uses a dynamic `await import('next/headers')` instead", () => {
    expect(SESSION_COOKIE_SRC).toMatch(
      /await\s+import\s*\(\s*["']next\/headers["']\s*\)/,
    );
  });
});
