"use client";

import Link from "next/link";
import {
  ArrowRight,
  Rocket,
  GitBranch,
  Sparkles,
  Shield,
  Code2,
  Plug,
  BookOpen,
  Lightbulb,
} from "lucide-react";
import type { ReactNode } from "react";

/**
 * Public docs hub — `/docs`, exempt from auth middleware.
 *
 * Brand-loud editorial register matching `/landing`, `/compare`, and
 * `/changelog`: Audiowide hero, JetBrains Mono eyebrow pill, purple →
 * pink gradient emphasis on hero strong word, warm radial-blob backdrop.
 *
 * Layout: hero + 2x3 category-card grid + featured-guides band + help
 * footer with support mailto.
 *
 * Cards link to `/docs/<slug>` pseudo-routes — those will 404 gracefully
 * via the existing `not-found.tsx`. CMS-backed doc content arrives in a
 * follow-up brief.
 */

interface DocCategory {
  slug: string;
  icon: ReactNode;
  title: string;
  blurb: string;
  count: string;
}

const CATEGORIES: DocCategory[] = [
  {
    slug: "getting-started",
    icon: <Rocket size={26} />,
    title: "Getting Started",
    blurb: "Hire your first AI teammate in four minutes. Onboarding tour, your first workflow, and the trust ratchet explained.",
    count: "8 guides",
  },
  {
    slug: "workflows",
    icon: <GitBranch size={26} />,
    title: "Workflows",
    blurb: "Build, edit, and version your AI workflows. Visual canvas, plain-English editing, multi-branch logic, and approval gates.",
    count: "14 guides",
  },
  {
    slug: "skills",
    icon: <Sparkles size={26} />,
    title: "Skills",
    blurb: "Add new skills to your teammates. The skills marketplace, authoring custom skills, and tenant-scoped skill libraries.",
    count: "11 guides",
  },
  {
    slug: "audit",
    icon: <Shield size={26} />,
    title: "Audit & Compliance",
    blurb: "Forensic-grade audit chain on every action. Per-action role permissions, workspace isolation, SOC 2-ready evidence, kill-switch ops.",
    count: "9 guides",
  },
  {
    slug: "api",
    icon: <Code2 size={26} />,
    title: "API Reference",
    blurb: "REST (Representational State Transfer — a way for programs to talk to each other over the web) plus WebSocket APIs for programmatic access.",
    count: "REST + WS",
  },
  {
    slug: "integrations",
    icon: <Plug size={26} />,
    title: "Integrations",
    blurb: "Slack, Zendesk, Salesforce, Workday, HubSpot, Linear, GitHub, Stripe, and 18+ more connectors with approval-aware wiring.",
    count: "26 connectors",
  },
];

interface FeaturedGuide {
  slug: string;
  title: string;
  read: string;
  icon: ReactNode;
}

const FEATURED: FeaturedGuide[] = [
  {
    slug: "getting-started/hire-first-teammate",
    title: "Hire your first AI teammate in 4 minutes",
    read: "4 min read",
    icon: <Rocket size={18} />,
  },
  {
    slug: "workflows/visual-canvas",
    title: "The workflow builder, end to end",
    read: "9 min read",
    icon: <GitBranch size={18} />,
  },
  {
    slug: "audit/trust-ratchet",
    title: "How the trust ratchet promotes workers",
    read: "6 min read",
    icon: <Shield size={18} />,
  },
  {
    slug: "integrations/slack-approvals",
    title: "Setting up Slack approval gates",
    read: "5 min read",
    icon: <Plug size={18} />,
  },
];

export default function DocsHubPage() {
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .dh {
          --purple: #8b5cf6;
          --pink: #ec4899;
          --coral: #ff6b6b;
          --teal: #14b8a6;
          --ink: #1e1b4b;
          --ink-2: #3f3f46;
          --ink-3: #52525b;
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
        .dh::before {
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
        .dh::after {
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
        .dh-top,
        .dh-stage {
          position: relative;
          z-index: 1;
        }
        .dh-top {
          padding: 22px 64px;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .dh-brand {
          display: flex;
          align-items: center;
          gap: 12px;
          text-decoration: none;
          color: inherit;
        }
        .dh-brand-mark {
          width: 28px;
          height: 28px;
          border-radius: 7px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          box-shadow: 0 4px 12px -2px rgba(139, 92, 246, 0.3);
        }
        .dh-brand-name {
          font-family: "Audiowide", sans-serif;
          font-size: 16px;
          letter-spacing: 0.08em;
        }
        .dh-nav {
          display: flex;
          gap: 28px;
          font-size: 14px;
          font-weight: 500;
        }
        .dh-nav a {
          color: var(--ink-2);
          text-decoration: none;
        }
        .dh-nav a:hover {
          color: var(--purple);
        }
        .dh-cta-pill {
          padding: 9px 18px;
          border-radius: 999px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-size: 14px;
          font-weight: 700;
          text-decoration: none;
          box-shadow: 0 6px 18px -4px rgba(139, 92, 246, 0.45);
        }
        .dh-hero {
          padding: 72px 64px 24px;
          max-width: 1180px;
          margin: 0 auto;
          text-align: center;
        }
        .dh-hero-eyebrow {
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
        .dh-hero-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--coral);
        }
        .dh-hero-h1 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 56px;
          line-height: 1.05;
          font-weight: 400;
          color: var(--ink);
          margin: 0 auto 18px;
          letter-spacing: 0;
          max-width: 820px;
        }
        .dh-hero-h1 strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .dh-hero-sub {
          font-size: 18px;
          font-weight: 300;
          color: var(--ink-2);
          line-height: 1.55;
          margin: 0 auto 24px;
          max-width: 620px;
        }
        .dh-hero-search {
          max-width: 580px;
          margin: 8px auto 0;
          display: flex;
          align-items: center;
          gap: 10px;
          padding: 12px 18px;
          background: white;
          border: 1px solid var(--line);
          border-radius: 14px;
          box-shadow: 0 8px 24px -12px rgba(139, 92, 246, 0.2);
        }
        .dh-hero-search-icon {
          width: 28px;
          height: 28px;
          border-radius: 7px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          flex-shrink: 0;
        }
        .dh-hero-search input {
          flex: 1;
          border: none;
          outline: none;
          font-family: "Inter", sans-serif;
          font-size: 15px;
          color: var(--ink);
          background: transparent;
        }
        .dh-hero-search-kbd {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          letter-spacing: 0.08em;
          padding: 4px 8px;
          border-radius: 6px;
          background: rgba(139, 92, 246, 0.08);
          color: var(--purple);
          font-weight: 700;
        }
        .dh-stage {
          padding: 32px 64px 60px;
          max-width: 1180px;
          margin: 0 auto;
        }
        @media (max-width: 900px) {
          .dh-top {
            padding: 16px 24px;
          }
          .dh-hero {
            padding: 48px 24px 16px;
          }
          .dh-hero-h1 {
            font-size: 36px;
          }
          .dh-stage {
            padding: 24px 20px 48px;
          }
          .dh-nav {
            display: none;
          }
        }
        .dh-grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 18px;
        }
        @media (max-width: 1000px) {
          .dh-grid {
            grid-template-columns: repeat(2, 1fr);
          }
        }
        @media (max-width: 640px) {
          .dh-grid {
            grid-template-columns: 1fr;
          }
        }
        .dh-card {
          background: white;
          border: 1px solid var(--line);
          border-radius: 18px;
          padding: 24px 24px 22px;
          text-decoration: none;
          color: inherit;
          display: flex;
          flex-direction: column;
          gap: 12px;
          transition: all 0.18s;
          position: relative;
          overflow: hidden;
        }
        .dh-card:hover {
          border-color: rgba(139, 92, 246, 0.35);
          transform: translateY(-2px);
          box-shadow: 0 16px 36px -16px rgba(139, 92, 246, 0.22);
        }
        .dh-card-icon {
          width: 48px;
          height: 48px;
          border-radius: 12px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          box-shadow: 0 8px 20px -10px rgba(139, 92, 246, 0.4);
        }
        .dh-card-title {
          font-family: "Inter", sans-serif;
          font-size: 20px;
          font-weight: 600;
          color: var(--ink);
          margin: 0;
          letter-spacing: -0.01em;
        }
        .dh-card-blurb {
          font-size: 14px;
          color: var(--ink-2);
          line-height: 1.55;
          margin: 0;
          flex: 1;
        }
        .dh-card-foot {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding-top: 8px;
        }
        .dh-card-count {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          color: var(--ink-3);
        }
        .dh-card-arrow {
          color: var(--purple);
          display: inline-flex;
          align-items: center;
          gap: 4px;
          font-size: 13px;
          font-weight: 700;
        }
        .dh-featured-section {
          margin-top: 56px;
        }
        .dh-section-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          color: var(--ink-3);
          margin: 0 0 10px;
          text-align: center;
        }
        .dh-section-title {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 32px;
          font-weight: 400;
          color: var(--ink);
          text-align: center;
          margin: 0 0 30px;
          letter-spacing: 0;
        }
        .dh-section-title strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .dh-featured-grid {
          display: grid;
          grid-template-columns: repeat(2, 1fr);
          gap: 14px;
        }
        @media (max-width: 720px) {
          .dh-featured-grid {
            grid-template-columns: 1fr;
          }
        }
        .dh-featured-card {
          background: white;
          border: 1px solid var(--line);
          border-radius: 14px;
          padding: 18px 20px;
          text-decoration: none;
          color: inherit;
          display: flex;
          align-items: center;
          gap: 14px;
          transition: all 0.18s;
        }
        .dh-featured-card:hover {
          border-color: rgba(139, 92, 246, 0.3);
          transform: translateX(2px);
          box-shadow: 0 10px 24px -12px rgba(139, 92, 246, 0.18);
        }
        .dh-featured-icon {
          width: 38px;
          height: 38px;
          border-radius: 9px;
          background: rgba(139, 92, 246, 0.1);
          color: var(--purple);
          display: grid;
          place-items: center;
          flex-shrink: 0;
        }
        .dh-featured-title {
          font-size: 15px;
          font-weight: 600;
          color: var(--ink);
          margin: 0 0 2px;
        }
        .dh-featured-read {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          font-weight: 600;
          color: var(--ink-3);
        }
        .dh-help-band {
          margin: 56px 64px 0;
          padding: 48px 48px;
          border-radius: 24px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b);
          color: white;
          text-align: center;
          position: relative;
          overflow: hidden;
        }
        @media (max-width: 700px) {
          .dh-help-band {
            margin: 32px 12px 0;
            padding: 32px 24px;
          }
        }
        .dh-help-band::before {
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
          animation: dh-shine 6s ease-in-out infinite;
          pointer-events: none;
        }
        @keyframes dh-shine {
          0%,
          80%,
          100% {
            transform: translateX(-100%);
          }
          90% {
            transform: translateX(100%);
          }
        }
        .dh-help-band h2 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 32px;
          font-weight: 400;
          color: white;
          margin: 0 0 12px;
          position: relative;
        }
        .dh-help-band p {
          font-size: 16px;
          color: rgba(255, 255, 255, 0.92);
          margin: 0 0 22px;
          max-width: 580px;
          margin-left: auto;
          margin-right: auto;
          position: relative;
        }
        .dh-help-actions {
          display: flex;
          gap: 12px;
          justify-content: center;
          flex-wrap: wrap;
          position: relative;
        }
        .dh-help-cta {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 12px 22px;
          border-radius: 12px;
          background: white;
          color: #8b5cf6;
          font-size: 14.5px;
          font-weight: 700;
          text-decoration: none;
        }
        .dh-help-cta.secondary {
          background: rgba(255, 255, 255, 0.18);
          color: white;
          border: 1px solid rgba(255, 255, 255, 0.4);
        }
        .dh-foot {
          padding: 56px 64px 32px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.12em;
          text-transform: uppercase;
          font-weight: 600;
          text-align: center;
          border-top: 1px solid var(--line);
          margin-top: 56px;
          display: flex;
          flex-direction: column;
          gap: 18px;
          align-items: center;
          position: relative;
          z-index: 1;
        }
        @media (max-width: 700px) {
          .dh-foot {
            padding: 32px 16px;
          }
        }
      `}</style>
      <div className="dh">
        <header className="dh-top">
          <Link href="/landing" className="dh-brand">
            <div className="dh-brand-mark" />
            <div className="dh-brand-name">SKYCOT</div>
          </Link>
          <nav className="dh-nav">
            <Link href="/landing#how">How it works</Link>
            <Link href="/landing#features">Features</Link>
            <Link href="/compare">Compare</Link>
            <Link href="/docs">Docs</Link>
            <Link href="/changelog">Changelog</Link>
          </nav>
          <Link href="/auth/signup" className="dh-cta-pill">
            Hire your first teammate
          </Link>
        </header>

        <section className="dh-hero">
          <div className="dh-hero-eyebrow">
            <span className="dh-hero-eyebrow-dot" />
            Documentation
          </div>
          <h1 className="dh-hero-h1">
            Everything you need to <strong>ship</strong>.
          </h1>
          <p className="dh-hero-sub">
            Operator guides, API (Application Programming Interface — the contract programs use to
            talk to SKYCOT) reference, workflow recipes, and integration docs.
          </p>
          <div className="dh-hero-search" role="search">
            <div className="dh-hero-search-icon">
              <BookOpen size={16} />
            </div>
            <input
              type="search"
              placeholder="Search the docs — try 'hire first teammate' or 'audit chain'"
              aria-label="Search documentation"
            />
            <span className="dh-hero-search-kbd">⌘K</span>
          </div>
        </section>

        <section className="dh-stage">
          <div className="dh-grid">
            {CATEGORIES.map((c) => (
              <Link key={c.slug} href={`/docs/${c.slug}`} className="dh-card">
                <div className="dh-card-icon">{c.icon}</div>
                <h3 className="dh-card-title">{c.title}</h3>
                <p className="dh-card-blurb">{c.blurb}</p>
                <div className="dh-card-foot">
                  <span className="dh-card-count">{c.count}</span>
                  <span className="dh-card-arrow">
                    Browse <ArrowRight size={14} />
                  </span>
                </div>
              </Link>
            ))}
          </div>

          <section className="dh-featured-section">
            <p className="dh-section-label">FEATURED GUIDES</p>
            <h2 className="dh-section-title">
              Start with the <strong>most-read</strong> ones.
            </h2>
            <div className="dh-featured-grid">
              {FEATURED.map((g) => (
                <Link key={g.slug} href={`/docs/${g.slug}`} className="dh-featured-card">
                  <div className="dh-featured-icon">{g.icon}</div>
                  <div>
                    <div className="dh-featured-title">{g.title}</div>
                    <div className="dh-featured-read">{g.read}</div>
                  </div>
                </Link>
              ))}
            </div>
          </section>
        </section>

        <section className="dh-help-band">
          <h2>Can&apos;t find what you need?</h2>
          <p>
            Our team replies within one business day. We&apos;re a small Melbourne crew building
            this in the open — talk to a human, not a bot.
          </p>
          <div className="dh-help-actions">
            <a href="mailto:support@skycot.com" className="dh-help-cta">
              <Lightbulb size={15} />
              Email support
            </a>
            <Link href="/changelog" className="dh-help-cta secondary">
              See what we shipped <ArrowRight size={14} />
            </Link>
          </div>
        </section>

        <footer className="dh-foot">
          <div style={{ display: "flex", gap: 24, flexWrap: "wrap", justifyContent: "center" }}>
            <Link href="/landing" style={{ color: "inherit", textDecoration: "none" }}>
              SKYCOT
            </Link>
            <Link href="/compare" style={{ color: "inherit", textDecoration: "none" }}>
              Compare
            </Link>
            <Link href="/docs" style={{ color: "inherit", textDecoration: "none" }}>
              Docs
            </Link>
            <Link href="/changelog" style={{ color: "inherit", textDecoration: "none" }}>
              Changelog
            </Link>
            <a href="https://github.com/mark-hallam/skycot-packs" style={{ color: "inherit", textDecoration: "none" }}>
              GitHub
            </a>
            <a href="mailto:hello@skycot.com" style={{ color: "inherit", textDecoration: "none" }}>
              Contact
            </a>
          </div>
          <div>© 2026 SKYCOT · Made in Melbourne · skycot.com</div>
        </footer>
      </div>
    </>
  );
}
