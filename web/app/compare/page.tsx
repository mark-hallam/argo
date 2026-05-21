import "./page.css";

import { Check, X, Minus } from "lucide-react";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";

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
    group: "Safe by default",
    question: "Can you re-verify every past action months later (cryptographic audit chain)?",
    scores: { SKYCOT: "yes", Zapier: "no", n8n: "no", "Crew.ai": "no", Lindy: "no" },
    caption:
      "`argo audit verify` walks the hash chain and reproves every row's self_hash. Zapier/n8n ship a log store; Crew.ai prints to stdout; Lindy offers a SaaS audit log without cryptographic verification.",
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
    group: "Plain-English editing",
    question: "Cross-app ⌘K palette — every worker, workflow, skill, page reachable in one keystroke?",
    scores: { SKYCOT: "yes", Zapier: "no", n8n: "no", "Crew.ai": "no", Lindy: "no" },
    caption:
      "Command palette indexes the whole product surface, with category-grouped results, recents, and quick actions. Zapier has app-search; the others have nothing equivalent.",
  },
  {
    group: "Enterprise-ready",
    question: "Multi-tenant by construction (not bolted on)?",
    scores: { SKYCOT: "yes", Zapier: "yes", n8n: "yes", "Crew.ai": "no", Lindy: "partial" },
  },
  {
    group: "Enterprise-ready",
    question: "Per-action role permissions across every step?",
    scores: { SKYCOT: "yes", Zapier: "partial", n8n: "partial", "Crew.ai": "no", Lindy: "no" },
  },
  {
    group: "Enterprise-ready",
    question: "Encryption-at-rest with operator-controlled keys?",
    scores: { SKYCOT: "yes", Zapier: "yes", n8n: "partial", "Crew.ai": "no", Lindy: "no" },
  },
  {
    group: "Enterprise-ready",
    question: "BYOK — your own KMS key (AWS / GCP / Azure) wraps the database?",
    scores: { SKYCOT: "yes", Zapier: "no", n8n: "partial", "Crew.ai": "no", Lindy: "no" },
    caption:
      "Envelope encryption via AWS KMS, GCP KMS, or Azure Key Vault. n8n is partial because you manage your own DB; Zapier/Crew.ai/Lindy are managed-only.",
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
      <MarketingNav />
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
                  <strong>If 10+ of those rows landed for you, this is your team.</strong>
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
      <MarketingFooter />
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
