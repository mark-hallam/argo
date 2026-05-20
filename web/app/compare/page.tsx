"use client";

import { Check, X, Minus } from "lucide-react";

/**
 * Competitive positioning page — `/compare`.
 *
 * Positions SKYCOT against the nearest competitors (Zapier, n8n,
 * Crew.ai, Lindy) on the dimensions that matter for the SKYCOT
 * mission:
 *
 *   1. "AI teammates" framing vs "automations" or "agents"
 *   2. Approval-gated by default (vs autonomous-first)
 *   3. Audit-first (vs log-after-the-fact)
 *   4. Plain-English workflow editing
 *   5. Multi-channel native (Slack / Email / Web inbox)
 *   6. Enterprise-grade (RBAC, tenant isolation, SSO-ready)
 *   7. Single-binary deploy (vs SaaS-only)
 *
 * Marketing copy reads in customer voice. The "claim" rows describe
 * the user's POV ('your AI knows when to ask') not the spec ('approval
 * gate enforcement').
 *
 * V1 ships as a static page sourced from this file; live competitive
 * intel pulls from a CMS in a follow-up brief.
 */

const COMPETITORS = ["SKYCOT", "Zapier", "n8n", "Crew.ai", "Lindy"] as const;

type Score = "yes" | "no" | "partial";

interface Dimension {
  group: string;
  question: string;
  scores: Record<(typeof COMPETITORS)[number], Score>;
  caption?: string;
}

const DIMENSIONS: Dimension[] = [
  {
    group: "Teammates, not tools",
    question: "Do they feel like real teammates (named, with personality + responsibilities)?",
    scores: {
      SKYCOT: "yes",
      Zapier: "no",
      n8n: "no",
      "Crew.ai": "partial",
      Lindy: "partial",
    },
    caption: "Zapier and n8n call them 'zaps' / 'workflows'. SKYCOT teammates have names, traits, and signature behaviors.",
  },
  {
    group: "Teammates, not tools",
    question: "Do they show up in Slack like a colleague would?",
    scores: { SKYCOT: "yes", Zapier: "partial", n8n: "no", "Crew.ai": "no", Lindy: "yes" },
  },
  {
    group: "Teammates, not tools",
    question: "Do they earn trust over time + get promoted?",
    scores: { SKYCOT: "yes", Zapier: "no", n8n: "no", "Crew.ai": "no", Lindy: "no" },
    caption: "SKYCOT's trust ratchet promotes workers from supervised → autonomous as they prove reliable.",
  },
  {
    group: "Safe by default",
    question: "Do they ask for approval before doing risky things?",
    scores: { SKYCOT: "yes", Zapier: "no", n8n: "no", "Crew.ai": "partial", Lindy: "partial" },
  },
  {
    group: "Safe by default",
    question: "Is every action audit-logged with full lineage?",
    scores: { SKYCOT: "yes", Zapier: "partial", n8n: "partial", "Crew.ai": "no", Lindy: "no" },
    caption: "SKYCOT ships forensic-grade audit chain by default. Most competitors offer audit as an enterprise add-on.",
  },
  {
    group: "Safe by default",
    question: "Can you freeze a worker in one click if something goes wrong?",
    scores: { SKYCOT: "yes", Zapier: "partial", n8n: "no", "Crew.ai": "no", Lindy: "no" },
  },
  {
    group: "Plain-English editing",
    question: "Can you edit a workflow by chatting in plain English?",
    scores: { SKYCOT: "yes", Zapier: "no", n8n: "no", "Crew.ai": "no", Lindy: "yes" },
  },
  {
    group: "Plain-English editing",
    question: "Visual step timeline with approval gates inline?",
    scores: { SKYCOT: "yes", Zapier: "yes", n8n: "yes", "Crew.ai": "no", Lindy: "no" },
  },
  {
    group: "Enterprise-ready",
    question: "Multi-tenant by construction (not bolted on)?",
    scores: { SKYCOT: "yes", Zapier: "yes", n8n: "yes", "Crew.ai": "no", Lindy: "partial" },
  },
  {
    group: "Enterprise-ready",
    question: "Capability-based RBAC across every action?",
    scores: { SKYCOT: "yes", Zapier: "partial", n8n: "partial", "Crew.ai": "no", Lindy: "no" },
  },
  {
    group: "Enterprise-ready",
    question: "Encryption-at-rest with operator-controlled keys?",
    scores: { SKYCOT: "yes", Zapier: "yes", n8n: "partial", "Crew.ai": "no", Lindy: "no" },
  },
  {
    group: "Deploy how you want",
    question: "Single binary you can self-host?",
    scores: { SKYCOT: "yes", Zapier: "no", n8n: "yes", "Crew.ai": "yes", Lindy: "no" },
  },
  {
    group: "Deploy how you want",
    question: "Air-gapped deploys for regulated environments?",
    scores: { SKYCOT: "yes", Zapier: "no", n8n: "partial", "Crew.ai": "partial", Lindy: "no" },
  },
];

const POSITIONING: Record<(typeof COMPETITORS)[number], string> = {
  SKYCOT: "AI teammates that ask before they act. Enterprise audit-first.",
  Zapier: "No-code workflow automation. Strong on integrations, light on AI.",
  n8n: "Open-source workflow builder. Strong self-host, light on the AI layer.",
  "Crew.ai": "Multi-agent framework for developers. Code-first, no operator surface.",
  Lindy: "AI assistant builder. Strong UX, light on multi-channel + audit.",
};

const TAGLINE: Record<(typeof COMPETITORS)[number], string> = {
  SKYCOT: "Hire a teammate",
  Zapier: "Connect apps",
  n8n: "Build workflows",
  "Crew.ai": "Build agents",
  Lindy: "Build assistants",
};

function groupDimensions(dims: Dimension[]) {
  const groups: { name: string; dims: Dimension[] }[] = [];
  for (const d of dims) {
    const last = groups[groups.length - 1];
    if (last && last.name === d.group) {
      last.dims.push(d);
    } else {
      groups.push({ name: d.group, dims: [d] });
    }
  }
  return groups;
}

export default function ComparePage() {
  const groups = groupDimensions(DIMENSIONS);
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .compare-stage {
          padding: 48px 32px 96px;
          max-width: 1180px;
          margin: 0 auto;
        }
        .compare-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 18px;
          padding: 5px 14px;
          border-radius: 999px;
          border: 1px solid var(--border, #ece9f5);
          background: var(--background, #fafaff);
        }
        .compare-hero {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 48px;
          line-height: 1.05;
          font-weight: 400;
          letter-spacing: 0;
          margin: 0 0 14px;
          color: var(--foreground, #1e1b4b);
          max-width: 740px;
        }
        .compare-hero strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .compare-sub {
          font-size: 16px;
          font-weight: 300;
          color: var(--muted-foreground, #3f3f46);
          margin: 0 0 36px;
          max-width: 620px;
        }
        .compare-positioning {
          display: grid;
          grid-template-columns: repeat(5, 1fr);
          gap: 12px;
          margin-bottom: 48px;
        }
        @media (max-width: 980px) {
          .compare-positioning {
            grid-template-columns: repeat(2, 1fr);
          }
        }
        .compare-position-card {
          border: 1px solid var(--border, #ece9f5);
          border-radius: 12px;
          padding: 16px;
          background: white;
        }
        .compare-position-card.us {
          background: linear-gradient(135deg, rgba(139, 92, 246, 0.06), rgba(236, 72, 153, 0.06));
          border-color: rgba(139, 92, 246, 0.3);
        }
        .compare-position-name {
          font-weight: 700;
          font-size: 16px;
          color: var(--foreground, #1e1b4b);
          margin: 0 0 4px;
        }
        .compare-position-name.us {
          font-family: "Audiowide", "Inter", sans-serif;
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .compare-position-tagline {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          font-weight: 600;
          color: var(--muted-foreground, #71717a);
          margin: 0 0 8px;
        }
        .compare-position-blurb {
          font-size: 12.5px;
          color: var(--muted-foreground, #3f3f46);
          line-height: 1.5;
          margin: 0;
        }
        .compare-table-wrap {
          background: white;
          border: 1px solid var(--border, #ece9f5);
          border-radius: 14px;
          /* Mobile-scroll wrap (a11y + responsive — wave 2): keep the
             5-column matrix scrollable horizontally instead of
             collapsing the layout below ~720px. The wrap clips the
             corners but the inner table can scroll. */
          overflow-x: auto;
          -webkit-overflow-scrolling: touch;
        }
        .compare-table {
          width: 100%;
          min-width: 720px;
          border-collapse: collapse;
          font-family: "Inter", sans-serif;
        }
        .compare-table th,
        .compare-table td {
          padding: 14px 18px;
          text-align: center;
          font-size: 13.5px;
          vertical-align: middle;
        }
        .compare-table th:first-child,
        .compare-table td:first-child {
          text-align: left;
          width: 44%;
        }
        .compare-table thead th {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 700;
          color: var(--muted-foreground, #71717a);
          background: var(--background, #fafaff);
          border-bottom: 1px solid var(--border, #ece9f5);
        }
        .compare-table thead th.us-col {
          color: #8b5cf6;
        }
        .compare-table tbody td.us-col {
          background: rgba(139, 92, 246, 0.04);
        }
        .compare-table tbody tr {
          border-bottom: 1px solid var(--border, #ece9f5);
        }
        .compare-table tbody tr:last-child {
          border-bottom: none;
        }
        .compare-group-row td {
          background: var(--background, #fafaff);
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          color: var(--muted-foreground, #71717a);
          padding-top: 18px;
          padding-bottom: 10px;
          border-top: 1px solid var(--border, #ece9f5);
        }
        .compare-group-row td:first-child {
          color: var(--foreground, #1e1b4b);
        }
        .compare-question-cell {
          font-size: 14px;
          font-weight: 500;
          color: var(--foreground, #1e1b4b);
        }
        .compare-caption {
          font-size: 12px;
          color: var(--muted-foreground, #71717a);
          margin-top: 4px;
          font-weight: 400;
          line-height: 1.45;
        }
        .compare-score {
          display: inline-grid;
          place-items: center;
          width: 28px;
          height: 28px;
          border-radius: 7px;
        }
        .compare-score.yes {
          background: rgba(20, 184, 166, 0.15);
          color: #0f766e;
        }
        .compare-score.no {
          background: rgba(244, 63, 94, 0.12);
          color: #be123c;
        }
        .compare-score.partial {
          background: rgba(245, 158, 11, 0.15);
          color: #b45309;
        }
        .compare-footer-row td {
          padding-top: 28px;
          padding-bottom: 28px;
          background: linear-gradient(135deg, rgba(139, 92, 246, 0.06), rgba(236, 72, 153, 0.06));
          text-align: center;
          font-size: 13.5px;
          color: var(--muted-foreground, #3f3f46);
        }
        .compare-cta {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 12px 22px;
          border-radius: 12px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-family: "Inter", sans-serif;
          font-size: 14px;
          font-weight: 700;
          border: none;
          cursor: pointer;
          text-decoration: none;
          box-shadow: 0 6px 16px -4px rgba(139, 92, 246, 0.45);
          margin-top: 12px;
        }
      `}</style>
      <div className="compare-stage">
        <div className="compare-eyebrow">
          <span style={{ width: 6, height: 6, borderRadius: "50%", background: "#14b8a6" }} />
          COMPARE
        </div>

        <h1 className="compare-hero">
          A teammate isn&apos;t a workflow.
          <br />
          It&apos;s the <strong>thing the workflow serves</strong>.
        </h1>

        <p className="compare-sub">
          Zapier, n8n, Crew.ai, Lindy — each one is good at part of the job. SKYCOT picks up where they
          leave off: named teammates that ask before they act, earn trust over time, and ship a full
          audit trail of every decision.
        </p>

        <div className="compare-positioning">
          {COMPETITORS.map((c, idx) => (
            <div key={c} className={`compare-position-card${idx === 0 ? " us" : ""}`}>
              <h3 className={`compare-position-name${idx === 0 ? " us" : ""}`}>{c}</h3>
              <p className="compare-position-tagline">{TAGLINE[c]}</p>
              <p className="compare-position-blurb">{POSITIONING[c]}</p>
            </div>
          ))}
        </div>

        <div className="compare-table-wrap">
          <table className="compare-table">
            <thead>
              <tr>
                <th></th>
                {COMPETITORS.map((c, idx) => (
                  <th key={c} className={idx === 0 ? "us-col" : ""}>
                    {c}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {groups.map((g) => (
                <ScoreGroup key={g.name} group={g.name} dims={g.dims} />
              ))}
              <tr className="compare-footer-row">
                <td colSpan={COMPETITORS.length + 1}>
                  <strong>If 8+ of those rows landed for you, this is your team.</strong>
                  <br />
                  <a className="compare-cta" href="/auth/signup">
                    Hire your first teammate →
                  </a>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </>
  );
}

function ScoreGroup({ group, dims }: { group: string; dims: Dimension[] }) {
  return (
    <>
      <tr className="compare-group-row">
        <td>{group}</td>
        {COMPETITORS.map((c) => (
          <td key={c} />
        ))}
      </tr>
      {dims.map((d, i) => (
        <tr key={`${group}-${i}`}>
          <td className="compare-question-cell">
            {d.question}
            {d.caption ? <div className="compare-caption">{d.caption}</div> : null}
          </td>
          {COMPETITORS.map((c, idx) => (
            <td key={c} className={idx === 0 ? "us-col" : ""}>
              <ScoreCell score={d.scores[c]} />
            </td>
          ))}
        </tr>
      ))}
    </>
  );
}

function ScoreCell({ score }: { score: Score }) {
  if (score === "yes")
    return (
      <span className="compare-score yes" aria-label="Yes">
        <Check size={15} />
      </span>
    );
  if (score === "no")
    return (
      <span className="compare-score no" aria-label="No">
        <X size={15} />
      </span>
    );
  return (
    <span className="compare-score partial" aria-label="Partial">
      <Minus size={15} />
    </span>
  );
}
