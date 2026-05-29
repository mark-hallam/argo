import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

/**
 * Contract tests for `scripts/check-no-dead-spa-routes.sh` — the
 * recurrence-prevention lint for UX-AUDIT-T0519 F11 (stale `Link href`
 * strings pointing at `/workflows/runs` and `/workflows/edit` produced
 * 404 noise in dev logs and visible 404s in the DevTools network panel).
 * The script is the v1 guard; this file pins both its "fires on a known
 * bad sample" and "stays quiet on documented allowlist patterns"
 * contracts so future workers don't soften the rule without thinking.
 *
 * The "current main" smoke test ensures the script passes on the real
 * `web/` directory checked into the repo.
 */

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(THIS_DIR, "..", "..", "..");
const WEB_DIR = resolve(THIS_DIR, "..", "..");
const SCRIPT = join(REPO_ROOT, "scripts", "check-no-dead-spa-routes.sh");

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
  sandbox = mkdtempSync(join(tmpdir(), "skycot-no-dead-spa-routes-"));
});

afterEach(() => {
  rmSync(sandbox, { recursive: true, force: true });
});

describe("check-no-dead-spa-routes — planted-bad-sample contract", () => {
  it("flags a Link href to /workflows/runs", () => {
    writeFileSync(
      join(sandbox, "bad-link.tsx"),
      'import Link from "next/link";\nexport const X = () => <Link href="/workflows/runs">go</Link>;\n',
    );
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("bad-link.tsx");
    expect(r.stderr).toContain("/workflows/runs");
  });

  it("flags an anchor href to /workflows/edit", () => {
    writeFileSync(
      join(sandbox, "bad-anchor.tsx"),
      'export const X = () => <a href="/workflows/edit">edit</a>;\n',
    );
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("bad-anchor.tsx");
  });

  it("flags a router.push call to a dead route", () => {
    writeFileSync(
      join(sandbox, "bad-router.tsx"),
      'import { useRouter } from "next/navigation";\nexport const X = () => { const r = useRouter(); r.push("/workflows/runs"); return null; };\n',
    );
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("bad-router.tsx");
  });

  it("flags an href with a query string appended to a dead route", () => {
    writeFileSync(
      join(sandbox, "bad-query.tsx"),
      'export const X = () => <a href="/workflows/edit?range=7d">go</a>;\n',
    );
    expect(runScript(sandbox).code).toBe(1);
  });

  it("flags a JSX expression href={\"/workflows/runs\"}", () => {
    writeFileSync(
      join(sandbox, "bad-expr.tsx"),
      'import Link from "next/link";\nexport const X = () => <Link href={"/workflows/runs"}>go</Link>;\n',
    );
    expect(runScript(sandbox).code).toBe(1);
  });

  it("reports all violations across files, not just the first", () => {
    writeFileSync(
      join(sandbox, "a.tsx"),
      'export const A = () => <a href="/workflows/runs">a</a>;\n',
    );
    writeFileSync(
      join(sandbox, "b.tsx"),
      'export const B = () => <a href="/workflows/edit">b</a>;\n',
    );
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("a.tsx");
    expect(r.stderr).toContain("b.tsx");
  });
});

describe("check-no-dead-spa-routes — allowlist contract", () => {
  it("does NOT fire on next.config.ts (the redirects ARE the references)", () => {
    writeFileSync(
      join(sandbox, "next.config.ts"),
      'export default { redirects: () => [{ source: "/workflows/runs", destination: "/audit", permanent: false }] };\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("does NOT fire on *.test.ts files (fixtures may quote dead routes)", () => {
    writeFileSync(
      join(sandbox, "redirect.test.ts"),
      'import { test, expect } from "vitest";\ntest("x", () => { expect("/workflows/runs").toBeTruthy(); });\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("does NOT fire on lines tagged // allowlist:dead-spa-route", () => {
    writeFileSync(
      join(sandbox, "doc.tsx"),
      'export const X = () => <a href="/workflows/runs">go</a>; // allowlist:dead-spa-route\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("does NOT fire on a string literal in non-nav context (e.g. comment)", () => {
    writeFileSync(
      join(sandbox, "comment.ts"),
      'export const docPath = "see /workflows/runs spec";\n',
    );
    // This string occurs in non-nav context (no href/.push/.replace/to=)
    // so the post-filter drops it silently.
    expect(runScript(sandbox).code).toBe(0);
  });

  it("does NOT fire on an empty target directory", () => {
    const r = runScript(sandbox);
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("OK");
  });
});

describe("check-no-dead-spa-routes — current main smoke test", () => {
  it("passes against the real web/ directory", () => {
    const r = runScript(WEB_DIR);
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("OK");
  });

  it("uses web/ as the default target when no argument is passed", () => {
    expect(runScript().code).toBe(0);
  });
});
