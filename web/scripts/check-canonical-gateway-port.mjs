#!/usr/bin/env node
/**
 * Build-time canonical-gateway-port guard.
 *
 * Wired as the `prebuild` npm script so it fires before every
 * `next build` (including `pnpm build` / `npm run build` / CI).
 * Refuses to ship if the canonical literal in
 * `web/next.config.ts` has drifted from
 * `CANONICAL_LOCAL_GATEWAY_URL` in `web/lib/config/gateway-url.ts`.
 *
 * In `--strict` mode (used by CI / production-build pipelines) also
 * refuses to ship if `NEXT_PUBLIC_GATEWAY_URL` is unset or points
 * somewhere other than the canonical URL.
 *
 * Background: the Content-Security-Policy `connect-src` directive is
 * assembled from `NEXT_PUBLIC_GATEWAY_URL` at build time and baked
 * into HTML. If the SPA's runtime fetches read a different URL
 * (different fallback, stale env var, drift), every fetch is silently
 * CSP-blocked with only a small browser-console line as evidence —
 * a P1 deploy hazard. See
 * `docs/build-plan/streams/fix-csp-connect-src-gateway-port-drift.md`.
 */

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const webRoot = resolve(here, "..");
const STRICT = process.argv.includes("--strict") || process.env.NODE_ENV === "production";

const NEXT_CONFIG_PATH = join(webRoot, "next.config.ts");
const GATEWAY_MODULE_PATH = join(webRoot, "lib/config/gateway-url.ts");

function readPort(file, label, pattern) {
  let body;
  try {
    body = readFileSync(file, "utf8");
  } catch (err) {
    fail(`${label}: could not read ${file} (${err.message})`);
    return null;
  }
  const match = body.match(pattern);
  if (!match) {
    fail(`${label}: pattern not found in ${file}`);
    return null;
  }
  return match[1];
}

function fail(msg) {
  process.stderr.write(`[check-canonical-gateway-port] ${msg}\n`);
  process.exitCode = 1;
}

function pass(msg) {
  process.stdout.write(`[check-canonical-gateway-port] ${msg}\n`);
}

const nextConfigUrl = readPort(
  NEXT_CONFIG_PATH,
  "next.config.ts",
  /CANONICAL_LOCAL_GATEWAY_URL\s*=\s*"([^"]+)"/,
);
const moduleUrl = readPort(
  GATEWAY_MODULE_PATH,
  "lib/config/gateway-url.ts",
  /CANONICAL_LOCAL_GATEWAY_URL\s*=\s*"([^"]+)"\s*as const/,
);

if (nextConfigUrl && moduleUrl && nextConfigUrl !== moduleUrl) {
  fail(
    `Canonical gateway URL drift: next.config.ts has \`${nextConfigUrl}\` but lib/config/gateway-url.ts has \`${moduleUrl}\`. ` +
      `These must match exactly or the CSP \`connect-src\` directive baked at build time will refuse the runtime fetches the SPA hooks issue.`,
  );
}

if (STRICT) {
  const envUrl = process.env.NEXT_PUBLIC_GATEWAY_URL ?? process.env.NEXT_PUBLIC_API_URL;
  if (!envUrl) {
    fail(
      "STRICT: NEXT_PUBLIC_GATEWAY_URL (or NEXT_PUBLIC_API_URL) is unset. " +
        "Production builds must explicitly set the gateway URL so the baked CSP `connect-src` whitelists the right host.",
    );
  } else if (moduleUrl && envUrl !== moduleUrl) {
    if (envUrl.startsWith("https://")) {
      pass(`STRICT: NEXT_PUBLIC_GATEWAY_URL=${envUrl} (production override; canonical is ${moduleUrl}).`);
    } else {
      fail(
        `STRICT: NEXT_PUBLIC_GATEWAY_URL=${envUrl} disagrees with the canonical dev port ${moduleUrl}. ` +
          "Production deploys must use a non-localhost HTTPS gateway URL; reject localhost overrides in strict mode.",
      );
    }
  }
}

if (process.exitCode === 1) {
  process.stderr.write(
    "[check-canonical-gateway-port] FAILED. Refusing to ship build.\n",
  );
} else if (nextConfigUrl && moduleUrl) {
  pass(`OK. Canonical gateway URL is \`${moduleUrl}\`.`);
}
