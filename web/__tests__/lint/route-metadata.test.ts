import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

/**
 * Contract tests for `scripts/check-route-metadata.sh` — the
 * recurrence-prevention lint for the (app) group default-title leak.
 * The (app) layout's `default: "Approvals"` previously inherited
 * onto 9 sibling routes (/genesis, /brand, /retro, /billing,
 * /verticals, /orchestration, /support/inbox, /brand/components,
 * /billing/subscribe) — visible in the browser tab as
 * "Approvals · SKYCOT" during demo recording even when the page was
 * Genesis or Brand. The fix neutralised the default to "SKYCOT" and
 * required every page.tsx to declare its own per-route metadata
 * (either inline or via an ancestor layout.tsx in the route subtree).
 *
 * This file pins both halves: the script fires on a known orphan,
 * stays quiet when metadata is declared anywhere up the route's
 * subtree, and passes on the real `web/app/(app)/` tree today.
 */

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(THIS_DIR, "..", "..", "..");
const WEB_DIR = resolve(THIS_DIR, "..", "..");
const SCRIPT = join(REPO_ROOT, "scripts", "check-route-metadata.sh");

interface RunResult {
  code: number;
  stdout: string;
  stderr: string;
}

function runScript(targetWebDir?: string): RunResult {
  const args = targetWebDir ? [SCRIPT, targetWebDir] : [SCRIPT];
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
let groupRoot: string;

beforeEach(() => {
  sandbox = mkdtempSync(join(tmpdir(), "skycot-route-metadata-"));
  groupRoot = join(sandbox, "app", "(app)");
  mkdirSync(groupRoot, { recursive: true });
  // Group root layout — has metadata (default + template) but does
  // NOT count as "per-route" metadata (the lint excludes the group
  // root specifically because its "default" is what we are guarding
  // against).
  writeFileSync(
    join(groupRoot, "layout.tsx"),
    'import type { Metadata } from "next";\nexport const metadata: Metadata = { title: { default: "SKYCOT", template: "%s · SKYCOT" } };\nexport default function L({ children }: { children: React.ReactNode }) { return <>{children}</>; }\n',
  );
});

afterEach(() => {
  rmSync(sandbox, { recursive: true, force: true });
});

describe("check-route-metadata — planted-orphan contract", () => {
  it("flags a page.tsx with no inline metadata and no layout metadata", () => {
    const route = join(groupRoot, "orphan");
    mkdirSync(route, { recursive: true });
    writeFileSync(join(route, "page.tsx"), '"use client";\nexport default function P() { return null; }\n');
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("orphan");
    expect(r.stderr).toMatch(/per-route metadata/);
  });

  it("flags an orphan even when a deeper sibling route DOES declare metadata", () => {
    mkdirSync(join(groupRoot, "good"), { recursive: true });
    writeFileSync(
      join(groupRoot, "good", "page.tsx"),
      'export const metadata = { title: "Good" };\nexport default function P() { return null; }\n',
    );
    mkdirSync(join(groupRoot, "bad"), { recursive: true });
    writeFileSync(
      join(groupRoot, "bad", "page.tsx"),
      '"use client";\nexport default function P() { return null; }\n',
    );
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("bad");
    expect(r.stderr).not.toContain("good/page.tsx");
  });

  it("rejects metadata that is NOT at module scope (the Next.js contract)", () => {
    // A `metadata` declaration inside a function body does not satisfy
    // Next.js's metadata export contract — the lint requires the
    // `^export const metadata` line at column zero.
    const route = join(groupRoot, "inside-fn");
    mkdirSync(route, { recursive: true });
    writeFileSync(
      join(route, "page.tsx"),
      'export default function P() {\n  const metadata = { title: "X" };\n  return null;\n}\n',
    );
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("inside-fn");
  });

  it("does NOT accept the (app) group root layout's metadata as per-route", () => {
    // The (app) group layout itself provides a `default` title, but the
    // lint exists precisely to refuse that fallback. A naked page.tsx
    // is an orphan even though the group layout has metadata.
    writeFileSync(
      join(groupRoot, "page.tsx"),
      '"use client";\nexport default function P() { return null; }\n',
    );
    const r = runScript(sandbox);
    expect(r.code).toBe(1);
    expect(r.stderr).toContain("page.tsx");
  });
});

describe("check-route-metadata — happy-path contract", () => {
  it("accepts a page.tsx with inline metadata", () => {
    const route = join(groupRoot, "inline");
    mkdirSync(route, { recursive: true });
    writeFileSync(
      join(route, "page.tsx"),
      'export const metadata = { title: "Inline" };\nexport default function P() { return null; }\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("accepts a client page.tsx whose sibling layout.tsx declares metadata", () => {
    const route = join(groupRoot, "with-layout");
    mkdirSync(route, { recursive: true });
    writeFileSync(
      join(route, "layout.tsx"),
      'import type { Metadata } from "next";\nexport const metadata: Metadata = { title: "WithLayout" };\nexport default function L({ children }: { children: React.ReactNode }) { return <>{children}</>; }\n',
    );
    writeFileSync(
      join(route, "page.tsx"),
      '"use client";\nexport default function P() { return null; }\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("accepts a nested page.tsx that inherits from an ancestor layout (e.g. /workers/[id])", () => {
    const ancestor = join(groupRoot, "workers");
    const child = join(ancestor, "[workerId]");
    mkdirSync(child, { recursive: true });
    writeFileSync(
      join(ancestor, "layout.tsx"),
      'import type { Metadata } from "next";\nexport const metadata: Metadata = { title: "Workers" };\nexport default function L({ children }: { children: React.ReactNode }) { return <>{children}</>; }\n',
    );
    writeFileSync(
      join(child, "page.tsx"),
      '"use client";\nexport default function P() { return null; }\n',
    );
    expect(runScript(sandbox).code).toBe(0);
  });

  it("accepts an empty (app) group (no pages = nothing to lint)", () => {
    expect(runScript(sandbox).code).toBe(0);
  });
});

describe("check-route-metadata — current main smoke test", () => {
  it("passes against the real web/ directory checked into the repo", () => {
    const r = runScript(WEB_DIR);
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("clean");
  });

  it("uses web/ as the default target when no argument is passed", () => {
    expect(runScript().code).toBe(0);
  });
});
