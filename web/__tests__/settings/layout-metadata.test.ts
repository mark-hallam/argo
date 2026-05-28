import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Pins every /settings/* sub-page's `layout.tsx` to declare an
// `export const metadata` with the page-name title. The parent
// `(app)/layout.tsx` template `"%s · SKYCOT"` slots the title in, so
// e.g. `title: "Connectors"` yields the browser tab "Connectors ·
// SKYCOT". Pages without metadata fall through to the parent settings
// shell default of "Settings", which reads "Settings · SKYCOT" in the
// tab regardless of which sub-page is open — visible to the demo camera.

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..", "..");
const SETTINGS_DIR = join(REPO_ROOT, "web", "app", "(app)", "settings");

const SUB_PAGES: { slug: string; title: string }[] = [
  { slug: "api-keys", title: "API keys" },
  { slug: "billing", title: "Billing" },
  { slug: "connectors", title: "Connectors" },
  { slug: "data-export", title: "Data export" },
  { slug: "knowledge", title: "Knowledge" },
  { slug: "members", title: "Members" },
  { slug: "notifications", title: "Notifications" },
  { slug: "notify-channels", title: "Notify channels" },
  { slug: "providers", title: "Providers" },
  { slug: "security", title: "Security" },
  { slug: "workspace", title: "Workspace" },
];

describe("settings sub-page layout.tsx — metadata title pins", () => {
  it.each(SUB_PAGES)(
    "$slug declares `export const metadata` with title \"$title\"",
    ({ slug, title }) => {
      const layoutPath = join(SETTINGS_DIR, slug, "layout.tsx");
      const text = readFileSync(layoutPath, "utf8");

      expect(text, `${slug}/layout.tsx must export const metadata`).toMatch(
        /export\s+const\s+metadata\s*:\s*Metadata\s*=/,
      );

      const titleRe = new RegExp(
        `title:\\s*"${title.replace(/[.*+?^${}()|[\\]\\\\]/g, "\\$&")}"`,
      );
      expect(
        text,
        `${slug}/layout.tsx must set title to "${title}" so tab reads "${title} · SKYCOT"`,
      ).toMatch(titleRe);
    },
  );
});
