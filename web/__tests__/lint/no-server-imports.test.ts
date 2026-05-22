import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

/**
 * Contract tests for `scripts/check-no-server-imports.sh` — the
 * recurrence-prevention lint for P0 #2027 (cookies/next/headers leaked
 * into the client SSR bundle, breaking every in-product route at cold
 * dev start). The script is the v1 guard; this test file pins both its
 * "fires on a known bad sample" and "stays quiet on documented server-
 * only patterns" contracts so future workers don't soften the rule
 * without thinking.
 *
 * The "current main" smoke test at the bottom ensures the script
 * passes on the real `web/` directory checked into the repo — i.e.
 * shipping today does not break the lint.
 */

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(THIS_DIR, "..", "..", "..");
const WEB_DIR = resolve(THIS_DIR, "..", "..");
const SCRIPT = join(REPO_ROOT, "scripts", "check-no-server-imports.sh");

interface RunResult {
  code: number;
  stdout: string;
  stderr: string;
}

function runScript(targetDir?: string): RunResult {
  const args = targetDir ? [SCRIPT, targetDir] : [SCRIPT];
  try {
    const stdout = execFileSync("bash", args, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return { code: 0, stdout, stderr: "" };
  } catch (err) {
    const e = err as { status?: number; stdout?: Buffer; stderr?: Buffer };
    return {
      code: e.status ?? 1,
      stdout: e.stdout?.toString() ?? "",
      stderr: e.stderr?.toString() ?? "",
    };
  }
}

let sandbox: string;

beforeEach(() => {
  sandbox = mkdtempSync(join(tmpdir(), "skycot-no-server-imports-"));
});

afterEach(() => {
  rmSync(sandbox, { recursive: true, force: true });
});

describe("check-no-server-imports — planted-bad-sample contract", () => {
  it("flags a client file that statically imports next/headers", () => {
    writeFileSync(
      join(sandbox, "bad-client.tsx"),
      '"use client";\nimport { cookies } from "next/headers";\nexport default function X() { return null; }\n',
    );
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("bad-client.tsx");
    expect(r.stderr).toMatch(/next\/headers/);
  });

  it("flags a client file that imports next/cookies (alternative entry)", () => {
    writeFileSync(
      join(sandbox, "bad-cookies.tsx"),
      'import { cookies } from "next/cookies";\nexport default function X() { return null; }\n',
    );
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("bad-cookies.tsx");
  });

  it("flags an unprefixed import line (no `use client` directive needed)", () => {
    // The detection heuristic treats every file as client-eligible unless
    // path-allowlisted — so even a file without `"use client"` is flagged,
    // because Turbopack will bundle it into any client component that
    // imports it.
    writeFileSync(
      join(sandbox, "shared-util.ts"),
      'import { headers } from "next/headers";\nexport function f() {}\n',
    );
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("shared-util.ts");
  });

  it("reports all violations, not just the first", () => {
    writeFileSync(
      join(sandbox, "a.tsx"),
      'import { cookies } from "next/headers";\n',
    );
    writeFileSync(
      join(sandbox, "b.tsx"),
      'import { headers } from "next/headers";\n',
    );
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("a.tsx");
    expect(r.stderr).toContain("b.tsx");
  });
});

describe("check-no-server-imports — allowlist contract", () => {
  it("does NOT fire on auth.ts", () => {
    writeFileSync(
      join(sandbox, "auth.ts"),
      'import { cookies } from "next/headers";\nexport const x = 1;\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("does NOT fire on middleware.ts", () => {
    writeFileSync(
      join(sandbox, "middleware.ts"),
      'import { cookies } from "next/headers";\nexport function middleware() {}\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("does NOT fire on *.server.ts (explicit server-only naming)", () => {
    writeFileSync(
      join(sandbox, "actions.server.ts"),
      'import { cookies } from "next/headers";\nexport async function action() {}\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("does NOT fire on app/**/route.ts (App Router handler)", () => {
    mkdirSync(join(sandbox, "app", "api", "session"), { recursive: true });
    writeFileSync(
      join(sandbox, "app", "api", "session", "route.ts"),
      'import { cookies } from "next/headers";\nexport async function GET() {}\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("does NOT fire on files with the per-file opt-out marker", () => {
    writeFileSync(
      join(sandbox, "legit.tsx"),
      '// allowlist:no-server-imports-in-client\nimport { cookies } from "next/headers";\nexport const x = 1;\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("does NOT fire on the canonical dynamic-import workaround", () => {
    writeFileSync(
      join(sandbox, "dynamic.ts"),
      'export async function f() { const h = await import("next/headers"); return h; }\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("does NOT fire on *.test.ts files (never enter the SPA bundle)", () => {
    writeFileSync(
      join(sandbox, "some.test.ts"),
      'import { cookies } from "next/headers";\nimport { it } from "vitest";\nit("x", () => {});\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("does NOT fire when the target directory is empty", () => {
    const r = runScript(sandbox);
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("OK");
  });
});

describe("check-no-server-imports — current main smoke test", () => {
  it("passes against the real web/ directory", () => {
    const r = runScript(WEB_DIR);
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("OK");
  });

  it("uses web/ as the default target when no argument is passed", () => {
    expect(runScript().code).toBe(0);
  });
});
