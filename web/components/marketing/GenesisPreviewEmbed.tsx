"use client";

import { useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { ArrowRight, Sparkles, Target, ListChecks, KeyRound, Clock } from "lucide-react";

/**
 * "Describe the job" Genesis hero — the interactive landing-page taste of the
 * single most differentiating live thing SKYCOT does: a real LLM reads a
 * plain-English job and authors a from-scratch, bespoke teammate (persona,
 * responsibilities, typed KPIs, a work cadence, and a least-privilege access
 * list scoped to only what the role needs).
 *
 * SAFE v1 (NO-THEATER, DEC-007): this surface does NOT call a live LLM per
 * anonymous visitor — a public, unauthenticated generate-on-demand endpoint is
 * a real cost-DoS vector (one paid model call per request). Instead it shows a
 * small set of REAL-SHAPED example specs (clearly labeled as examples) and
 * routes "describe your own" into the self-serve signup, where the teammate is
 * generated live from the visitor's exact words. The live in-hero generation is
 * a follow-up gated behind a per-IP + global hourly cost cap.
 *
 * The example specs mirror the structure argo-genesis actually emits
 * (broad_role.rs): a derived persona name, a role title, 3–4 responsibilities,
 * typed KPIs, a least-privilege access whitelist, and a cron cadence.
 */

interface TeammateSpec {
  /** The plain-English job that produced this teammate (the visitor's input). */
  job: string;
  /** Persona name derived from the role text. */
  name: string;
  /** Role title. */
  role: string;
  /** 3–4 standing responsibilities. */
  responsibilities: string[];
  /** Typed KPIs the teammate is measured on. */
  kpis: string[];
  /** Least-privilege access — only what the role needs, nothing more. */
  access: string[];
  /** Work cadence. */
  cadence: string;
}

const EXAMPLES: TeammateSpec[] = [
  {
    job: "Screen inbound leads, qualify them, and book interviews",
    name: "Priya",
    role: "Inbound Lead Qualifier",
    responsibilities: [
      "Read every new inbound lead and score fit against your ICP",
      "Ask qualifying questions and capture answers on the record",
      "Book qualified prospects straight onto the right calendar",
      "Flag anything ambiguous for a human instead of guessing",
    ],
    kpis: ["Leads screened / day", "Qualified-to-booked rate", "Median time-to-first-reply"],
    access: ["Inbound inbox (read)", "CRM (read + write)", "Calendar (book only)"],
    cadence: "On every new lead, plus a 9am daily digest",
  },
  {
    job: "Triage support tickets and draft first replies",
    name: "Jamal",
    role: "Support Triage Specialist",
    responsibilities: [
      "Categorize and prioritize every incoming ticket",
      "Draft a first reply grounded in your help docs",
      "Route billing / security tickets to the right human",
      "Never send a refund or account change without approval",
    ],
    kpis: ["Tickets triaged / day", "Median first-response time", "Draft-accepted rate"],
    access: ["Helpdesk (read + draft)", "Knowledge base (read)", "Refunds: always asks"],
    cadence: "On every new ticket, real-time",
  },
  {
    job: "Draft our weekly investor update from the metrics",
    name: "Nori",
    role: "Investor Comms Associate",
    responsibilities: [
      "Pull the week's metrics from your dashboards",
      "Draft the update in your house voice and format",
      "Highlight what changed and why, with the numbers",
      "Hold the draft for your review — never sends externally on its own",
    ],
    kpis: ["Update drafted by Mon 9am", "Edits-per-draft (lower is better)", "Metrics cited vs available"],
    access: ["Metrics dashboards (read)", "Doc drafts (write)", "External send: always asks"],
    cadence: "Every Monday 9am, plus on request",
  },
];

const SUGGESTIONS = EXAMPLES.map((e) => e.job);

export function GenesisPreviewEmbed() {
  const router = useRouter();
  const [selected, setSelected] = useState(0);
  const [input, setInput] = useState("");
  const spec = EXAMPLES[selected];

  function startFree(job: string) {
    const q = job.trim();
    router.push(q ? `/auth/signup?role=${encodeURIComponent(q.slice(0, 200))}` : "/auth/signup");
  }

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    startFree(input);
  }

  const card: React.CSSProperties = {
    border: "1px solid var(--line, #ece9f5)",
    borderRadius: 18,
    background: "white",
    boxShadow: "0 10px 40px -24px rgba(80,40,140,0.45)",
    overflow: "hidden",
  };

  return (
    <div data-testid="genesis-embed" style={card}>
      {/* Header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          padding: "14px 18px",
          borderBottom: "1px solid var(--line, #ece9f5)",
        }}
      >
        <span
          aria-hidden="true"
          style={{
            display: "inline-flex",
            width: 30,
            height: 30,
            borderRadius: 9,
            alignItems: "center",
            justifyContent: "center",
            color: "white",
            background: "linear-gradient(135deg, var(--brand-primary, #8B5CF6), var(--gradient-end, #EC4899))",
          }}
        >
          <Sparkles size={17} />
        </span>
        <div style={{ flex: 1 }}>
          <p style={{ margin: 0, fontWeight: 700, fontSize: 14.5, color: "var(--ink, #1e1b4b)" }}>
            Describe the job →
          </p>
          <p style={{ margin: 0, fontSize: 12.5, color: "var(--ink3, #7a7396)" }}>
            SKYCOT designs the teammate
          </p>
        </div>
      </div>

      {/* Input */}
      <form onSubmit={onSubmit} style={{ display: "flex", gap: 8, padding: "16px 18px 8px" }}>
        <input
          data-testid="genesis-input"
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="e.g. chase overdue invoices and reconcile payments…"
          maxLength={200}
          aria-label="Describe the job you'd hire for"
          style={{
            flex: 1,
            padding: "11px 13px",
            border: "1px solid var(--line, #ece9f5)",
            borderRadius: 11,
            fontSize: 14,
            fontFamily: "Inter, sans-serif",
            color: "var(--ink, #1e1b4b)",
            outline: "none",
          }}
        />
        <button
          type="submit"
          data-testid="genesis-generate"
          disabled={input.trim().length === 0}
          aria-label="Generate this teammate"
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            padding: "11px 16px",
            border: "none",
            borderRadius: 11,
            fontSize: 13.5,
            fontWeight: 700,
            color: "white",
            cursor: input.trim().length === 0 ? "not-allowed" : "pointer",
            opacity: input.trim().length === 0 ? 0.55 : 1,
            background: "linear-gradient(135deg, var(--brand-primary, #8B5CF6), var(--gradient-end, #EC4899))",
          }}
        >
          Generate <ArrowRight size={15} />
        </button>
      </form>

      {/* Example chips */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 7, padding: "4px 18px 14px" }}>
        <span style={{ fontSize: 12, color: "var(--ink3, #7a7396)", alignSelf: "center" }}>Or try:</span>
        {SUGGESTIONS.map((q, idx) => (
          <button
            key={q}
            type="button"
            data-testid={`genesis-chip-${idx}`}
            onClick={() => setSelected(idx)}
            style={{
              padding: "6px 11px",
              borderRadius: 999,
              border: "1px solid",
              borderColor: idx === selected ? "var(--brand-primary, #8B5CF6)" : "var(--line, #ece9f5)",
              background: idx === selected ? "rgba(139,92,246,0.08)" : "white",
              color: idx === selected ? "var(--brand-primary, #8B5CF6)" : "var(--ink2, #4b4470)",
              fontSize: 12.5,
              fontWeight: 600,
              cursor: "pointer",
              fontFamily: "Inter, sans-serif",
            }}
          >
            {q}
          </button>
        ))}
      </div>

      {/* The synthesized teammate spec */}
      <div
        data-testid="genesis-spec"
        style={{ borderTop: "1px solid var(--line, #ece9f5)", background: "var(--bg, #faf9ff)", padding: "16px 18px" }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
          <span
            aria-hidden="true"
            style={{
              display: "inline-flex",
              width: 38,
              height: 38,
              borderRadius: 11,
              alignItems: "center",
              justifyContent: "center",
              color: "white",
              fontWeight: 800,
              fontSize: 15,
              background: "linear-gradient(135deg, var(--brand-primary, #8B5CF6), var(--gradient-end, #EC4899))",
            }}
          >
            {spec.name.slice(0, 1)}
          </span>
          <div>
            <p style={{ margin: 0, fontWeight: 800, fontSize: 15.5, color: "var(--ink, #1e1b4b)" }}>{spec.name}</p>
            <p style={{ margin: 0, fontSize: 12.5, color: "var(--ink3, #7a7396)" }}>{spec.role}</p>
          </div>
        </div>

        <SpecBlock icon={<ListChecks size={13} />} label="Responsibilities">
          <ul style={{ margin: "4px 0 0", paddingLeft: 16 }}>
            {spec.responsibilities.map((r) => (
              <li key={r} style={{ fontSize: 13, color: "var(--ink2, #4b4470)", marginBottom: 3 }}>{r}</li>
            ))}
          </ul>
        </SpecBlock>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: 12, marginTop: 12 }}>
          <SpecBlock icon={<Target size={13} />} label="Measured on">
            <ChipList items={spec.kpis} />
          </SpecBlock>
          <SpecBlock icon={<KeyRound size={13} />} label="Least-privilege access">
            <ChipList items={spec.access} />
          </SpecBlock>
        </div>

        <SpecBlock icon={<Clock size={13} />} label="Cadence">
          <p style={{ margin: "3px 0 0", fontSize: 13, color: "var(--ink2, #4b4470)" }}>{spec.cadence}</p>
        </SpecBlock>
      </div>

      {/* Footer: honest label + the conversion CTA */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 12,
          flexWrap: "wrap",
          padding: "13px 18px",
          borderTop: "1px solid var(--line, #ece9f5)",
        }}
      >
        <p data-testid="genesis-sample-note" style={{ margin: 0, fontSize: 11.5, color: "var(--ink3, #7a7396)", maxWidth: 360 }}>
          Example of a real Genesis output. Your teammate is generated live from your exact
          words when you start — persona, KPIs, and least-privilege access included.
        </p>
        <button
          type="button"
          data-testid="genesis-start-free"
          onClick={() => startFree(input || spec.job)}
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            padding: "10px 16px",
            border: "none",
            borderRadius: 11,
            fontSize: 13.5,
            fontWeight: 700,
            color: "white",
            cursor: "pointer",
            whiteSpace: "nowrap",
            background: "linear-gradient(135deg, var(--brand-primary, #8B5CF6), var(--gradient-end, #EC4899))",
          }}
        >
          Start free — generate yours <ArrowRight size={15} />
        </button>
      </div>
    </div>
  );
}

function SpecBlock({ icon, label, children }: { icon: React.ReactNode; label: string; children: React.ReactNode }) {
  return (
    <div>
      <p
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 5,
          margin: 0,
          fontSize: 10.5,
          fontWeight: 800,
          letterSpacing: "0.06em",
          textTransform: "uppercase",
          color: "var(--brand-primary, #8B5CF6)",
        }}
      >
        {icon}
        {label}
      </p>
      {children}
    </div>
  );
}

function ChipList({ items }: { items: string[] }) {
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 5, marginTop: 5 }}>
      {items.map((it) => (
        <span
          key={it}
          style={{
            padding: "3px 9px",
            borderRadius: 7,
            background: "white",
            border: "1px solid var(--line, #ece9f5)",
            fontSize: 12,
            color: "var(--ink2, #4b4470)",
          }}
        >
          {it}
        </span>
      ))}
    </div>
  );
}
