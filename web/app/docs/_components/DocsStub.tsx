import "./docs-stub.css";

import Link from "next/link";
import { ArrowLeft, ArrowRight, Mail } from "lucide-react";
import type { ReactNode } from "react";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";

interface DocsStubProps {
  slug: string;
  category?: string;
  title: string;
  intro: ReactNode;
}

export function DocsStub({ slug, category, title, intro }: DocsStubProps) {
  const mailto = `mailto:hello@skycot.com?subject=${encodeURIComponent(
    `Docs request: ${slug}`,
  )}`;
  return (
    <div className="ds" data-testid="docs-stub">
      <MarketingNav />

      <article className="ds-article">
        <nav className="ds-crumbs" aria-label="Breadcrumb">
          <Link href="/docs" className="ds-crumbs-link">
            Docs
          </Link>
          {category ? (
            <>
              <span className="ds-crumbs-sep">/</span>
              <span className="ds-crumbs-current">{category}</span>
            </>
          ) : null}
        </nav>

        <h1 className="ds-h1">{title}</h1>
        <div className="ds-intro">{intro}</div>

        <div className="ds-cta-band">
          <div className="ds-cta-copy">
            <p className="ds-cta-eyebrow">Article in progress</p>
            <p className="ds-cta-line">
              The full write-up is queued — drop a note and we&apos;ll prioritize it.
            </p>
          </div>
          <a className="ds-cta-btn" href={mailto}>
            <Mail size={15} />
            Request this article
          </a>
        </div>

        <Link href="/docs" className="ds-back">
          <ArrowLeft size={14} />
          Back to all docs
        </Link>
      </article>

      <MarketingFooter />
    </div>
  );
}
