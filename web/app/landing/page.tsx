import Link from "next/link";
import { ArrowRight, Check, Shield, MessageSquareText, Sparkles, GitBranch, Users2 } from "lucide-react";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";
import { WaitlistForm } from "@/components/marketing/WaitlistForm";
import { PortiaEmbed } from "@/components/marketing/PortiaEmbed";
import { DemoVideoSlot } from "@/components/marketing/DemoVideoSlot";
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
      "SKYCOT spins up a named teammate (Mariah, Peta, Priya, Jamal, or Nori) with the right skills, joins your Slack, and introduces themselves.",
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
      "Every action, every approval, every decision — fully audit-logged with teammate, action, cost, latency, and outcome.",
  },
];

// The four verified-unique pillars (docs/skycot/marketing/DIFFERENTIATORS.md).
// Across all 25 surveyed competitors, none matched these — so the copy leans
// in. Each pairs with the live proof we already ship, linked for the curious.
const DIFFERENTIATORS = [
  {
    num: "01",
    icon: <Shield size={22} />,
    kicker: "Provable by design",
    title: "Every action, provable",
    blurb:
      "Every action an AI teammate takes writes to a hash-chained, tamper-evident ledger — cryptographically impossible to alter or delete after the fact. An audit trail you can take to your board, or your regulator.",
    proofLabel: "See the audit chain",
    proofHref: "/audit",
  },
  {
    num: "02",
    icon: <MessageSquareText size={22} />,
    kicker: "Can't overclaim",
    title: "An AI that knows what it's not allowed to say",
    blurb:
      "A claim-level truth boundary gates what a teammate may assert — not just what it can access. Portia, our Pitch Agent, literally cannot state a number she can't back up. She defers to a human instead.",
    proofLabel: "Watch Portia refuse to overclaim",
    proofHref: "/pitch",
  },
  {
    num: "03",
    icon: <Check size={22} />,
    kicker: "Approval-first",
    title: "Nothing risky without your yes",
    blurb:
      "Sensitive actions stop for human approval — delivered to wherever you already are, across 9 channels: Slack, Discord, SMS, voice, iMessage, web, push, Teams, and email. Approve or reject from your phone.",
    proofLabel: "See the approvals inbox",
    proofHref: "/approvals",
  },
  {
    num: "04",
    icon: <Users2 size={22} />,
    kicker: "Hire, don't prompt",
    title: "Hire teammates, not chatbots",
    blurb:
      "You hire AI teammates that hold a standing responsibility — persistent identity, memory, delegation to each other, shared learned skills. Not one-off task bots you re-prompt every time.",
    proofLabel: "Meet the roster",
    proofHref: "/teammates",
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
              Early access · Inviting our first 100 teams
            </div>
            <h1 className="land-hero-h1">
              Don&apos;t buy another tool.
              <br />
              Hire a <strong>teammate</strong>.<br />
              (Seriously.)
            </h1>
            <p className="land-hero-sub">
              SKYCOT turns a plain-English job description into an AI teammate that lives in Slack,
              learns from your feedback, and asks for approval before it acts. Provable by design,
              truth-bounded, enterprise-grade.
            </p>
            <div className="land-hero-waitlist">
              <p className="land-hero-waitlist-label">
                Hire your first teammate
              </p>
              <WaitlistForm source="landing-hero" size="lg" />
            </div>
            <div className="land-hero-actions">
              <a href="#why" className="land-btn-secondary">
                Why we&apos;re different
              </a>
              <a href="#demo" className="land-btn-secondary">
                Watch the demo
              </a>
            </div>
            <div className="land-trust-strip">
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                NO CREDIT CARD
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                TAMPER-EVIDENT AUDIT TRAIL
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                TRUTH-BOUNDED BY DEFAULT
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                APPROVAL-FIRST · 9 CHANNELS
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                MADE IN MELBOURNE
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
              <strong style={{ color: "var(--brand-primary)" }}>Mariah</strong>: 3 listings live, 2 buyers waiting on you.
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
              <strong style={{ color: "var(--gradient-end)" }}>Peta</strong>: standup digest drafted, 3 action items chased.
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
              <strong style={{ color: "var(--brand-primary)" }}>Priya</strong>: 8 candidates screened today.
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
              <strong style={{ color: "var(--brand-teal)" }}>Jamal</strong>: 12 tickets resolved, median reply 4m.
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
              <strong style={{ color: "var(--brand-teal)" }}>Nori</strong>: Q2 update draft ready for review.
            </div>
            <div className="land-art-center" style={{ left: "40%", zIndex: 1 }}>
              SKYCOT
            </div>
          </div>
        </section>

        {/* The moat — verified differentiators, led hard, right under the hero */}
        <section className="land-section land-moat" id="why">
          <h2 className="land-section-title">
            Four things <strong>nobody else</strong> ships.
          </h2>
          <p className="land-moat-lede">
            We surveyed 25 AI-agent platforms — Microsoft Agent 365, Salesforce
            Agentforce, Sierra, Decagon, Lindy, Ema, Dust, LangGraph, CrewAI and
            more. <strong>Not one matched all four.</strong> The field is crowding
            on orchestration and &ldquo;AI employees.&rdquo; Our wedge is the part a
            serious buyer needs before they let an AI <em>do</em> anything: trust
            and governance for agents that take real action.
          </p>
          <div className="land-moat-grid">
            {DIFFERENTIATORS.map((item) => (
              <article key={item.title} className="land-moat-card">
                <div className="land-moat-card-top">
                  <div className="land-moat-icon">{item.icon}</div>
                  <span className="land-moat-num">{item.num}</span>
                </div>
                <p className="land-moat-kicker">{item.kicker}</p>
                <h3>{item.title}</h3>
                <p className="land-moat-blurb">{item.blurb}</p>
                <Link href={item.proofHref} className="land-moat-proof">
                  {item.proofLabel} <ArrowRight size={14} />
                </Link>
              </article>
            ))}
          </div>
        </section>

        {/* Demo film — labeled placeholder until the operator records the MP4 */}
        <section className="land-section land-demo" id="demo">
          <p className="land-section-label">SEE IT IN MOTION</p>
          <h2 className="land-section-title">
            Watch a teammate get <strong>hired</strong>.
          </h2>
          <p className="land-demo-lede">
            A short tour: describe a job in plain English, meet your new teammate,
            approve their first decisions, and watch every action land in the
            tamper-evident audit trail.
          </p>
          <div className="land-demo-wrap">
            <DemoVideoSlot durationLabel="90-second tour" />
          </div>
        </section>

        {/* Talk to Portia — public taste of the truth-bounded Pitch Agent */}
        <section className="land-section land-portia" id="portia">
          <p className="land-section-label">TALK TO PORTIA</p>
          <h2 className="land-section-title">
            Ask our AI <strong>anything</strong> about SKYCOT.
          </h2>
          <p className="land-portia-lede">
            Portia is our Pitch Agent — truth-bounded, so she can&apos;t state a
            number she can&apos;t back up. Ask her a hard question and watch the
            truth boundary work in real time.
          </p>
          <div className="land-portia-wrap">
            <PortiaEmbed />
          </div>
        </section>

        {/* How it works */}
        <section id="how" className="land-section">
          <p className="land-section-label">HOW IT WORKS</p>
          <h2 className="land-section-title">
            From plain English to a <strong>working teammate</strong>.
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
          <p className="land-section-label">REPRESENTATIVE SCENARIOS</p>
          <h2 className="land-section-title" style={{ marginBottom: 28 }}>
            Example teams SKYCOT is <strong>built to support</strong>.
          </h2>
          <p className="land-social-note">
            These are seeded deployment profiles backed by current workflow templates. Real
            design-partner stories replace them when publication is cleared.
          </p>
          <div className="land-social-strip">
            {LOGOS.map((logo) => (
              <Link
                key={logo.name}
                href={logo.href}
                className="land-social-card"
                aria-label={`View ${logo.name} representative profile`}
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
                <div className="land-quote-status">Representative profile</div>
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
              See all profiles <ArrowRight size={15} />
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

        {/* Final CTA — early-access waitlist */}
        <section className="land-final" id="early-access">
          <h2>
            Be one of our <strong>first 100 teams</strong>.
          </h2>
          <p className="land-final-sub">
            Early access is rolling out to a small group. Leave your email and
            we&apos;ll reach out when your slot opens.
          </p>
          <div className="land-final-waitlist">
            <WaitlistForm source="landing-final" size="lg" />
          </div>
        </section>

        <MarketingFooter />
      </div>
    </>
  );
}
