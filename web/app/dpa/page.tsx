import fs from "node:fs/promises";
import path from "node:path";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";
import { Markdown } from "@/app/_content/markdown";

// Server-rendered per request. Was `force-static`, but the per-request
// nonce-based CSP (web/middleware.ts) cannot be baked into static HTML:
// a prerendered page ships `nonce=""` on its inline scripts, which the
// runtime `'strict-dynamic'` policy then blocks. Dynamic rendering lets
// the page pick up the request nonce. The DPA copy still edge-caches via
// the marketing Cache-Control headers in next.config.ts.
export const dynamic = "force-dynamic";

const EFFECTIVE_DATE = "2026-05-17";

export default async function DpaPage() {
  const source = await fs.readFile(
    path.join(process.cwd(), "app", "_content", "legal", "dpa-template.md"),
    "utf-8",
  );
  return (
    <>
      <MarketingNav />
      <article className="mx-auto max-w-3xl px-6 py-16 flex flex-col gap-6">
        <p className="text-xs font-mono uppercase tracking-wide text-[var(--muted-foreground)]">
          Effective {EFFECTIVE_DATE}
        </p>
        <Markdown source={source} />
      </article>
      <MarketingFooter />
    </>
  );
}
