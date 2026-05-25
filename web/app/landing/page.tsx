import Link from "next/link";
import { ArrowRight, Check, Shield, MessageSquareText, Sparkles, GitBranch, Users2 } from "lucide-react";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";
import { SEED_CUSTOMERS } from "@/lib/content/customers";
import "./page.css";

/**
 * Public marketing landing — `/landing`, exempt from auth middleware.
 *
 * The pre-signup surface. Brand-loud-marketing register per the
 * locked D3-with-audiowide mockup at
 * docs/skycot/design-explorations/2026-05-18/marketing-landing/
 * option-D3-with-audiowide.html.
 *
 * Hero: "Don't buy another tool. Hire a teammate. (Seriously.)"
 * Hero CTA: Sign up | Watch 90-second demo
 * Sections: How it works · Customer logos · Feature highlights ·
 *           Comparison teaser → /compare · Final CTA
 *
 * V1 ships static copy; CMS-driven testimonials + case studies arrive
 * in a follow-up brief.
 */

const FEATURES = [
  {
    icon: <Users2 size={22} />,
    title: "Named teammates, not tools",
    blurb:
      "Each AI teammate has a name, a personality, and a job. They show up in Slack like a colleague.",
  },
  {
    icon: <Shield size={22} />,
    title: "Asks before it acts",
    blurb:
      "Every risky action waits for your yes. Approval-first by default. Trust ratchets up over time.",
  },
  {
    icon: <MessageSquareText size={22} />,
    title: "Edit workflows in chat",
    blurb:
      "Describe the change in plain English. Your teammate proposes the diff, you approve, it ships.",
  },
  {
    icon: <GitBranch size={22} />,
    title: "Versioned + reversible",
    blurb:
      "Every workflow change versioned like code. Side-by-side diff. One-tap rollback.",
  },
  {
    icon: <Sparkles size={22} />,
    title: "Multi-channel native",
    blurb:
      "Slack, iMessage, email, web inbox — all wire in the same approval flow. No extra setup.",
  },
  {
    icon: <Check size={22} />,
    title: "Audit-first, enterprise-grade",
    blurb:
      "Forensic-grade audit chain on every action. Per-action role permissions. Encryption-at-rest by default.",
  },
];

const CUSTOMER_STORIES = SEED_CUSTOMERS.slice(0, 5);

const LOGOS = CUSTOMER_STORIES.map((customer) => ({
  href: `/customers/${customer.slug}`,
  initials: customer.logoInitials,
  name: customer.name,
}));

const TESTIMONIALS = [
  {
    quote: CUSTOMER_STORIES[0].cardQuote,
    name: "Priya Subramaniam",
    role: "VP Support",
    company: CUSTOMER_STORIES[0].name,
  },
  {
    quote: CUSTOMER_STORIES[1].cardQuote,
    name: "Dr Mei Lin",
    role: "Chief Compliance Officer",
    company: CUSTOMER_STORIES[1].name,
  },
  {
    quote: CUSTOMER_STORIES[3].cardQuote,
    name: "Liam Cheng",
    role: "Founder",
    company: CUSTOMER_STORIES[3].name,
  },
];

const HOW = [
  {
    n: "01",
    title: "Describe the job",
    blurb:
      "In plain English, tell SKYCOT what you'd hire someone for. 'I need someone to screen inbound leads, qualify them, and book interviews.'",
  },
  {
    n: "02",
    title: "Meet your new teammate",
    blurb:
      "SKYCOT spins up a named worker (Mariah, Peta, Priya, Jamal, or Nori) with the right skills, joins your Slack, and introduces themselves.",
  },
  {
    n: "03",
    title: "Approve their first decisions",
    blurb:
      "Your teammate proposes, you approve. Their trust grade rises as they ship correct decisions. Eventually they earn full autonomy.",
  },
  {
    n: "04",
    title: "Watch the audit chain",
    blurb:
      "Every action, every approval, every decision — fully audit-logged with worker, action, cost, latency, and outcome.",
  },
];

export default function MarketingLandingPage() {
  return (
    <>
      <div className="land">
        <MarketingNav current="/landing" />

        {/* Hero */}
        <section className="land-hero">
          <div>
            <div className="land-hero-eyebrow">
              <span className="land-hero-eyebrow-dot" />
              v1.0 shipped · Hiring our first 100 teams
            </div>
            <h1 className="land-hero-h1">
              Don&apos;t buy another tool.
              <br />
              Hire a <strong>teammate</strong>.<br />
              (Seriously.)
            </h1>
            <p className="land-hero-sub">
              SKYCOT compiles plain-English job descriptions into AI teammates that live in Slack,
              learn from your feedback, and ask for approval before they act. Audit-first,
              single-binary, enterprise-grade.
            </p>
            <div className="land-hero-actions">
              <Link href="/auth/signup" className="land-btn-primary">
                Hire your first teammate <ArrowRight size={16} />
              </Link>
              <a href="#how" className="land-btn-secondary">
                Watch a 90-second demo
              </a>
            </div>
            <div className="land-trust-strip">
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                NO CREDIT CARD
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                SOC 2-READY AUDIT TRAIL
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                ISO 27001 CONTROLS SUPPORTED
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                GDPR · HIPAA WORKFLOWS
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                EARLY ACCESS · APPLY
              </span>
            </div>
          </div>

          <div className="land-hero-art" style={{ width: "min(100%, 520px)" }}>
            <div
              className="land-art-bubble land-art-bubble-1"
              style={{
                top: "2%",
                left: "58%",
                minWidth: 160,
                maxWidth: 180,
                padding: "10px 12px",
                zIndex: 2,
              }}
            >
              <strong style={{ color: "#8b5cf6" }}>Mariah</strong>: 3 listings live, 2 buyers waiting on you.
            </div>
            <div
              className="land-art-bubble land-art-bubble-2"
              style={{
                top: "20%",
                left: "58%",
                minWidth: 160,
                maxWidth: 180,
                padding: "10px 12px",
                zIndex: 2,
              }}
            >
              <strong style={{ color: "#ec4899" }}>Peta</strong>: standup digest drafted, 3 action items chased.
            </div>
            <div
              className="land-art-bubble land-art-bubble-3"
              style={{
                top: "38%",
                left: "58%",
                minWidth: 160,
                maxWidth: 180,
                padding: "10px 12px",
                zIndex: 2,
              }}
            >
              <strong style={{ color: "#8b5cf6" }}>Priya</strong>: 8 candidates screened today.
            </div>
            <div
              className="land-art-bubble land-art-bubble-4"
              style={{
                top: "56%",
                left: "58%",
                minWidth: 160,
                maxWidth: 180,
                padding: "10px 12px",
                zIndex: 2,
              }}
            >
              <strong style={{ color: "#14b8a6" }}>Jamal</strong>: 12 tickets resolved, median reply 4m.
            </div>
            <div
              className="land-art-bubble"
              style={{
                top: "74%",
                left: "58%",
                fontSize: 12.5,
                minWidth: 160,
                maxWidth: 180,
                padding: "10px 12px",
                zIndex: 2,
              }}
            >
              <strong style={{ color: "#14b8a6" }}>Nori</strong>: Q2 update draft ready for review.
            </div>
            <div className="land-art-center">SKYCOT</div>
          </div>
        </section>

        {/* How it works */}
        <section id="how" className="land-section">
          <p className="land-section-label">HOW IT WORKS</p>
          <h2 className="land-section-title">
            From plain English to a <strong>working teammate</strong> in four minutes.
          </h2>
          <div className="land-how-grid">
            {HOW.map((s) => (
              <div key={s.n} className="land-how-step">
                <div className="land-how-n">{s.n}</div>
                <h3 className="land-how-title">{s.title}</h3>
                <p className="land-how-blurb">{s.blurb}</p>
              </div>
            ))}
          </div>
        </section>

        {/* Features */}
        <section id="features" className="land-section">
          <p className="land-section-label">WHY SKYCOT</p>
          <h2 className="land-section-title">
            Why your team will <strong>actually use them</strong>.
          </h2>
          <div className="land-feature-grid">
            {FEATURES.map((f) => (
              <div key={f.title} className="land-feature">
                <div className="land-feature-icon">{f.icon}</div>
                <h3 className="land-feature-title">{f.title}</h3>
                <p className="land-feature-blurb">{f.blurb}</p>
              </div>
            ))}
          </div>
        </section>

        {/* Social proof — logos + testimonials */}
        <section className="land-social">
          <p className="land-section-label">TRUSTED BY TEAMS LIKE YOURS</p>
          <h2 className="land-section-title" style={{ marginBottom: 28 }}>
            Already shipping with <strong>SKYCOT</strong>.
          </h2>
          <div className="land-social-strip">
            {LOGOS.map((logo) => (
              <Link
                key={logo.name}
                href={logo.href}
                className="land-social-card"
                aria-label={`Read ${logo.name} customer story`}
                style={{ textDecoration: "none" }}
              >
                <span className="land-social-logo">{logo.initials}</span>
                <span>{logo.name}</span>
              </Link>
            ))}
          </div>
          <div className="land-quote-grid">
            {TESTIMONIALS.map((t) => (
              <article key={t.name} className="land-quote-card">
                <div className="land-quote-mark">&ldquo;</div>
                <p className="land-quote-text">{t.quote}</p>
                <div className="land-quote-byline">
                  <div className="land-quote-avatar">
                    {t.name.split(" ").map((n) => n[0]).join("").slice(0, 2)}
                  </div>
                  <div>
                    <div className="land-quote-name">{t.name}</div>
                    <div className="land-quote-role">{t.role} · {t.company}</div>
                  </div>
                </div>
              </article>
            ))}
          </div>
          <div style={{ textAlign: "center", marginTop: 36 }}>
            <Link
              href="/customers"
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 8,
                padding: "12px 22px",
                borderRadius: 12,
                background: "white",
                border: "1px solid var(--line, #ece9f5)",
                color: "var(--ink, #1e1b4b)",
                fontSize: 14.5,
                fontWeight: 600,
                textDecoration: "none",
              }}
            >
              See all customers <ArrowRight size={15} />
            </Link>
          </div>
        </section>

        {/* Integration logos */}
        <section className="land-section">
          <p className="land-section-label">CONNECTS TO YOUR STACK</p>
          <h2 className="land-section-title">
            Already where your <strong>work happens</strong>.
          </h2>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))",
              gap: 14,
              maxWidth: 900,
              margin: "0 auto",
            }}
          >
            {[
              "Slack",
              "Gmail",
              "Outlook",
              "Zendesk",
              "Intercom",
              "Salesforce",
              "HubSpot",
              "QuickBooks",
              "Xero",
              "NetSuite",
              "Greenhouse",
              "Lever",
              "Workday",
              "Notion",
              "Linear",
              "GitHub",
              "Stripe",
              "Shopify",
            ].map((name) => (
              <div
                key={name}
                style={{
                  padding: "16px 18px",
                  border: "1px solid var(--line, #ece9f5)",
                  borderRadius: 12,
                  background: "white",
                  textAlign: "center",
                  fontFamily: "Inter, sans-serif",
                  fontSize: 13.5,
                  fontWeight: 600,
                  color: "var(--ink, #1e1b4b)",
                }}
              >
                {name}
              </div>
            ))}
          </div>
        </section>

        {/* Compare teaser band */}
        <section className="land-compare-band">
          <h2>How do we stack up vs Zapier, n8n, Crew.ai, Lindy?</h2>
          <p>
            Side-by-side, dimension-by-dimension. Spoiler: 8 of 13 rows are clean wins. Some
            are close. Some we&apos;ve decided not to compete on.
          </p>
          <Link href="/compare" className="land-compare-cta">
            See the comparison <ArrowRight size={16} />
          </Link>
        </section>

        {/* Final CTA */}
        <section className="land-final">
          <h2>
            Your first teammate is <strong>four minutes away</strong>.
          </h2>
          <Link href="/auth/signup" className="land-btn-primary">
            Hire your first teammate <ArrowRight size={16} />
          </Link>
        </section>

        <MarketingFooter />
      </div>
    </>
  );
}
