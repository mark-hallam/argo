import "./page.css";

import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { SEED_CUSTOMERS } from "@/lib/content/customers";

/**
 * Public customers index — `/customers`, exempt from auth middleware.
 *
 * Brand-loud register matching `/landing` and `/pricing`. 6-card grid +
 * "want to be featured?" CTA band. Each card links to a per-customer
 * detail page at `/customers/[slug]`.
 *
 * NOTE: placeholder customers — every story carries `dataPending: true`
 * for trivial bulk-replace when real design-partner content lands.
 */

export default function CustomersPage() {
  return (
    <>
      <div className="cus">
        <header className="cus-top">
          <Link href="/landing" className="cus-brand">
            <div className="cus-brand-mark" />
            <div className="cus-brand-name">SKYCOT</div>
          </Link>
          <nav className="cus-nav">
            <Link href="/landing#how">How it works</Link>
            <Link href="/landing#features">Features</Link>
            <Link href="/compare">Compare</Link>
            <Link href="/customers" className="active">Customers</Link>
            <Link href="/pricing">Pricing</Link>
          </nav>
          <div className="cus-top-right">
            <Link href="/auth/signin" className="cus-signin">Sign in</Link>
            <Link href="/auth/signup" className="cus-cta-pill">Hire your first teammate</Link>
          </div>
        </header>

        <section className="cus-hero">
          <div className="cus-eyebrow">
            <span className="cus-eyebrow-dot" />
            CUSTOMERS
          </div>
          <h1 className="cus-h1">
            Companies running on <strong>SKYCOT</strong>.
          </h1>
          <p className="cus-sub">
            From sales teams of 12 to ops teams of 1,200, SKYCOT is the AI-teammates platform for serious businesses.
          </p>
        </section>

        <div className="cus-grid">
          {SEED_CUSTOMERS.map((c) => (
            <Link
              key={c.slug}
              href={`/customers/${c.slug}`}
              className="cus-card"
              data-pending-real-quote={c.dataPending ? "true" : undefined}
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
              <p className="cus-kpi-line">
                <strong>{c.cardKpi}</strong>
              </p>
              <blockquote className="cus-quote">&ldquo;{c.cardQuote}&rdquo;</blockquote>
              <div className="cus-quote-author">{c.cardQuoteAuthor}</div>
              <div className="cus-card-foot">
                <span className="cus-vertical-chip">{c.vertical}</span>
                <span className="cus-card-cta">
                  Read story <ArrowRight size={14} />
                </span>
              </div>
            </Link>
          ))}
        </div>

        <section className="cus-final">
          <h2>Get featured here.</h2>
          <p>We&apos;re onboarding design partners every week. Tell us what you&apos;d hire SKYCOT for and we&apos;ll show you how.</p>
          <a href="mailto:hello@skycot.com?subject=SKYCOT%20design%20partner" className="cus-final-cta">
            Email hello@skycot.com <ArrowRight size={16} />
          </a>
        </section>

        <footer className="cus-foot">
          <div style={{ display: "flex", gap: 24, flexWrap: "wrap", justifyContent: "center" }}>
            <Link href="/landing">SKYCOT</Link>
            <Link href="/compare">Compare</Link>
            <Link href="/customers">Customers</Link>
            <Link href="/pricing">Pricing</Link>
            <a href="https://github.com/mark-hallam/skycot-packs">GitHub</a>
            <a href="mailto:hello@skycot.com">Contact</a>
          </div>
          <div>© 2026 SKYCOT · Made in Melbourne · skycot.com</div>
        </footer>
      </div>
    </>
  );
}
