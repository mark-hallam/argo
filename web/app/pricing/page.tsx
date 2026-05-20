"use client";

import Link from "next/link";
import { useState } from "react";
import { ArrowRight, Check, ChevronDown } from "lucide-react";

/**
 * Public pricing page — `/pricing`, exempt from auth middleware.
 *
 * Brand-loud-marketing register matching `/landing` and `/customers`.
 * Three tiers (Free / Pro / Enterprise) mirror the `/billing` PLANS
 * constant in `web/app/(app)/billing/page.tsx`. Feature comparison
 * matrix + FAQ + final CTA.
 *
 * V1 ships static copy; CMS-driven feature matrix arrives in a
 * follow-up brief.
 */

type Tier = "free" | "pro" | "enterprise";

interface TierCard {
  tier: Tier;
  title: string;
  price: string;
  period: string;
  blurb: string;
  features: string[];
  cta: string;
  ctaHref: string;
  featured?: boolean;
}

const TIERS: TierCard[] = [
  {
    tier: "free",
    title: "Free",
    price: "$0",
    period: "forever",
    blurb: "Try SKYCOT with the mock provider. No credit card.",
    features: [
      "1 teammate",
      "100 actions / month",
      "Community support",
      "Mock provider only",
      "Single channel (web inbox)",
    ],
    cta: "Start free",
    ctaHref: "/auth/signup",
  },
  {
    tier: "pro",
    title: "Pro",
    price: "$29",
    period: "per teammate / month",
    blurb: "Real provider keys, multi-channel, marketplace skills.",
    features: [
      "5 teammates",
      "5,000 actions / month",
      "Email support",
      "Real provider keys (OpenAI / Anthropic / Bedrock)",
      "Multi-channel (Slack / iMessage / Email)",
      "Approval flows + trust ratchet",
      "Marketplace skills",
      "Versioned workflows + rollback",
    ],
    cta: "Upgrade to Pro",
    ctaHref: "/auth/signup?tier=pro",
    featured: true,
  },
  {
    tier: "enterprise",
    title: "Enterprise",
    price: "Custom",
    period: "talk to us",
    blurb: "SSO, audit export, custom verticals, priority queue.",
    features: [
      "Unlimited teammates",
      "Unlimited actions",
      "Dedicated support",
      "SSO / SAML",
      "Audit export (S3 / GCS / Azure)",
      "Custom verticals + private marketplace",
      "Priority queue + dedicated infra",
      "On-prem / single-binary deploy",
    ],
    cta: "Contact sales",
    ctaHref: "mailto:hello@skycot.com",
  },
];

interface MatrixRow {
  label: string;
  free: boolean;
  pro: boolean;
  enterprise: boolean;
}

const MATRIX: MatrixRow[] = [
  { label: "SOC 2-ready audit trail", free: true, pro: true, enterprise: true },
  { label: "GDPR workflow tools", free: true, pro: true, enterprise: true },
  { label: "HIPAA-aligned redaction", free: false, pro: true, enterprise: true },
  { label: "ISO 27001 controls supported", free: false, pro: true, enterprise: true },
  { label: "Hash-chain + immutable log (regulator-ready)", free: false, pro: false, enterprise: true },
  { label: "Encryption-at-rest (SQLCipher)", free: true, pro: true, enterprise: true },
  { label: "Per-action role permissions", free: true, pro: true, enterprise: true },
  { label: "Forensic audit chain", free: true, pro: true, enterprise: true },
  { label: "Sub-workflows + delegation", free: false, pro: true, enterprise: true },
  { label: "Custom verticals + prompt packs", free: false, pro: false, enterprise: true },
  { label: "BYO model keys", free: false, pro: true, enterprise: true },
  { label: "SSO / SAML", free: false, pro: false, enterprise: true },
];

interface FaqItem {
  q: string;
  a: string;
}

const FAQ: FaqItem[] = [
  {
    q: "How does per-teammate billing work?",
    a: "On Pro, every named teammate you hire (Mariah, Priya, Jamal, etc.) is $29/month. You only pay for teammates that are actively working — pause them and billing stops. Enterprise is a flat negotiated rate regardless of teammate count.",
  },
  {
    q: "Can I bring my own AI model keys?",
    a: "Yes, on Pro and Enterprise. Add your OpenAI, Anthropic, Bedrock, or self-hosted endpoint keys in Settings → Providers. SKYCOT routes through them transparently and you pay the model provider directly for inference.",
  },
  {
    q: "What happens if my teammate goes over my action limit?",
    a: "On Free, the teammate pauses and asks you to upgrade. On Pro, you get overage at $0.01 per action with an alert at 80% used. On Enterprise, limits are negotiated up-front — no surprises.",
  },
  {
    q: "Is data shared between tenants?",
    a: "No. Each customer workspace is fully isolated — every audit entry, skill run, integration call, and memory key carries the workspace stamp, and our automated checks block any code that could leak across the boundary. Tenant isolation is the load-bearing security guarantee.",
  },
  {
    q: "Can I self-host SKYCOT?",
    a: "Yes, on Enterprise. SKYCOT ships as a single binary with SQLite + SQLCipher, runs on a Mac mini or any Linux server, and includes a Docker image. The same binary you run locally runs in production — no separate self-host build.",
  },
  {
    q: "How is this different from Zapier / n8n?",
    a: "Zapier and n8n call them 'zaps' and 'workflows'. SKYCOT teammates have names, traits, and signature behaviors — they show up in Slack like a colleague, ask before they act, and earn trust over time. See /compare for the side-by-side breakdown.",
  },
];

export default function PricingPage() {
  const [open, setOpen] = useState<number | null>(0);

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .pri {
          --purple: #8b5cf6;
          --pink: #ec4899;
          --coral: #ff6b6b;
          --teal: #14b8a6;
          --ink: #1e1b4b;
          --ink-2: #3f3f46;
          --ink-3: #71717a;
          --bg: #fafaff;
          --line: #ece9f5;
          font-family: "Inter", sans-serif;
          background: var(--bg);
          color: var(--ink);
          min-height: 100vh;
          position: relative;
          letter-spacing: -0.005em;
          -webkit-font-smoothing: antialiased;
        }
        .pri::before {
          content: "";
          position: fixed;
          top: -20%;
          right: -10%;
          width: 700px;
          height: 700px;
          background: radial-gradient(circle, rgba(139, 92, 246, 0.13) 0%, transparent 70%);
          z-index: 0;
          pointer-events: none;
        }
        .pri::after {
          content: "";
          position: fixed;
          bottom: -30%;
          left: -10%;
          width: 800px;
          height: 800px;
          background: radial-gradient(circle, rgba(236, 72, 153, 0.1) 0%, transparent 70%);
          z-index: 0;
          pointer-events: none;
        }
        .pri-top,
        .pri-stage {
          position: relative;
          z-index: 1;
        }
        .pri-top {
          padding: 22px 64px;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .pri-brand {
          display: flex;
          align-items: center;
          gap: 12px;
        }
        .pri-brand-mark {
          width: 28px;
          height: 28px;
          border-radius: 7px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          box-shadow: 0 4px 12px -2px rgba(139, 92, 246, 0.3);
        }
        .pri-brand-name {
          font-family: "Audiowide", sans-serif;
          font-size: 16px;
          letter-spacing: 0.08em;
        }
        .pri-nav {
          display: flex;
          gap: 28px;
          font-size: 14px;
          font-weight: 500;
        }
        .pri-nav a {
          color: var(--ink-2);
          text-decoration: none;
        }
        .pri-nav a:hover {
          color: var(--purple);
        }
        .pri-nav a.active {
          color: var(--purple);
          font-weight: 600;
        }
        .pri-top-right {
          display: flex;
          gap: 12px;
          align-items: center;
        }
        .pri-signin {
          font-size: 14px;
          color: var(--ink);
          text-decoration: none;
          font-weight: 600;
        }
        .pri-cta-pill {
          padding: 9px 18px;
          border-radius: 999px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-size: 14px;
          font-weight: 700;
          text-decoration: none;
          box-shadow: 0 6px 18px -4px rgba(139, 92, 246, 0.45);
        }
        .pri-hero {
          padding: 64px 64px 40px;
          text-align: center;
          max-width: 880px;
          margin: 0 auto;
        }
        @media (max-width: 700px) {
          .pri-hero {
            padding: 40px 24px 24px;
          }
          .pri-top {
            padding: 16px 24px;
          }
          .pri-nav {
            display: none;
          }
        }
        .pri-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 6px 14px;
          border-radius: 999px;
          background: rgba(139, 92, 246, 0.1);
          color: var(--purple);
          margin-bottom: 24px;
        }
        .pri-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--teal);
        }
        .pri-h1 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 60px;
          line-height: 1.05;
          font-weight: 400;
          color: var(--ink);
          margin: 0 0 20px;
          letter-spacing: 0;
        }
        .pri-h1 strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .pri-sub {
          font-size: 18px;
          font-weight: 300;
          color: var(--ink-2);
          line-height: 1.55;
          margin: 0 auto;
          max-width: 620px;
        }
        @media (max-width: 700px) {
          .pri-h1 {
            font-size: 36px;
          }
        }
        .pri-tier-grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 18px;
          max-width: 1180px;
          margin: 32px auto 0;
          padding: 0 64px;
        }
        @media (max-width: 900px) {
          .pri-tier-grid {
            grid-template-columns: 1fr;
            padding: 0 24px;
          }
        }
        .pri-tier {
          background: white;
          border: 1px solid var(--line);
          border-radius: 18px;
          padding: 28px;
          display: flex;
          flex-direction: column;
          gap: 14px;
          transition: all 0.18s;
          position: relative;
        }
        .pri-tier:hover {
          transform: translateY(-3px);
          box-shadow: 0 18px 38px -14px rgba(139, 92, 246, 0.2);
          border-color: rgba(139, 92, 246, 0.3);
        }
        .pri-tier.featured {
          background: linear-gradient(180deg, #fdf4ff 0%, #ffffff 80%);
          border: 1px solid rgba(139, 92, 246, 0.3);
          box-shadow: 0 16px 32px -14px rgba(139, 92, 246, 0.25);
        }
        .pri-tier-ribbon {
          position: absolute;
          top: -10px;
          right: 20px;
          padding: 4px 12px;
          border-radius: 999px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          font-weight: 700;
          letter-spacing: 0.16em;
          text-transform: uppercase;
        }
        .pri-tier-title {
          font-family: "Audiowide", sans-serif;
          font-size: 22px;
          letter-spacing: 0.04em;
          margin: 0;
          color: var(--ink);
        }
        .pri-tier-price {
          font-family: "Audiowide", sans-serif;
          font-size: 44px;
          line-height: 1;
          color: var(--ink);
        }
        .pri-tier-price.gradient {
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .pri-tier-period {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          color: var(--ink-3);
          font-weight: 600;
          margin-top: 4px;
        }
        .pri-tier-blurb {
          font-size: 14px;
          color: var(--ink-2);
          line-height: 1.5;
          margin: 0;
        }
        .pri-tier-features {
          list-style: none;
          padding: 0;
          margin: 0;
          display: flex;
          flex-direction: column;
          gap: 10px;
          flex: 1;
        }
        .pri-tier-features li {
          display: flex;
          gap: 10px;
          align-items: flex-start;
          font-size: 13.5px;
          color: var(--ink-2);
          line-height: 1.45;
        }
        .pri-tier-features svg {
          flex-shrink: 0;
          color: var(--teal);
          margin-top: 2px;
        }
        .pri-tier-cta {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: 8px;
          padding: 13px 22px;
          border-radius: 12px;
          font-size: 14.5px;
          font-weight: 700;
          text-decoration: none;
          cursor: pointer;
          border: none;
          transition: all 0.16s;
        }
        .pri-tier-cta.primary {
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          box-shadow: 0 8px 22px -8px rgba(139, 92, 246, 0.5);
        }
        .pri-tier-cta.primary:hover {
          transform: translateY(-1px);
        }
        .pri-tier-cta.secondary {
          background: white;
          color: var(--ink);
          border: 1px solid var(--line);
        }
        .pri-section {
          padding: 80px 64px;
          max-width: 1180px;
          margin: 0 auto;
        }
        @media (max-width: 700px) {
          .pri-section {
            padding: 48px 24px;
          }
        }
        .pri-section-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          margin: 0 0 10px;
          text-align: center;
        }
        .pri-section-title {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 36px;
          font-weight: 400;
          color: var(--ink);
          text-align: center;
          margin: 0 0 40px;
          letter-spacing: 0;
          line-height: 1.1;
        }
        .pri-matrix-wrap {
          background: white;
          border-radius: 14px;
          border: 1px solid var(--line);
          /* Mobile-scroll wrap (responsive — wave 2): 4-column matrix
             has fixed 130px cells; below ~580px it overflows. Wrap
             scrolls horizontally instead of squashing. */
          overflow-x: auto;
          -webkit-overflow-scrolling: touch;
        }
        .pri-matrix {
          width: 100%;
          min-width: 580px;
          border-collapse: collapse;
          background: white;
        }
        .pri-matrix th,
        .pri-matrix td {
          padding: 14px 18px;
          text-align: left;
          font-size: 14px;
          border-bottom: 1px solid var(--line);
        }
        .pri-matrix th {
          background: rgba(139, 92, 246, 0.04);
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          font-weight: 700;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          color: var(--ink-2);
        }
        .pri-matrix th:not(:first-child),
        .pri-matrix td:not(:first-child) {
          text-align: center;
          width: 130px;
        }
        .pri-matrix tr:last-child td {
          border-bottom: none;
        }
        .pri-matrix tr:hover td {
          background: rgba(139, 92, 246, 0.02);
        }
        .pri-check {
          color: var(--teal);
          display: inline-flex;
        }
        .pri-empty {
          color: #d4d4d8;
          font-size: 16px;
          display: inline-block;
        }
        .pri-faq {
          max-width: 820px;
          margin: 0 auto;
          display: flex;
          flex-direction: column;
          gap: 10px;
        }
        .pri-faq-item {
          background: white;
          border: 1px solid var(--line);
          border-radius: 12px;
          overflow: hidden;
          transition: all 0.16s;
        }
        .pri-faq-item.open {
          border-color: rgba(139, 92, 246, 0.3);
          box-shadow: 0 12px 24px -12px rgba(139, 92, 246, 0.15);
        }
        .pri-faq-q {
          width: 100%;
          padding: 18px 22px;
          background: transparent;
          border: none;
          display: flex;
          align-items: center;
          justify-content: space-between;
          font-size: 15.5px;
          font-weight: 600;
          color: var(--ink);
          cursor: pointer;
          text-align: left;
          font-family: inherit;
        }
        .pri-faq-q svg {
          color: var(--purple);
          transition: transform 0.18s;
          flex-shrink: 0;
          margin-left: 16px;
        }
        .pri-faq-item.open .pri-faq-q svg {
          transform: rotate(180deg);
        }
        .pri-faq-a {
          padding: 0 22px 20px;
          font-size: 14.5px;
          color: var(--ink-2);
          line-height: 1.6;
        }
        .pri-final {
          margin: 16px 64px 80px;
          padding: 72px 56px;
          border-radius: 28px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b);
          color: white;
          text-align: center;
          position: relative;
          overflow: hidden;
        }
        @media (max-width: 700px) {
          .pri-final {
            margin: 16px 12px 48px;
            padding: 48px 24px;
          }
        }
        .pri-final::before {
          content: "";
          position: absolute;
          inset: 0;
          background: linear-gradient(
            115deg,
            transparent 30%,
            rgba(255, 255, 255, 0.14) 50%,
            transparent 70%
          );
          transform: translateX(-100%);
          animation: pri-shine 6s ease-in-out infinite;
          pointer-events: none;
        }
        @keyframes pri-shine {
          0%, 80%, 100% {
            transform: translateX(-100%);
          }
          90% {
            transform: translateX(100%);
          }
        }
        .pri-final h2 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 42px;
          font-weight: 400;
          color: white;
          margin: 0 0 16px;
          position: relative;
        }
        .pri-final p {
          font-size: 17px;
          color: rgba(255, 255, 255, 0.92);
          margin: 0 auto 28px;
          max-width: 560px;
          position: relative;
        }
        .pri-final-cta {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 14px 28px;
          border-radius: 12px;
          background: white;
          color: #8b5cf6;
          font-size: 15px;
          font-weight: 700;
          text-decoration: none;
          position: relative;
        }
        .pri-foot {
          padding: 32px 64px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.12em;
          text-transform: uppercase;
          font-weight: 600;
          text-align: center;
          border-top: 1px solid var(--line);
          display: flex;
          flex-direction: column;
          gap: 18px;
          align-items: center;
        }
        .pri-foot a {
          color: inherit;
          text-decoration: none;
        }
      `}</style>
      <div className="pri">
        <header className="pri-top">
          <Link href="/landing" className="pri-brand" style={{ textDecoration: "none", color: "inherit" }}>
            <div className="pri-brand-mark" />
            <div className="pri-brand-name">SKYCOT</div>
          </Link>
          <nav className="pri-nav">
            <Link href="/landing#how">How it works</Link>
            <Link href="/landing#features">Features</Link>
            <Link href="/compare">Compare</Link>
            <Link href="/customers">Customers</Link>
            <Link href="/pricing" className="active">Pricing</Link>
          </nav>
          <div className="pri-top-right">
            <Link href="/auth/signin" className="pri-signin">Sign in</Link>
            <Link href="/auth/signup" className="pri-cta-pill">Hire your first teammate</Link>
          </div>
        </header>

        <section className="pri-hero">
          <div className="pri-eyebrow">
            <span className="pri-eyebrow-dot" />
            PRICING
          </div>
          <h1 className="pri-h1">
            Hire teammates. Pay like they&apos;re <strong>real</strong>.
          </h1>
          <p className="pri-sub">
            Start free. Add a Pro tier when you&apos;re ready for production. Talk to us for enterprise.
          </p>
        </section>

        <div className="pri-tier-grid">
          {TIERS.map((t) => (
            <div key={t.tier} className={`pri-tier${t.featured ? " featured" : ""}`}>
              {t.featured && <div className="pri-tier-ribbon">Most popular</div>}
              <div>
                <h3 className="pri-tier-title">{t.title}</h3>
              </div>
              <div>
                <div className={`pri-tier-price${t.featured ? " gradient" : ""}`}>{t.price}</div>
                <div className="pri-tier-period">{t.period}</div>
              </div>
              <p className="pri-tier-blurb">{t.blurb}</p>
              <ul className="pri-tier-features">
                {t.features.map((f) => (
                  <li key={f}>
                    <Check size={16} />
                    <span>{f}</span>
                  </li>
                ))}
              </ul>
              <Link
                href={t.ctaHref}
                className={`pri-tier-cta ${t.featured ? "primary" : "secondary"}`}
              >
                {t.cta} <ArrowRight size={15} />
              </Link>
            </div>
          ))}
        </div>

        <section className="pri-section">
          <p className="pri-section-label">FEATURE MATRIX</p>
          <h2 className="pri-section-title">Compare every line.</h2>
          <div className="pri-matrix-wrap">
          <table className="pri-matrix">
            <thead>
              <tr>
                <th>Feature</th>
                <th>Free</th>
                <th>Pro</th>
                <th>Enterprise</th>
              </tr>
            </thead>
            <tbody>
              {MATRIX.map((row) => (
                <tr key={row.label}>
                  <td>{row.label}</td>
                  <td>{row.free ? <span className="pri-check"><Check size={18} /></span> : <span className="pri-empty">—</span>}</td>
                  <td>{row.pro ? <span className="pri-check"><Check size={18} /></span> : <span className="pri-empty">—</span>}</td>
                  <td>{row.enterprise ? <span className="pri-check"><Check size={18} /></span> : <span className="pri-empty">—</span>}</td>
                </tr>
              ))}
            </tbody>
          </table>
          </div>
        </section>

        <section className="pri-section">
          <p className="pri-section-label">FAQ</p>
          <h2 className="pri-section-title">Frequently asked questions.</h2>
          <div className="pri-faq">
            {FAQ.map((item, idx) => (
              <div key={item.q} className={`pri-faq-item${open === idx ? " open" : ""}`}>
                <button
                  type="button"
                  className="pri-faq-q"
                  onClick={() => setOpen(open === idx ? null : idx)}
                  aria-expanded={open === idx}
                >
                  <span>{item.q}</span>
                  <ChevronDown size={18} />
                </button>
                {open === idx && <div className="pri-faq-a">{item.a}</div>}
              </div>
            ))}
          </div>
        </section>

        <section className="pri-final">
          <h2>Ready to hire your first teammate?</h2>
          <p>No credit card. No demo call. Four minutes to your first working teammate.</p>
          <Link href="/auth/signup" className="pri-final-cta">
            Start free <ArrowRight size={16} />
          </Link>
        </section>

        <footer className="pri-foot">
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
