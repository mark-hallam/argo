"use client";

import Link from "next/link";
import { ArrowRight, Check } from "lucide-react";

import { VERTICAL_NAV, type VerticalConfig } from "../_data/verticals";

/**
 * Shared marketing shell for the six `/for/<vertical>` landing pages.
 *
 * Inherits the locked D3-with-Audiowide register from `/landing`
 * (`web/app/landing/page.tsx`). The only per-vertical deviations are
 * (a) vertical-tailored copy supplied by `vertical`, and (b) a subtle
 * accent-color tint on the hero radial gradient. Brand identity stays
 * locked — purple→pink gradient on Audiowide stays the headline accent.
 */
export function VerticalLandingShell({ vertical }: { vertical: VerticalConfig }) {
  const accent = vertical.accent;

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .vland {
          --purple: var(--brand-primary);
          --pink: var(--brand-pink);
          --coral: var(--brand-coral);
          --teal: var(--brand-teal);
          --ink: var(--foreground);
          --ink-2: var(--muted-foreground);
          --ink-3: var(--muted-foreground);
          --bg: var(--background);
          --line: var(--border);
          font-family: "Inter", sans-serif;
          background: var(--bg);
          color: var(--ink);
          min-height: 100vh;
          position: relative;
          letter-spacing: -0.005em;
          -webkit-font-smoothing: antialiased;
        }
        .vland::before {
          content: "";
          position: fixed;
          top: -20%;
          right: -10%;
          width: 700px;
          height: 700px;
          background: radial-gradient(circle, var(--accent-tint, rgba(139, 92, 246, 0.13)) 0%, transparent 70%);
          z-index: 0;
          pointer-events: none;
        }
        .vland::after {
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
        .vland-top,
        .vland-stage {
          position: relative;
          z-index: 1;
        }
        .vland-top {
          padding: 22px 64px;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .vland-brand {
          display: flex;
          align-items: center;
          gap: 12px;
        }
        .vland-brand-mark {
          width: 28px;
          height: 28px;
          border-radius: 7px;
          background: linear-gradient(135deg, var(--purple), var(--pink));
          box-shadow: 0 4px 12px -2px rgba(139, 92, 246, 0.3);
        }
        .vland-brand-name {
          font-family: "Audiowide", sans-serif;
          font-size: 16px;
          letter-spacing: 0.08em;
        }
        .vland-nav {
          display: flex;
          gap: 28px;
          font-size: 14px;
          font-weight: 500;
          align-items: center;
        }
        .vland-nav a {
          color: var(--ink-2);
          text-decoration: none;
        }
        .vland-nav a:hover {
          color: var(--purple);
        }
        .vland-for-trigger {
          background: none;
          border: none;
          cursor: pointer;
          padding: 0;
          font: inherit;
          color: var(--ink-2);
          display: inline-flex;
          gap: 6px;
          align-items: center;
        }
        .vland-for-trigger:hover {
          color: var(--purple);
        }
        .vland-for-wrap {
          position: relative;
        }
        .vland-for-wrap:hover .vland-for-menu,
        .vland-for-wrap:focus-within .vland-for-menu {
          display: grid;
        }
        .vland-for-menu {
          display: none;
          position: absolute;
          top: calc(100% + 12px);
          left: 50%;
          transform: translateX(-50%);
          background: white;
          border: 1px solid var(--line);
          border-radius: 14px;
          padding: 10px;
          min-width: 280px;
          grid-template-columns: 1fr;
          gap: 4px;
          box-shadow: 0 18px 40px -16px rgba(30, 27, 75, 0.18);
          z-index: 50;
        }
        .vland-for-item {
          padding: 10px 12px;
          border-radius: 10px;
          text-decoration: none;
          color: var(--ink);
        }
        .vland-for-item:hover {
          background: rgba(139, 92, 246, 0.06);
        }
        .vland-for-item-label {
          font-weight: 600;
          font-size: 14px;
          margin: 0;
        }
        .vland-for-item-tag {
          margin: 2px 0 0;
          font-size: 12.5px;
          color: var(--ink-3);
        }
        .vland-top-right {
          display: flex;
          gap: 12px;
          align-items: center;
        }
        .vland-signin {
          font-size: 14px;
          color: var(--ink);
          text-decoration: none;
          font-weight: 600;
        }
        .vland-cta-pill {
          padding: 9px 18px;
          border-radius: 999px;
          background: linear-gradient(135deg, var(--purple), var(--pink));
          color: white;
          font-size: 14px;
          font-weight: 700;
          text-decoration: none;
          box-shadow: 0 6px 18px -4px rgba(139, 92, 246, 0.45);
        }
        .vland-hero {
          padding: 80px 64px 56px;
          max-width: 1080px;
          margin: 0 auto;
          text-align: center;
        }
        @media (max-width: 900px) {
          .vland-hero {
            padding: 48px 24px 32px;
          }
          .vland-top {
            padding: 16px 24px;
          }
          .vland-nav {
            display: none;
          }
        }
        .vland-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 6px 14px;
          border-radius: 999px;
          background: var(--accent-eyebrow-bg, rgba(139, 92, 246, 0.1));
          color: var(--accent-eyebrow-fg);
          margin-bottom: 24px;
        }
        .vland-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--coral);
        }
        .vland-h1 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 64px;
          line-height: 1.05;
          font-weight: 400;
          color: var(--ink);
          margin: 0 0 22px;
          letter-spacing: 0;
        }
        .vland-h1 strong {
          font-weight: 400;
          background: linear-gradient(135deg, var(--purple), var(--pink), var(--coral));
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .vland-sub {
          font-size: 18px;
          font-weight: 300;
          color: var(--ink-2);
          line-height: 1.55;
          margin: 0 auto 32px;
          max-width: 640px;
        }
        .vland-actions {
          display: flex;
          gap: 14px;
          align-items: center;
          justify-content: center;
          flex-wrap: wrap;
        }
        .vland-btn-primary {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 14px 24px;
          border-radius: 12px;
          background: linear-gradient(135deg, var(--purple), var(--pink));
          color: white;
          font-size: 15px;
          font-weight: 700;
          border: none;
          cursor: pointer;
          text-decoration: none;
          box-shadow: 0 8px 24px -6px rgba(139, 92, 246, 0.45);
        }
        .vland-btn-secondary {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 14px 22px;
          border-radius: 12px;
          background: transparent;
          color: var(--ink);
          font-size: 15px;
          font-weight: 600;
          border: 1px solid var(--line);
          cursor: pointer;
          text-decoration: none;
        }
        .vland-trust {
          margin-top: 36px;
          display: flex;
          gap: 24px;
          flex-wrap: wrap;
          justify-content: center;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.06em;
          text-transform: uppercase;
          font-weight: 600;
        }
        .vland-trust-item {
          display: inline-flex;
          gap: 6px;
          align-items: center;
        }
        .vland-trust-dot {
          width: 5px;
          height: 5px;
          border-radius: 50%;
          background: var(--teal);
        }
        .vland-section {
          padding: 72px 64px;
          max-width: 1180px;
          margin: 0 auto;
          position: relative;
          z-index: 1;
        }
        @media (max-width: 700px) {
          .vland-section {
            padding: 48px 24px;
          }
          .vland-h1 {
            font-size: 42px;
          }
          .vland-section-title {
            font-size: 30px;
          }
        }
        .vland-section-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          margin: 0 0 10px;
          text-align: center;
        }
        .vland-section-title {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 38px;
          font-weight: 400;
          color: var(--ink);
          text-align: center;
          margin: 0 0 48px;
          line-height: 1.1;
        }
        .vland-section-title strong {
          font-weight: 400;
          background: linear-gradient(135deg, var(--purple), var(--pink));
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .vland-cards {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 18px;
        }
        @media (max-width: 900px) {
          .vland-cards {
            grid-template-columns: 1fr;
          }
        }
        .vland-card {
          background: white;
          border: 1px solid var(--line);
          border-radius: 16px;
          padding: 26px;
          transition: all 0.18s;
        }
        .vland-card:hover {
          border-color: rgba(139, 92, 246, 0.3);
          transform: translateY(-2px);
          box-shadow: 0 14px 28px -12px rgba(139, 92, 246, 0.18);
        }
        .vland-card-icon {
          width: 40px;
          height: 40px;
          border-radius: 10px;
          background: linear-gradient(135deg, var(--purple), var(--pink));
          color: white;
          display: grid;
          place-items: center;
          margin-bottom: 14px;
        }
        .vland-card-title {
          font-weight: 600;
          font-size: 17px;
          color: var(--ink);
          margin: 0 0 8px;
        }
        .vland-card-blurb {
          font-size: 14px;
          color: var(--ink-2);
          line-height: 1.55;
          margin: 0;
        }
        .vland-flow {
          background: white;
          border: 1px solid var(--line);
          border-radius: 18px;
          padding: 28px;
          max-width: 760px;
          margin: 0 auto;
        }
        .vland-flow-name {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--accent-eyebrow-fg);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin: 0 0 6px;
        }
        .vland-flow-blurb {
          font-size: 15px;
          color: var(--ink-2);
          margin: 0 0 20px;
        }
        .vland-flow-step {
          display: grid;
          grid-template-columns: 110px 1fr;
          gap: 12px;
          padding: 12px 0;
          border-top: 1px solid var(--line);
          align-items: center;
        }
        .vland-flow-step:first-of-type {
          border-top: none;
        }
        .vland-flow-actor {
          font-family: "JetBrains Mono", monospace;
          font-size: 11.5px;
          color: var(--accent-eyebrow-fg);
          letter-spacing: 0.1em;
          text-transform: uppercase;
          font-weight: 700;
        }
        .vland-flow-action {
          font-size: 14.5px;
          color: var(--ink);
        }
        .vland-flow-link {
          display: inline-flex;
          gap: 6px;
          align-items: center;
          margin-top: 20px;
          font-size: 13.5px;
          font-weight: 600;
          color: var(--purple);
          text-decoration: none;
        }
        .vland-quote {
          max-width: 720px;
          margin: 0 auto;
          background: white;
          border: 1px solid var(--line);
          border-radius: 18px;
          padding: 36px;
          text-align: center;
        }
        .vland-quote-mark {
          font-family: "Audiowide", sans-serif;
          font-size: 44px;
          line-height: 1;
          background: linear-gradient(135deg, var(--purple), var(--pink));
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
          margin-bottom: 8px;
        }
        .vland-quote-text {
          font-size: 19px;
          color: var(--ink);
          line-height: 1.5;
          margin: 0 0 22px;
          font-weight: 300;
        }
        .vland-quote-byline {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.1em;
          text-transform: uppercase;
          font-weight: 600;
        }
        .vland-cta-band {
          margin: 24px 64px 0;
          padding: 56px 48px;
          border-radius: 24px;
          background: linear-gradient(135deg, var(--purple), var(--pink), var(--coral));
          color: white;
          text-align: center;
          position: relative;
          overflow: hidden;
        }
        @media (max-width: 700px) {
          .vland-cta-band {
            margin: 16px 12px 0;
            padding: 36px 24px;
          }
        }
        .vland-cta-band::before {
          content: "";
          position: absolute;
          inset: 0;
          background: linear-gradient(115deg, transparent 30%, rgba(255, 255, 255, 0.14) 50%, transparent 70%);
          transform: translateX(-100%);
          animation: vland-shine 6s ease-in-out infinite;
          pointer-events: none;
        }
        @keyframes vland-shine {
          0%, 80%, 100% { transform: translateX(-100%); }
          90% { transform: translateX(100%); }
        }
        .vland-cta-band h2 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 34px;
          font-weight: 400;
          color: white;
          margin: 0 0 12px;
          position: relative;
        }
        .vland-cta-band p {
          font-size: 15px;
          color: rgba(255, 255, 255, 0.92);
          margin: 0 auto 24px;
          max-width: 520px;
          position: relative;
        }
        .vland-cta-band a {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 14px 28px;
          border-radius: 12px;
          background: white;
          color: var(--purple);
          font-size: 15px;
          font-weight: 700;
          text-decoration: none;
          position: relative;
        }
        .vland-foot {
          padding: 32px 64px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.12em;
          text-transform: uppercase;
          font-weight: 600;
          text-align: center;
          border-top: 1px solid var(--line);
        }
        .vland-foot-links {
          display: flex;
          gap: 24px;
          flex-wrap: wrap;
          justify-content: center;
          margin-bottom: 18px;
        }
        .vland-foot-links a {
          color: inherit;
          text-decoration: none;
        }
      `}</style>
      <div
        className="vland"
        style={
          {
            "--accent-tint": hexToTint(accent, 0.13),
            "--accent-eyebrow-bg": hexToTint(accent, 0.12),
            "--accent-eyebrow-fg": accent,
          } as React.CSSProperties
        }
      >
        <header className="vland-top">
          <Link href="/landing" className="vland-brand" style={{ textDecoration: "none", color: "inherit" }}>
            <div className="vland-brand-mark" />
            <div className="vland-brand-name">SKYCOT</div>
          </Link>
          <nav className="vland-nav">
            <Link href="/landing#how">How it works</Link>
            <span className="vland-for-wrap">
              <button className="vland-for-trigger" type="button" aria-haspopup="true">
                For <ArrowRight size={12} style={{ transform: "rotate(90deg)" }} />
              </button>
              <div className="vland-for-menu" role="menu">
                {VERTICAL_NAV.map((v) => (
                  <Link key={v.slug} href={`/for/${v.slug}`} className="vland-for-item" role="menuitem">
                    <p className="vland-for-item-label">{v.label}</p>
                    <p className="vland-for-item-tag">{v.tagline}</p>
                  </Link>
                ))}
              </div>
            </span>
            <Link href="/compare">Compare</Link>
            <Link href="/customers">Customers</Link>
            <Link href="/pricing">Pricing</Link>
          </nav>
          <div className="vland-top-right">
            <Link href="/auth/signin" className="vland-signin">
              Sign in
            </Link>
            <Link href="/auth/signup" className="vland-cta-pill">
              Hire your first worker
            </Link>
          </div>
        </header>

        {/* Hero */}
        <section className="vland-hero">
          <div className="vland-eyebrow">
            <span className="vland-eyebrow-dot" />
            {vertical.eyebrow}
          </div>
          <h1 className="vland-h1">
            {vertical.headline.lead} <strong>{vertical.headline.accent}</strong>
            {vertical.headline.tail ? <> {vertical.headline.tail}</> : null}
          </h1>
          <p className="vland-sub">{vertical.subhead}</p>
          <div className="vland-actions">
            <Link href="/auth/signup" className="vland-btn-primary">
              Start free <ArrowRight size={16} />
            </Link>
            <Link href={`/workflows/templates?vertical=${vertical.slug}`} className="vland-btn-secondary">
              See it work
            </Link>
          </div>
          <div className="vland-trust">
            <span className="vland-trust-item">
              <span className="vland-trust-dot" />
              SOC 2 TYPE II
            </span>
            <span className="vland-trust-item">
              <span className="vland-trust-dot" />
              GDPR · HIPAA · AFSL
            </span>
            <span className="vland-trust-item">
              <span className="vland-trust-dot" />
              2,847 teams this month
            </span>
          </div>
        </section>

        {/* 3 workflow cards */}
        <section className="vland-section">
          <p className="vland-section-label">WHAT THEY DO</p>
          <h2 className="vland-section-title">
            {vertical.workflowHeading.lead} <strong>{vertical.workflowHeading.accent}</strong>.
          </h2>
          <div className="vland-cards">
            {vertical.workflows.map((w) => (
              <div key={w.title} className="vland-card">
                <div className="vland-card-icon">
                  <Check size={20} />
                </div>
                <h3 className="vland-card-title">{w.title}</h3>
                <p className="vland-card-blurb">{w.outcome}</p>
              </div>
            ))}
          </div>
        </section>

        {/* Signature workflow showcase */}
        <section className="vland-section">
          <p className="vland-section-label">HERE&apos;S HOW IT RUNS</p>
          <h2 className="vland-section-title">
            One <strong>signature workflow</strong>, end-to-end.
          </h2>
          <div className="vland-flow">
            <p className="vland-flow-name">{vertical.signatureWorkflow.name}</p>
            <p className="vland-flow-blurb">{vertical.signatureWorkflow.blurb}</p>
            {vertical.signatureWorkflow.steps.map((s, i) => (
              <div key={`${s.actor}-${i}`} className="vland-flow-step">
                <div className="vland-flow-actor">{s.actor}</div>
                <div className="vland-flow-action">{s.action}</div>
              </div>
            ))}
            <Link href={`/workflows/templates?vertical=${vertical.slug}`} className="vland-flow-link">
              Browse all {vertical.slug} templates <ArrowRight size={14} />
            </Link>
          </div>
        </section>

        {/* Customer quote (placeholder for v1; marked for find-replace) */}
        <section className="vland-section">
          <p className="vland-section-label">WHAT TEAMS SAY</p>
          <h2 className="vland-section-title">
            From the <strong>early hires</strong>.
          </h2>
          <article className="vland-quote" data-pending-quote="true">
            <div className="vland-quote-mark">&ldquo;</div>
            <p className="vland-quote-text">{vertical.customerQuote.text}</p>
            <p className="vland-quote-byline">
              {vertical.customerQuote.name} · {vertical.customerQuote.role} · {vertical.customerQuote.company}
            </p>
          </article>
        </section>

        {/* CTA strip */}
        <section className="vland-cta-band">
          <h2>{vertical.ctaStrip.headline}</h2>
          <p>{vertical.ctaStrip.sub}</p>
          <Link href="/auth/signup">
            Start free <ArrowRight size={16} />
          </Link>
        </section>

        {/* Footer */}
        <footer className="vland-foot">
          <div className="vland-foot-links">
            <Link href="/landing">SKYCOT</Link>
            <Link href="/compare">Compare</Link>
            <Link href="/customers">Customers</Link>
            <Link href="/pricing">Pricing</Link>
            <a href="https://github.com/mark-hallam/skycot-packs">GitHub</a>
            <Link href="/docs">Docs</Link>
            <a href="mailto:hello@skycot.com">Contact</a>
          </div>
          <div>© 2026 SKYCOT · Made in Melbourne · skycot.com</div>
        </footer>
      </div>
    </>
  );
}

/** Convert a #rrggbb hex to an `rgba()` tint at the requested alpha. */
function hexToTint(hex: string, alpha: number): string {
  const m = /^#?([a-f0-9]{6})$/i.exec(hex);
  if (!m) return `rgba(139, 92, 246, ${alpha})`;
  const n = parseInt(m[1], 16);
  const r = (n >> 16) & 0xff;
  const g = (n >> 8) & 0xff;
  const b = n & 0xff;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
