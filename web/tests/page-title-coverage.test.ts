import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { SKYCOT_TITLE_DEFAULT } from "@/lib/marketing-copy";

// Regression guard for fix-sla-and-tertiary-pages-inherit-default-marketing-title
// (UX audit 2026-05-28-0252 whole-site F05). Tertiary marketing /
// support pages must declare a page-specific `metadata.title` (either
// in `page.tsx` or in a sibling `layout.tsx`) so the browser tab
// renders e.g. "SLA · SKYCOT" rather than falling through to the root
// marketing default "SKYCOT — Hire your first AI teammate".
//
// Sibling briefs cover the in-app SPA leak (/approvals) and the
// onboarding step pages; this file owns the marketing/tertiary
// surface only.

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const APP_DIR = join(THIS_DIR, "..", "app");

// Routes audited as part of whole-site F05. Each entry names the
// route directory under `web/app/`. The test accepts a title declared
// either in `<route>/page.tsx` or `<route>/layout.tsx`.
const TERTIARY_ROUTES = [
  "sla",
  "help",
  "pricing",
  "security",
  "customers",
  "changelog",
  "for/finance",
  "for/marketing",
  "for/ops",
  "for/recruiting",
  "for/sales",
  "for/support",
];

function readMetadataSource(route: string): string {
  const layout = join(APP_DIR, route, "layout.tsx");
  const page = join(APP_DIR, route, "page.tsx");
  const parts: string[] = [];
  if (existsSync(layout)) parts.push(readFileSync(layout, "utf8"));
  if (existsSync(page)) parts.push(readFileSync(page, "utf8"));
  return parts.join("\n");
}

function extractTitle(source: string): string | null {
  // Locate the `export const metadata` block, then pick the first
  // `title:` value inside it. Accepts a string literal
  // (`title: "Pricing"`) or an identifier / member expression
  // (`title: V.title`) — the latter is the pattern used by the
  // /for/<vertical> layouts which source their copy from a shared
  // data module.
  const blockStart = source.search(/export\s+const\s+metadata\s*[:=]/);
  if (blockStart < 0) return null;
  const block = source.slice(blockStart, blockStart + 4000);
  const literal = block.match(/title\s*:\s*["'`]([^"'`]+)["'`]/);
  if (literal) return literal[1];
  const ident = block.match(/title\s*:\s*([A-Za-z_$][\w$.]*)\s*[,}]/);
  if (ident) return ident[1];
  return null;
}

describe("tertiary marketing pages declare a page-specific title", () => {
  for (const route of TERTIARY_ROUTES) {
    it(`/${route} declares metadata.title and does not inherit the root default`, () => {
      const source = readMetadataSource(route);
      expect(
        source.length,
        `Expected web/app/${route}/page.tsx or layout.tsx to exist`,
      ).toBeGreaterThan(0);

      const title = extractTitle(source);
      expect(
        title,
        `Expected web/app/${route} to declare an \`export const metadata\` with a \`title\` field so the browser tab does not fall through to the root marketing default "${SKYCOT_TITLE_DEFAULT}".`,
      ).not.toBeNull();
      expect(
        title,
        `Page-specific title for /${route} must differ from the root default "${SKYCOT_TITLE_DEFAULT}".`,
      ).not.toBe(SKYCOT_TITLE_DEFAULT);
      expect(title?.trim().length).toBeGreaterThan(0);
    });
  }
});
