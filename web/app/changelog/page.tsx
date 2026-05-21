import "./page.css";

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
          "Drag-and-drop visual canvas with pan / zoom / mini-map. AI prompt bar for plain-English step proposals. Per-trigger config forms (webhook + schedule + email + form). Versioned + audit-logged on every save. Multi-branch rendering, sub-workflows, and live execution overlay are shipping next.",
        tags: ["Workflows", "UX"],
      },
      {
        date: "2026-05-20",
        version: "v1.0.4",
        title: "Team & roles UX live",
        body:
          "Member management with four roles (Owner, Admin, Operator, Viewer), invite modal with copy-link affordance, and pending-invite tracking. Per-action role permissions are now visible end-to-end — what each role can do is right there on the row.",
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
          <main id="main" tabIndex={-1}>
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
