import "./page.css";

import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { SEED_CUSTOMERS } from "@/lib/content/customers";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";

/**
 * Public customers index — `/customers`, exempt from auth middleware.
 *
 * Brand-loud register matching `/landing` and `/pricing`. 6-card grid +
 * "become a design partner" CTA band. Each card links to a representative
 * scenario page at `/customers/[slug]`.
 *
 * NOTE: these are REPRESENTATIVE SCENARIOS, not real named customers — third
 * person, no fabricated person-quotes, no compliance claims in a fictional
 * mouth (shakedown F3). Every story carries `dataPending: true` for trivial
 * bulk-replace when real, cleared design-partner content lands.
 */

export default function CustomersPage() {
  return (
    <>
      <div className="cus">
        <MarketingNav />

        <section className="cus-hero">
          <div className="cus-eyebrow">
            <span className="cus-eyebrow-dot" />
            SCENARIO PROFILES
          </div>
          <h1 className="cus-h1">
            Teams SKYCOT is <strong>built to support</strong>.
          </h1>
          <p className="cus-sub">
            Representative deployment profiles using current workflow templates. Real design-partner stories replace these once cleared for publication.
          </p>
        </section>

        <div className="cus-grid">
          {SEED_CUSTOMERS.map((c) => (
            <Link
              key={c.slug}
              href={`/customers/${c.slug}`}
              className="cus-card"
              data-pending-real-scenario={c.dataPending ? "true" : undefined}
            >
              <div className="cus-card-head">
                <div className="cus-card-logo" style={{ background: c.logoGradient }}>
                  {c.logoInitials}
                </div>
                <div>
                  <h3 className="cus-card-name">{c.name}</h3>
                  <div className="cus-card-meta">{c.industry} · {c.size}</div>
                </div>
              </div>
              <span className="cus-status-chip">Representative scenario · not a real customer</span>
              <p className="cus-kpi-line">
                <strong>{c.cardKpi}</strong>
              </p>
              <p className="cus-scenario">{c.cardQuote}</p>
              <div className="cus-card-foot">
                <span className="cus-vertical-chip">{c.vertical}</span>
                <span className="cus-card-cta">
                  View profile <ArrowRight size={14} />
                </span>
              </div>
            </Link>
          ))}
        </div>

        <section className="cus-final">
          <h2>Become a design partner.</h2>
          <p>We&apos;re onboarding design partners every week. Tell us what you&apos;d hire SKYCOT for and we&apos;ll show you how.</p>
          <a href="mailto:hello@skycot.com?subject=SKYCOT%20design%20partner" className="cus-final-cta">
            Email hello@skycot.com <ArrowRight size={16} />
          </a>
        </section>

        <MarketingFooter />
      </div>
    </>
  );
}
