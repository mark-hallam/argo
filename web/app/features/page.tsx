import Link from "next/link";
import {
  BrainCircuit,
  GitBranch,
  Library,
  MessagesSquare,
  Monitor,
  Users,
  type LucideIcon,
} from "lucide-react";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";

/**
 * `/features` — the six load-bearing claims, ported from the old
 * `marketing/` app during the single-app consolidation. Rebuilt with
 * `web/`'s public-page conventions (MarketingNav/Footer shell, inline
 * styles over the locked brand tokens) rather than the old marketing
 * component library. The top-nav routes "Features" to `/landing#features`
 * for the at-a-glance grid; this page is the deeper per-claim deep-dive.
 */

interface Claim {
  icon: LucideIcon;
  eyebrow: string;
  headline: string;
  body: string;
  mediaLabel: string;
  rows: string[];
}

const CLAIMS: Claim[] = [
  {
    icon: Users,
    eyebrow: "01 — Teammate, not tool",
    headline: "A worker is an instance with identity.",
    body: "Persistent name, voice, memory, and role-membership over time. You don't run a worker; you delegate to one. The same worker shows up in Slack on Monday, in Gmail on Wednesday, and on the audit log every day in between.",
    mediaLabel: "Worker identity",
    rows: [
      "worker: Maya",
      "  role: marketing intern",
      "  memory: 142 facts",
      "  hires: 2025-08-14",
      "  approval rate: 96%",
    ],
  },
  {
    icon: BrainCircuit,
    eyebrow: "02 — Delegation is the unit",
    headline: "Hire for a role, not for one task.",
    body: '"Be my marketing intern" or "handle inbound recruiter messages" is the contract: a charter, a set of duties, KPIs. The worker decides which actions to run; you decide whether to approve them.',
    mediaLabel: "Role charter",
    rows: [
      "role: inbound-recruiter-triage",
      "  duty: classify new messages (hourly)",
      "  duty: draft polite declines (on-demand)",
      "  duty: forward genuine leads to founder",
      "  KPI: reply within 4h business",
    ],
  },
  {
    icon: GitBranch,
    eyebrow: "03 — The platform builds the worker",
    headline: "No hand-authoring TOML.",
    body: "Describe the role in plain English. SKYCOT generates the system prompt, the action vocabulary, the memory schema, and the eval seeds. Iterate on the description; the worker updates underneath you.",
    mediaLabel: "Worker generation",
    rows: [
      '$ skycot hire "customer support, weekends"',
      "→ generated vertical: customer-support",
      "→ 12 actions, 3 escalation paths",
      "→ 8 eval seeds, all passing",
      "Worker ready. Hire now? (y/n)",
    ],
  },
  {
    icon: Library,
    eyebrow: "04 — Skills are shared and learned",
    headline: "A library of moves your team can teach.",
    body: "Workers draw from a shared skills library, and when one of yours figures out a novel sequence that works, that becomes a reusable skill. Provenance is tracked. Trust is earned. Cross-tenant sharing is opt-in.",
    mediaLabel: "Skill library",
    rows: [
      "skills:",
      "  draft-cold-email          ★ 247 uses",
      "  reconcile-stripe-payouts  ★ 84 uses",
      "  qualify-inbound-lead      ★ 31 uses (pending)",
      "+ Teach a new skill",
    ],
  },
  {
    icon: Monitor,
    eyebrow: "05 — Real-world execution",
    headline: "Any task a human can do on a PC.",
    body: "Two execution backends, used per-action: MCP for known APIs (Gmail, Slack, GitHub, Salesforce), and computer-use for novel desktops (internal SaaS, extract-from-PDF, anything where 'look at the screen and decide' is the only path). Workers have both.",
    mediaLabel: "Execution split",
    rows: [
      "action: send-followup-email   → mcp:gmail.send",
      "action: pull-contract-pdf     → mcp:gdrive.download",
      "action: extract-renewal-date  → computer-use",
      "action: post-to-internal-wiki → computer-use",
    ],
  },
  {
    icon: MessagesSquare,
    eyebrow: "06 — Multi-channel approval",
    headline: "Approve in the channel you live in.",
    body: "Email, Slack, SMS, push, voice, web. The worker drafts; the channel routes; you say yes or no. Auto-approval policies handle the routine. The CLI is a last-resort fallback, not the product.",
    mediaLabel: "Approval channels",
    rows: [
      "channel: slack       (default)",
      "channel: email       (out-of-hours)",
      "channel: sms         (urgent)",
      "channel: voice       (escalation)",
      "auto-approve: drafts <$50, retries, scheduled",
    ],
  },
];

function ClaimMedia({ label, rows }: { label: string; rows: string[] }) {
  return (
    <div
      role="img"
      aria-label={label}
      style={{
        borderRadius: 12,
        border: "1px solid var(--border)",
        background: "var(--card)",
        padding: 16,
        fontFamily: "var(--font-mono)",
        fontSize: 12,
        lineHeight: 1.7,
        color: "var(--muted-foreground)",
        boxShadow: "0 1px 2px rgba(0,0,0,0.04)",
      }}
    >
      {rows.map((row, i) => (
        <div key={i} style={i === 0 ? { color: "var(--foreground)" } : undefined}>
          {row}
        </div>
      ))}
    </div>
  );
}

export default function FeaturesPage() {
  return (
    <>
      <MarketingNav />
      <article
        style={{
          maxWidth: 1080,
          margin: "0 auto",
          padding: "80px 24px 40px",
          fontFamily:
            'Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif',
          color: "var(--foreground)",
          background: "var(--background)",
        }}
      >
        <header style={{ maxWidth: 720, margin: "0 auto 56px", textAlign: "center" }}>
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              letterSpacing: "0.2em",
              textTransform: "uppercase",
              color: "var(--muted-foreground)",
              margin: "0 0 16px",
            }}
          >
            Six load-bearing claims
          </p>
          <h1
            style={{
              fontFamily: "var(--font-audiowide)",
              fontSize: 44,
              fontWeight: 400,
              lineHeight: 1.08,
              margin: "0 0 18px",
              backgroundImage: "var(--brand-gradient)",
              backgroundClip: "text",
              WebkitBackgroundClip: "text",
              color: "transparent",
              WebkitTextFillColor: "transparent",
            }}
          >
            What it means to hire an AI worker.
          </h1>
          <p style={{ fontSize: 18, lineHeight: 1.55, margin: "0 0 28px", color: "var(--muted-foreground)" }}>
            Every architectural decision in SKYCOT traces back to one of six
            commitments. Marketing words are cheap; these are the ones we hold
            ourselves to.
          </p>
          <div style={{ display: "flex", gap: 12, justifyContent: "center", flexWrap: "wrap" }}>
            <Link
              href="/auth/signup"
              style={{
                display: "inline-block",
                padding: "12px 24px",
                backgroundImage: "var(--brand-gradient)",
                color: "var(--brand-primary-foreground)",
                textDecoration: "none",
                borderRadius: 8,
                fontWeight: 600,
                fontSize: 15,
              }}
            >
              Try it now
            </Link>
            <Link
              href="/pricing"
              style={{
                display: "inline-block",
                padding: "12px 24px",
                border: "1px solid var(--border)",
                color: "var(--foreground)",
                textDecoration: "none",
                borderRadius: 8,
                fontWeight: 600,
                fontSize: 15,
              }}
            >
              See pricing
            </Link>
          </div>
        </header>

        <div style={{ display: "flex", flexDirection: "column", gap: 56 }}>
          {CLAIMS.map((claim, i) => {
            const Icon = claim.icon;
            const reverse = i % 2 === 1;
            return (
              <section
                key={claim.eyebrow}
                style={{
                  display: "grid",
                  gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))",
                  gap: 32,
                  alignItems: "center",
                }}
              >
                <div style={{ order: reverse ? 2 : 1 }}>
                  <div
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      justifyContent: "center",
                      width: 40,
                      height: 40,
                      borderRadius: 10,
                      background:
                        "color-mix(in oklch, var(--brand-primary) 12%, transparent)",
                      color: "var(--brand-primary)",
                      marginBottom: 14,
                    }}
                  >
                    <Icon size={20} aria-hidden />
                  </div>
                  <p
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 12,
                      letterSpacing: "0.1em",
                      textTransform: "uppercase",
                      color: "var(--brand-primary)",
                      margin: "0 0 8px",
                    }}
                  >
                    {claim.eyebrow}
                  </p>
                  <h2 style={{ fontSize: 26, fontWeight: 700, lineHeight: 1.2, margin: "0 0 12px" }}>
                    {claim.headline}
                  </h2>
                  <p style={{ fontSize: 15, lineHeight: 1.6, margin: 0, color: "var(--muted-foreground)" }}>
                    {claim.body}
                  </p>
                </div>
                <div style={{ order: reverse ? 1 : 2 }}>
                  <ClaimMedia label={claim.mediaLabel} rows={claim.rows} />
                </div>
              </section>
            );
          })}
        </div>
      </article>

      <section
        style={{
          maxWidth: 720,
          margin: "0 auto",
          padding: "64px 24px",
          textAlign: "center",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 20,
          borderTop: "1px solid var(--border)",
        }}
      >
        <h2 style={{ fontSize: 30, fontWeight: 600, lineHeight: 1.2, color: "var(--foreground)", margin: 0 }}>
          The product earns trust by being correct.
        </h2>
        <p style={{ fontSize: 16, lineHeight: 1.6, color: "var(--muted-foreground)", margin: 0 }}>
          We don&apos;t ask you to take any of this on faith. Spin up a worker
          and watch what it does.
        </p>
        <Link
          href="/auth/signup"
          style={{
            display: "inline-block",
            padding: "12px 24px",
            backgroundImage: "var(--brand-gradient)",
            color: "var(--brand-primary-foreground)",
            textDecoration: "none",
            borderRadius: 8,
            fontWeight: 600,
            fontSize: 15,
          }}
        >
          Hire your first worker
        </Link>
      </section>
      <MarketingFooter />
    </>
  );
}
