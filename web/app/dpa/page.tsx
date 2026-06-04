import fs from "node:fs/promises";
import path from "node:path";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";
import { Markdown } from "@/app/_content/markdown";

// Statically generated at build time from the co-located template.
export const dynamic = "force-static";

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
