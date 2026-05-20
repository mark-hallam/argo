"use client";

import Link from "next/link";
import { ArrowRight, Rss } from "lucide-react";

/**
 * Public changelog — `/changelog`, exempt from auth middleware.
 *
 * Brand-loud editorial register matching `/landing` and `/compare`:
 * Audiowide hero, JetBrains Mono eyebrow pills, purple → pink gradient
 * emphasis, warm radial-blob backdrop, deep-navy ink on warm-white bg.
 *
 * Layout: sticky right-side week-nav rail + chronological feed. Each
 * entry has a date pill, version tag, gradient-emphasized title, plain
 * English 2-3 sentence body, and category tags.
 *
 * V1 ships static seed content authored from BUILD_STATUS.md ledger
 * (2026-05-13 → 2026-05-20). RSS link is a stub for now; a CMS-backed
 * feed arrives in a follow-up brief.
 */

type EntryTag =
  | "UX"
  | "Workflows"
  | "Audit"
  | "Connectors"
  | "API"
  | "Brand"
  | "Security"
  | "Ops"
  | "Demo"
  | "Reliability";

interface Entry {
  date: string;
  version: string;
  title: string;
  body: string;
  tags: EntryTag[];
}

interface Week {
  label: string;
  range: string;
  entries: Entry[];
}

const WEEKS: Week[] = [
  {
    label: "This week",
    range: "May 18 – May 20, 2026",
    entries: [
      {
        date: "2026-05-20",
        version: "v1.0.5",
        title: "Class-leading visual workflow builder shipped",
        body:
          "Drag-and-drop React Flow canvas, AI prompt-to-workflow generation, multi-branch rendering, sub-workflows, and live execution overlay. Edit your AI worker's playbook the way you'd edit a Figma file. Versioned, reversible, and audit-logged on every save.",
        tags: ["Workflows", "UX"],
      },
      {
        date: "2026-05-20",
        version: "v1.0.4",
        title: "Team & roles UX live",
        body:
          "Member management with four roles (Owner, Admin, Operator, Viewer), invite modal with copy-link affordance, and pending-invite tracking. Capability-based RBAC is now visible end-to-end — what each role can do is right there on the row.",
        tags: ["UX", "Security"],
      },
      {
        date: "2026-05-20",
        version: "v1.0.3",
        title: "Personality cards on worker profiles",
        body:
          "Every AI teammate now has a personality card on their profile — trait chips, signature behaviors, what they're great at, and when to escalate. Your AI hires read like colleagues, not chatbots.",
        tags: ["UX", "Brand"],
      },
      {
        date: "2026-05-20",
        version: "v1.0.2",
        title: "Workflows catalog + visual viewer",
        body:
          "Public `/workflows` catalog with nine seeded workflows across sales, support, ops, and recruiting. Each opens into a read-only visual diagram so your team can browse what's possible before they hire.",
        tags: ["Workflows", "UX"],
      },
      {
        date: "2026-05-20",
        version: "v1.0.1",
        title: "Brand-loud heroes across Approvals, Team, Cohorts",
        body:
          "Approvals inbox, Team & roles, and Cohorts pages all got the editorial-D3 brand-consistency pass — Audiowide hero typography, warm radial-blob backdrops, and gradient-emphasis on hero phrases. The product now matches the marketing.",
        tags: ["Brand", "UX"],
      },
      {
        date: "2026-05-19",
        version: "v1.0.0",
        title: "Editorial-A auth lock-in + globals.css brand-token swap",
        body:
          "Signup, verify-email, and signin migrated to the locked editorial-A design. globals.css now ships the canonical brand tokens (purple → pink gradient, warm bg, deep-navy ink) so any new page inherits the right look by default.",
        tags: ["Brand", "Security"],
      },
    ],
  },
  {
    label: "Last week",
    range: "May 11 – May 17, 2026",
    entries: [
      {
        date: "2026-05-17",
        version: "v0.9.7",
        title: "F-12 P2 production fail-closed shipped",
        body:
          "Eval-judge connectivity failures in production now fail closed rather than silently passing the turn. Codex full-architecture audit's P2 finding cleared. Every production turn either has a verdict or a deliberate exception.",
        tags: ["Reliability", "Audit"],
      },
      {
        date: "2026-05-16",
        version: "v0.9.6",
        title: "SQLCipher enterprise default landed",
        body:
          "Encryption-at-rest for the local SQLite store via SQLCipher is now the default build path. Four-PR sequence verified, key-derivation hardened, and migration tooling carries existing tenants forward without dump-and-reload.",
        tags: ["Security", "Reliability"],
      },
      {
        date: "2026-05-15",
        version: "v0.9.5",
        title: "Audit track 10 of 11 merged",
        body:
          "Forensic-grade audit chain is now wired through ten of the eleven primitives — every action, every approval, every escalation lands in the chain with worker, tenant, capability, cost, and latency attached. The eleventh (computer-use macOS) lands with the v1 demo cut.",
        tags: ["Audit", "Reliability"],
      },
      {
        date: "2026-05-14",
        version: "v0.9.4",
        title: "Slack approval ship-gate smoke passing",
        body:
          "End-to-end smoke test for the Slack approval flow now green in CI: worker proposes → Slack message → human approves → action ships → audit row written. The full happy path runs in under twelve seconds on a cold start.",
        tags: ["Connectors", "Audit", "Demo"],
      },
      {
        date: "2026-05-13",
        version: "v0.9.3",
        title: "Mission claim 3 cost-tier proven",
        body:
          "Cost-tier routing — cheap drafts on Haiku, escalate to Sonnet on hard turns, audit the cost delta — is now proven on a live mission. Median per-task cost dropped 38% with no measurable quality regression.",
        tags: ["Reliability", "Ops"],
      },
      {
        date: "2026-05-13",
        version: "v0.9.2",
        title: "Kill-switch shipped",
        body:
          "One-tap freeze for any worker, cohort, or tenant. Stops all in-flight turns, halts approvals, and writes a kill row to the audit chain. The 'something is going wrong, stop everything' button is now real.",
        tags: ["Security", "Ops", "Audit"],
      },
    ],
  },
];

const ALL_TAGS: EntryTag[] = [
  "UX",
  "Workflows",
  "Audit",
  "Connectors",
  "API",
  "Brand",
  "Security",
  "Ops",
  "Demo",
  "Reliability",
];

const TAG_COLORS: Record<EntryTag, { bg: string; fg: string }> = {
  UX: { bg: "rgba(139, 92, 246, 0.1)", fg: "#7c3aed" },
  Workflows: { bg: "rgba(236, 72, 153, 0.1)", fg: "#db2777" },
  Audit: { bg: "rgba(20, 184, 166, 0.12)", fg: "#0d9488" },
  Connectors: { bg: "rgba(8, 145, 178, 0.1)", fg: "#0891b2" },
  API: { bg: "rgba(99, 102, 241, 0.1)", fg: "#4f46e5" },
  Brand: { bg: "rgba(255, 107, 107, 0.12)", fg: "#e11d48" },
  Security: { bg: "rgba(244, 63, 94, 0.1)", fg: "#be123c" },
  Ops: { bg: "rgba(245, 158, 11, 0.12)", fg: "#b45309" },
  Demo: { bg: "rgba(168, 85, 247, 0.12)", fg: "#9333ea" },
  Reliability: { bg: "rgba(34, 197, 94, 0.1)", fg: "#15803d" },
};

export default function ChangelogPage() {
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .cl {
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
        .cl::before {
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
        .cl::after {
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
        .cl-top,
        .cl-stage {
          position: relative;
          z-index: 1;
        }
        .cl-top {
          padding: 22px 64px;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .cl-brand {
          display: flex;
          align-items: center;
          gap: 12px;
          text-decoration: none;
          color: inherit;
        }
        .cl-brand-mark {
          width: 28px;
          height: 28px;
          border-radius: 7px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          box-shadow: 0 4px 12px -2px rgba(139, 92, 246, 0.3);
        }
        .cl-brand-name {
          font-family: "Audiowide", sans-serif;
          font-size: 16px;
          letter-spacing: 0.08em;
        }
        .cl-nav {
          display: flex;
          gap: 28px;
          font-size: 14px;
          font-weight: 500;
        }
        .cl-nav a {
          color: var(--ink-2);
          text-decoration: none;
        }
        .cl-nav a:hover {
          color: var(--purple);
        }
        .cl-top-right {
          display: flex;
          gap: 12px;
          align-items: center;
        }
        .cl-rss {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-size: 13px;
          color: var(--ink-2);
          text-decoration: none;
          font-weight: 600;
        }
        .cl-rss:hover {
          color: var(--purple);
        }
        .cl-cta-pill {
          padding: 9px 18px;
          border-radius: 999px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-size: 14px;
          font-weight: 700;
          text-decoration: none;
          box-shadow: 0 6px 18px -4px rgba(139, 92, 246, 0.45);
          display: inline-flex;
          align-items: center;
          gap: 6px;
        }
        .cl-hero {
          padding: 56px 64px 32px;
          max-width: 1180px;
          margin: 0 auto;
        }
        .cl-hero-eyebrow {
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
        .cl-hero-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--teal);
        }
        .cl-hero-h1 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 56px;
          line-height: 1.05;
          font-weight: 400;
          color: var(--ink);
          margin: 0 0 18px;
          letter-spacing: 0;
          max-width: 820px;
        }
        .cl-hero-h1 strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .cl-hero-sub {
          font-size: 18px;
          font-weight: 300;
          color: var(--ink-2);
          line-height: 1.55;
          margin: 0 0 8px;
          max-width: 620px;
        }
        .cl-stage {
          padding: 32px 64px 80px;
          max-width: 1180px;
          margin: 0 auto;
          display: grid;
          grid-template-columns: 1fr 260px;
          gap: 56px;
          align-items: start;
        }
        @media (max-width: 900px) {
          .cl-top {
            padding: 16px 24px;
          }
          .cl-hero {
            padding: 40px 24px 24px;
          }
          .cl-hero-h1 {
            font-size: 36px;
          }
          .cl-stage {
            padding: 16px 24px 48px;
            grid-template-columns: 1fr;
            gap: 32px;
          }
          .cl-nav {
            display: none;
          }
        }
        .cl-week {
          margin-bottom: 56px;
        }
        .cl-week-h {
          display: flex;
          align-items: baseline;
          gap: 14px;
          margin-bottom: 24px;
          padding-bottom: 14px;
          border-bottom: 1px solid var(--line);
        }
        .cl-week-label {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 22px;
          font-weight: 400;
          color: var(--ink);
        }
        .cl-week-range {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 600;
          color: var(--ink-3);
        }
        .cl-entries {
          display: flex;
          flex-direction: column;
          gap: 16px;
        }
        .cl-entry {
          background: white;
          border: 1px solid var(--line);
          border-radius: 16px;
          padding: 24px 26px;
          transition: all 0.18s;
        }
        .cl-entry:hover {
          border-color: rgba(139, 92, 246, 0.3);
          box-shadow: 0 12px 30px -14px rgba(139, 92, 246, 0.18);
          transform: translateY(-1px);
        }
        .cl-entry-meta {
          display: flex;
          align-items: center;
          gap: 10px;
          margin-bottom: 12px;
          flex-wrap: wrap;
        }
        .cl-date-pill {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          color: var(--ink-3);
          padding: 5px 10px;
          border-radius: 999px;
          background: rgba(139, 92, 246, 0.06);
        }
        .cl-version-tag {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.1em;
          text-transform: uppercase;
          font-weight: 700;
          color: white;
          padding: 5px 10px;
          border-radius: 999px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
        }
        .cl-entry-title {
          font-family: "Inter", sans-serif;
          font-size: 22px;
          font-weight: 600;
          color: var(--ink);
          margin: 0 0 10px;
          line-height: 1.2;
          letter-spacing: -0.01em;
        }
        .cl-entry-body {
          font-size: 14.5px;
          color: var(--ink-2);
          line-height: 1.6;
          margin: 0 0 16px;
          font-weight: 400;
        }
        .cl-entry-tags {
          display: flex;
          flex-wrap: wrap;
          gap: 6px;
        }
        .cl-entry-tag {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 4px 9px;
          border-radius: 6px;
        }
        .cl-rail {
          position: sticky;
          top: 32px;
          padding: 20px 22px;
          background: white;
          border: 1px solid var(--line);
          border-radius: 14px;
        }
        .cl-rail-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          color: var(--ink-3);
          margin-bottom: 14px;
        }
        .cl-rail-list {
          display: flex;
          flex-direction: column;
          gap: 12px;
          list-style: none;
          padding: 0;
          margin: 0 0 18px;
        }
        .cl-rail-list a {
          font-size: 13.5px;
          color: var(--ink);
          text-decoration: none;
          font-weight: 500;
          display: block;
          padding: 2px 0;
        }
        .cl-rail-list a:hover {
          color: var(--purple);
        }
        .cl-rail-list .cl-rail-range {
          display: block;
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          font-weight: 600;
          color: var(--ink-3);
          margin-top: 2px;
        }
        .cl-rail-divider {
          height: 1px;
          background: var(--line);
          margin: 16px 0;
        }
        .cl-rail-tags {
          display: flex;
          flex-wrap: wrap;
          gap: 5px;
        }
        .cl-subscribe-card {
          margin-top: 18px;
          padding: 18px;
          border-radius: 12px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
        }
        .cl-subscribe-card h4 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 16px;
          margin: 0 0 6px;
          font-weight: 400;
        }
        .cl-subscribe-card p {
          font-size: 12.5px;
          margin: 0 0 12px;
          opacity: 0.92;
          line-height: 1.5;
        }
        .cl-subscribe-card a {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-size: 13px;
          font-weight: 700;
          background: white;
          color: #8b5cf6;
          padding: 8px 14px;
          border-radius: 999px;
          text-decoration: none;
        }
        .cl-foot {
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
          position: relative;
          z-index: 1;
        }
        @media (max-width: 700px) {
          .cl-foot {
            padding: 24px 16px;
          }
        }
      `}</style>
      <div className="cl">
        <header className="cl-top">
          <Link href="/landing" className="cl-brand">
            <div className="cl-brand-mark" />
            <div className="cl-brand-name">SKYCOT</div>
          </Link>
          <nav className="cl-nav">
            <Link href="/landing#how">How it works</Link>
            <Link href="/landing#features">Features</Link>
            <Link href="/compare">Compare</Link>
            <Link href="/docs">Docs</Link>
            <Link href="/changelog">Changelog</Link>
          </nav>
          <div className="cl-top-right">
            <a href="/changelog/rss.xml" className="cl-rss" aria-label="RSS feed">
              <Rss size={14} /> RSS
            </a>
            <a href="mailto:hello@skycot.com?subject=Subscribe%20me%20to%20the%20changelog" className="cl-cta-pill">
              Get notified <ArrowRight size={14} />
            </a>
          </div>
        </header>

        <section className="cl-hero">
          <div className="cl-hero-eyebrow">
            <span className="cl-hero-eyebrow-dot" />
            Public changelog · updated weekly
          </div>
          <h1 className="cl-hero-h1">
            What we <strong>shipped this week</strong>.
          </h1>
          <p className="cl-hero-sub">
            Every meaningful change to SKYCOT lands here. Subscribe to RSS (Really Simple
            Syndication — a feed format that lets you follow updates without checking the page) for
            updates.
          </p>
        </section>

        <div className="cl-stage">
          <main>
            {WEEKS.map((w) => (
              <section key={w.label} id={w.label.toLowerCase().replace(/\s+/g, "-")} className="cl-week">
                <div className="cl-week-h">
                  <div className="cl-week-label">{w.label}</div>
                  <div className="cl-week-range">{w.range}</div>
                </div>
                <div className="cl-entries">
                  {w.entries.map((e) => (
                    <article key={e.date + e.version} className="cl-entry">
                      <div className="cl-entry-meta">
                        <span className="cl-date-pill">{e.date}</span>
                        <span className="cl-version-tag">{e.version}</span>
                      </div>
                      <h2 className="cl-entry-title">{e.title}</h2>
                      <p className="cl-entry-body">{e.body}</p>
                      <div className="cl-entry-tags">
                        {e.tags.map((t) => (
                          <span
                            key={t}
                            className="cl-entry-tag"
                            style={{ background: TAG_COLORS[t].bg, color: TAG_COLORS[t].fg }}
                          >
                            {t}
                          </span>
                        ))}
                      </div>
                    </article>
                  ))}
                </div>
              </section>
            ))}
          </main>

          <aside>
            <div className="cl-rail">
              <div className="cl-rail-label">Jump to week</div>
              <ul className="cl-rail-list">
                {WEEKS.map((w) => (
                  <li key={w.label}>
                    <a href={`#${w.label.toLowerCase().replace(/\s+/g, "-")}`}>
                      {w.label}
                      <span className="cl-rail-range">{w.range}</span>
                    </a>
                  </li>
                ))}
              </ul>
              <div className="cl-rail-divider" />
              <div className="cl-rail-label">Filter by tag</div>
              <div className="cl-rail-tags">
                {ALL_TAGS.map((t) => (
                  <span
                    key={t}
                    className="cl-entry-tag"
                    style={{ background: TAG_COLORS[t].bg, color: TAG_COLORS[t].fg }}
                  >
                    {t}
                  </span>
                ))}
              </div>
              <div className="cl-subscribe-card">
                <h4>Never miss a ship.</h4>
                <p>One short email when we ship — no marketing, no fluff. Just what changed.</p>
                <a href="mailto:hello@skycot.com?subject=Subscribe%20me%20to%20the%20changelog">
                  Get notified <ArrowRight size={13} />
                </a>
              </div>
            </div>
          </aside>
        </div>

        <footer className="cl-foot">
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
