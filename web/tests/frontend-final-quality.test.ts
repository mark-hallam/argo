import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

function readWeb(path: string): string {
  return readFileSync(join(process.cwd(), path), "utf8");
}

describe("final frontend quality guardrails", () => {
  it("keeps touched in-product surfaces on runtime brand tokens", () => {
    const sources = [
      "app/(app)/settings/security/page.tsx",
      "app/(app)/settings/security/SamlConfigModal.tsx",
      "app/(app)/workflows/[verticalId]/[name]/build/page.css",
      "app/(app)/workflows/[verticalId]/[name]/build/BuilderInspector.tsx",
      "app/(app)/workflows/[verticalId]/[name]/build/step-config-code.tsx",
    ];
    const forbidden = [
      /#8b5cf6/i,
      /#ec4899/i,
      /#1e1b4b/i,
      /#52525b/i,
      /rgba\(\s*139\s*,\s*92\s*,\s*246/i,
      /(?:background|color):\s*(?:white|#fff|#ffffff)\b/i,
      /font-family:\s*"Inter"/i,
    ];

    for (const sourcePath of sources) {
      const source = readWeb(sourcePath);
      expect(source, `${sourcePath} should consume runtime theme tokens`).toMatch(
        /var\(--foreground\)|var\(--muted-foreground\)|var\(--card\)|var\(--border\)|var\(--primary\)/,
      );
      for (const pattern of forbidden) {
        expect(source, `${sourcePath} contains raw locked-brand styling: ${pattern}`).not.toMatch(
          pattern,
        );
      }
    }
  });

  it("pins keyboard and dialog semantics on camera-visible controls", () => {
    const samlModal = readWeb("app/(app)/settings/security/SamlConfigModal.tsx");
    const builderInspector = readWeb("app/(app)/workflows/[verticalId]/[name]/build/BuilderInspector.tsx");

    expect(samlModal).toMatch(/role="dialog"/);
    expect(samlModal).toMatch(/aria-modal="true"/);
    expect(samlModal).toMatch(/aria-describedby="saml-modal-description"/);
    expect(samlModal).toMatch(/event\.key === "Escape"/);
    expect(builderInspector).toMatch(/aria-label="Close step inspector"/);
    expect(builderInspector).toMatch(/<X size=\{16\} aria-hidden="true" \/>/);
  });

  it("keeps high-value pages responsive and theme-aware by source", () => {
    const sources = [
      "app/(app)/settings/security/page.tsx",
      "app/(app)/workflows/[verticalId]/[name]/build/page.css",
      "app/landing/page.css",
      "app/pricing/page.css",
      "app/compare/page.css",
    ];

    for (const sourcePath of sources) {
      const source = readWeb(sourcePath);
      expect(source, `${sourcePath} needs mobile/tablet breakpoints`).toMatch(/@media\s*\(max-width:/);
      expect(source, `${sourcePath} should inherit light/dark theme tokens`).toMatch(
        /var\(--background\)|var\(--foreground\)|var\(--card\)|var\(--border\)/,
      );
      expect(source, `${sourcePath} should expose focus-visible styling`).toMatch(/focus-visible/);
    }
  });

  it("lazy-loads heavy canvas renderers without removing styled-jsx client pages", () => {
    const skillCanvasPage = readWeb("app/(app)/workers/[workerId]/skills/[skillId]/canvas/page.tsx");
    const workflowCodeEditor = readWeb(
      "app/(app)/workflows/[verticalId]/[name]/build/step-config-code.tsx",
    );

    expect(skillCanvasPage).toMatch(/^"use client";/);
    expect(skillCanvasPage).toMatch(/import dynamic from "next\/dynamic"/);
    expect(skillCanvasPage).toMatch(/dynamic<SkillCanvasProps>/);
    expect(skillCanvasPage).not.toMatch(
      /import\s+\{\s*SkillCanvas\s*\}\s+from\s+"@\/components\/skycot\/canvas\/SkillCanvas"/,
    );
    expect(workflowCodeEditor).toMatch(/dynamic\(\s*\(\) => import\("@\/components\/builder\/MonacoCodeEditor"\)/);
  });
});
