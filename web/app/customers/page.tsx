"use client";

import Link from "next/link";
import { ArrowRight } from "lucide-react";

/**
 * Public customers page — `/customers`, exempt from auth middleware.
 *
 * Brand-loud-marketing register matching `/landing` and `/pricing`.
 * 9-card customer grid + final "want to be featured?" CTA band.
 *
 * NOTE: placeholder customers; the v1 ship is pre-launch and these
 * companies are not real SKYCOT customers yet. The brief explicitly
 * asks for believable-but-generic names. Replace with real logos and
 * quotes as we onboard our first design partners.
 */

interface Customer {
  name: string;
  initials: string;
  color: string;
  industry: string;
  size: string;
  quote: string;
  worker: string;
  hours: string;
  actions: string;
  since: string;
  vertical: string;
}

const CUSTOMERS: Customer[] = [
  {
    name: "Northwind Commerce",
    initials: "NC",
    color: "linear-gradient(135deg, #8b5cf6, #ec4899)",
    industry: "E-commerce",
    size: "120 employees",
    quote: "Mariah replied to 12,400 buyers our human team didn't have to. She knows when to escalate and when to just ship the answer.",
    worker: "Mariah",
    hours: "640",
    actions: "12,400",
    since: "Jan 2026",
    vertical: "Customer Support",
  },
  {
    name: "Acmeco Retail",
    initials: "CR",
    color: "linear-gradient(135deg, #ec4899, #ff6b6b)",
    industry: "Retail",
    size: "45 stores · 380 staff",
    quote: "We hired Patrick to chase status across 12 store openings. He filed daily reports we used to spend two days a week writing.",
    worker: "Patrick",
    hours: "320",
    actions: "8,100",
    since: "Feb 2026",
    vertical: "Project Ops",
  },
  {
    name: "Skyline Engineering",
    initials: "SE",
    color: "linear-gradient(135deg, #0891b2, #14b8a6)",
    industry: "AEC / Construction",
    size: "240 employees",
    quote: "Nori drafts our weekly client updates. We review, she sends. Saves the senior engineers a full afternoon each week.",
    worker: "Nori",
    hours: "180",
    actions: "1,920",
    since: "Mar 2026",
    vertical: "Status Reporting",
  },
  {
    name: "Apex Recruiting",
    initials: "AR",
    color: "linear-gradient(135deg, #8b5cf6, #14b8a6)",
    industry: "Talent / HR Tech",
    size: "32 employees",
    quote: "Priya screens 200 candidates a week and flags the 8 we should actually talk to. The trust ratchet means she only escalates the close calls now.",
    worker: "Priya",
    hours: "410",
    actions: "9,600",
    since: "Dec 2025",
    vertical: "Recruiting",
  },
  {
    name: "Beacon Health Ops",
    initials: "BH",
    color: "linear-gradient(135deg, #ec4899, #14b8a6)",
    industry: "Healthcare",
    size: "1,200 employees",
    quote: "HIPAA-compliant out of the box. Jamal handles patient-portal tickets — every action audit-logged, every approval traced. Our compliance team finally relaxed.",
    worker: "Jamal",
    hours: "890",
    actions: "16,200",
    since: "Nov 2025",
    vertical: "Patient Support",
  },
  {
    name: "Maple Finance",
    initials: "MF",
    color: "linear-gradient(135deg, #ff6b6b, #8b5cf6)",
    industry: "Financial Services",
    size: "85 employees · AFSL holder",
    quote: "AFSL compliance was the deal-breaker. SKYCOT was the only platform that shipped with the audit chain regulators actually want.",
    worker: "Tobias",
    hours: "260",
    actions: "4,800",
    since: "Feb 2026",
    vertical: "Compliance Ops",
  },
  {
    name: "Polaris Marketing",
    initials: "PM",
    color: "linear-gradient(135deg, #14b8a6, #8b5cf6)",
    industry: "Agency / Marketing",
    size: "60 employees · 40 clients",
    quote: "Peta coordinates 14 client engagements at once. She remembers what every project needs and chases the right person without ever forgetting.",
    worker: "Peta",
    hours: "520",
    actions: "11,300",
    since: "Jan 2026",
    vertical: "Project Coordination",
  },
  {
    name: "Zenith Support",
    initials: "ZS",
    color: "linear-gradient(135deg, #ec4899, #8b5cf6)",
    industry: "B2B SaaS",
    size: "180 employees",
    quote: "Median first-response went from 4 hours to 4 minutes. Jamal handles 78% of tickets without escalation — the audit trail means we know exactly when he gets it right.",
    worker: "Jamal",
    hours: "1,120",
    actions: "23,400",
    since: "Oct 2025",
    vertical: "Customer Support",
  },
  {
    name: "Harbour Legal",
    initials: "HL",
    color: "linear-gradient(135deg, #8b5cf6, #ec4899)",
    industry: "Legal Services",
    size: "28 partners",
    quote: "Approval-first is the only way we'd put an AI on client-facing work. Every action gates through a partner. The audit log is depositioned-ready.",
    worker: "Nori",
    hours: "210",
    actions: "3,400",
    since: "Mar 2026",
    vertical: "Document Review",
  },
];

export default function CustomersPage() {
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .cus {
          --purple: #8b5cf6;
          --pink: #ec4899;
          --coral: #ff6b6b;
          --teal: #14b8a6;
          --cyan: #0891b2;
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
        .cus::before {
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
        .cus::after {
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
        .cus-top,
        .cus-stage {
          position: relative;
          z-index: 1;
        }
        .cus-top {
          padding: 22px 64px;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .cus-brand {
          display: flex;
          align-items: center;
          gap: 12px;
          text-decoration: none;
          color: inherit;
        }
        .cus-brand-mark {
          width: 28px;
          height: 28px;
          border-radius: 7px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          box-shadow: 0 4px 12px -2px rgba(139, 92, 246, 0.3);
        }
        .cus-brand-name {
          font-family: "Audiowide", sans-serif;
          font-size: 16px;
          letter-spacing: 0.08em;
        }
        .cus-nav {
          display: flex;
          gap: 28px;
          font-size: 14px;
          font-weight: 500;
        }
        .cus-nav a {
          color: var(--ink-2);
          text-decoration: none;
        }
        .cus-nav a:hover,
        .cus-nav a.active {
          color: var(--purple);
        }
        .cus-nav a.active {
          font-weight: 600;
        }
        .cus-top-right {
          display: flex;
          gap: 12px;
          align-items: center;
        }
        .cus-signin {
          font-size: 14px;
          color: var(--ink);
          text-decoration: none;
          font-weight: 600;
        }
        .cus-cta-pill {
          padding: 9px 18px;
          border-radius: 999px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-size: 14px;
          font-weight: 700;
          text-decoration: none;
          box-shadow: 0 6px 18px -4px rgba(139, 92, 246, 0.45);
        }
        .cus-hero {
          padding: 64px 64px 40px;
          text-align: center;
          max-width: 880px;
          margin: 0 auto;
        }
        @media (max-width: 700px) {
          .cus-hero {
            padding: 40px 24px 24px;
          }
          .cus-top {
            padding: 16px 24px;
          }
          .cus-nav {
            display: none;
          }
        }
        .cus-eyebrow {
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
        .cus-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--teal);
        }
        .cus-h1 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 60px;
          line-height: 1.05;
          font-weight: 400;
          color: var(--ink);
          margin: 0 0 20px;
          letter-spacing: 0;
        }
        .cus-h1 strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .cus-sub {
          font-size: 18px;
          font-weight: 300;
          color: var(--ink-2);
          line-height: 1.55;
          margin: 0 auto;
          max-width: 600px;
        }
        @media (max-width: 700px) {
          .cus-h1 {
            font-size: 36px;
          }
        }
        .cus-grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 18px;
          max-width: 1240px;
          margin: 32px auto 0;
          padding: 0 64px;
        }
        @media (max-width: 980px) {
          .cus-grid {
            grid-template-columns: repeat(2, 1fr);
            padding: 0 32px;
          }
        }
        @media (max-width: 640px) {
          .cus-grid {
            grid-template-columns: 1fr;
            padding: 0 24px;
          }
        }
        .cus-card {
          background: white;
          border: 1px solid var(--line);
          border-radius: 16px;
          padding: 24px;
          display: flex;
          flex-direction: column;
          gap: 16px;
          transition: all 0.18s;
        }
        .cus-card:hover {
          transform: translateY(-3px);
          box-shadow: 0 16px 32px -14px rgba(139, 92, 246, 0.2);
          border-color: rgba(139, 92, 246, 0.3);
        }
        .cus-card-head {
          display: flex;
          gap: 14px;
          align-items: center;
        }
        .cus-card-logo {
          width: 52px;
          height: 52px;
          border-radius: 12px;
          display: grid;
          place-items: center;
          color: white;
          font-family: "Audiowide", sans-serif;
          font-size: 17px;
          letter-spacing: 0.04em;
          flex-shrink: 0;
        }
        .cus-card-name {
          font-weight: 700;
          font-size: 16px;
          color: var(--ink);
          margin: 0;
          line-height: 1.2;
        }
        .cus-card-meta {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: var(--ink-3);
          letter-spacing: 0.1em;
          text-transform: uppercase;
          margin-top: 4px;
        }
        .cus-quote {
          font-size: 14.5px;
          color: var(--ink);
          line-height: 1.55;
          margin: 0;
          font-style: italic;
          padding-left: 14px;
          border-left: 3px solid;
          border-image: linear-gradient(180deg, #8b5cf6, #ec4899) 1;
        }
        .cus-kpis {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 8px;
          padding-top: 6px;
          border-top: 1px solid var(--line);
        }
        .cus-kpi {
          text-align: center;
        }
        .cus-kpi-val {
          font-family: "Audiowide", sans-serif;
          font-size: 18px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
          line-height: 1.1;
        }
        .cus-kpi-lbl {
          font-family: "JetBrains Mono", monospace;
          font-size: 9.5px;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          color: var(--ink-3);
          margin-top: 4px;
          font-weight: 600;
        }
        .cus-card-foot {
          display: flex;
          gap: 8px;
          align-items: center;
          padding-top: 6px;
          border-top: 1px solid var(--line);
        }
        .cus-vertical-chip {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          padding: 4px 10px;
          border-radius: 999px;
          background: rgba(139, 92, 246, 0.08);
          color: var(--purple);
          font-weight: 700;
        }
        .cus-final {
          margin: 80px 64px;
          padding: 72px 56px;
          border-radius: 28px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b);
          color: white;
          text-align: center;
          position: relative;
          overflow: hidden;
        }
        @media (max-width: 700px) {
          .cus-final {
            margin: 48px 12px;
            padding: 48px 24px;
          }
        }
        .cus-final::before {
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
          animation: cus-shine 6s ease-in-out infinite;
          pointer-events: none;
        }
        @keyframes cus-shine {
          0%, 80%, 100% {
            transform: translateX(-100%);
          }
          90% {
            transform: translateX(100%);
          }
        }
        .cus-final h2 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 42px;
          font-weight: 400;
          color: white;
          margin: 0 0 16px;
          position: relative;
        }
        .cus-final p {
          font-size: 17px;
          color: rgba(255, 255, 255, 0.92);
          margin: 0 auto 28px;
          max-width: 560px;
          position: relative;
        }
        .cus-final-cta {
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
        .cus-foot {
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
        .cus-foot a {
          color: inherit;
          text-decoration: none;
        }
      `}</style>
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
            Teams shipping with <strong>SKYCOT</strong>.
          </h1>
          <p className="cus-sub">
            300+ teams. From solo founders to Fortune 500. Hear how they&apos;re using their teammates.
          </p>
        </section>

        <div className="cus-grid">
          {CUSTOMERS.map((c) => (
            <article key={c.name} className="cus-card">
              <div className="cus-card-head">
                <div className="cus-card-logo" style={{ background: c.color }}>
                  {c.initials}
                </div>
                <div>
                  <h3 className="cus-card-name">{c.name}</h3>
                  <div className="cus-card-meta">{c.industry} · {c.size}</div>
                </div>
              </div>
              <blockquote className="cus-quote">&ldquo;{c.quote}&rdquo;</blockquote>
              <div className="cus-kpis">
                <div className="cus-kpi">
                  <div className="cus-kpi-val">{c.hours}</div>
                  <div className="cus-kpi-lbl">Hrs/mo saved</div>
                </div>
                <div className="cus-kpi">
                  <div className="cus-kpi-val">{c.actions}</div>
                  <div className="cus-kpi-lbl">Actions/mo</div>
                </div>
                <div className="cus-kpi">
                  <div className="cus-kpi-val" style={{ fontSize: 13.5 }}>{c.since}</div>
                  <div className="cus-kpi-lbl">Since</div>
                </div>
              </div>
              <div className="cus-card-foot">
                <span className="cus-vertical-chip">{c.vertical}</span>
              </div>
            </article>
          ))}
        </div>

        <section className="cus-final">
          <h2>Want to be featured?</h2>
          <p>We&apos;re onboarding design partners every week. Tell us what you&apos;d hire SKYCOT for and we&apos;ll show you how.</p>
          <a href="mailto:hello@skycot.com" className="cus-final-cta">
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
