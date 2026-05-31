"use client";

import "./page.css";

import { ArrowLeft, ArrowRight, Sparkles } from "lucide-react";
import Link from "next/link";
import { useVerticals } from "@/lib/api/verticals";

/**
 * 8-entry fallback used when the gateway `/verticals` endpoint is
 * unreachable in dev/mock-offline. Matches the cohorts dept taxonomy
 * in `web/components/skycot/cohorts/dept-registry.ts` so the picker
 * always covers every department tab the operator sees on /cohorts —
 * notably Marketing and Compliance, which the original 5-element
 * scaffold omitted. `marketplace` is the home vertical of the
 * canonical Scene-5 demo workflow `customer_research_followup`
 * (`crates/argo-demo-seed/src/workflow.rs:26`); without a tile here
 * the operator cannot author a new marketplace workflow from
 * /workflows/build when the gateway returns an empty catalog.
 */
const FALLBACK_VERTICALS: ReadonlyArray<readonly [string, string, string]> = [
  ["sales", "Sales", "Lead follow-up, research, qualification, and CRM handoffs."],
  ["support", "Support", "Ticket triage, customer follow-up, and escalation workflows."],
  ["marketplace", "Marketplace", "Listing posts, price sanity checks, buyer follow-up, and sold-state cleanup for marketplace sellers."],
  ["gtm-engineer", "GTM", "Campaign operations, launch checklists, and market scans."],
  ["people-ops", "People Ops", "Candidate screens, onboarding checklists, and policy reviews."],
  ["finance", "Finance", "Invoice review, approval routing, and month-end controls."],
  ["marketing", "Marketing", "Content drafts, social posts, and competitive scans with brand-voice checks."],
  ["compliance", "Compliance", "Policy reviews, audit prep, and regulatory deadline tracking."],
];

/**
 * D3-recipe accent palette, applied per-tile by index so the picker
 * keeps the brand gradient look regardless of how many verticals the
 * gateway returns (the static 5-entry list this replaced hardcoded an
 * accent per id; the data-driven list cycles the same palette).
 */
const ACCENT_PALETTE = [
  "var(--brand-primary)",
  "var(--brand-teal)",
  "var(--brand-pink)",
  "var(--warning)",
  "var(--brand-coral)",
] as const;

function accentFor(index: number): string {
  return ACCENT_PALETTE[index % ACCENT_PALETTE.length];
}

function summariseDescription(text: string): string {
  const trimmed = text.trim();
  if (trimmed.length <= 160) return trimmed;
  const cut = trimmed.slice(0, 160);
  const lastSpace = cut.lastIndexOf(" ");
  return `${cut.slice(0, lastSpace > 80 ? lastSpace : 160)}…`;
}

export default function WorkflowBuildEntryPage() {
  const { verticals, source, isLoading } = useVerticals();

  const useGateway = source === "gateway" && verticals.length > 0;
  const entries: ReadonlyArray<readonly [string, string, string]> = useGateway
    ? verticals.map(
        (v) =>
          [v.id, v.name, summariseDescription(v.headline ?? v.description)] as const,
      )
    : FALLBACK_VERTICALS;

  return (
    <div className="build-stage">
      <Link href="/workflows" className="build-back">
        <ArrowLeft size={11} /> ALL WORKFLOWS
      </Link>

      <div className="build-eyebrow">
        <Sparkles size={11} /> BUILD A WORKFLOW · CHOOSE A VERTICAL
      </div>

      <h1 className="build-hero">
        Start with a
        <br />
        <strong>blank canvas</strong>.
      </h1>

      <p className="build-sub">
        Pick the vertical that owns this workflow. The builder opens with an empty
        canvas and a draft name ready to edit — or open the template library if
        you&apos;d rather clone a proven playbook.
      </p>

      <div className="build-cta-row">
        <Link
          href="/workflows/templates"
          className="build-cta-templates"
          data-testid="build-templates-cta"
        >
          <Sparkles size={13} /> Open template library
        </Link>
        <span className="build-cta-or">or pick a vertical below</span>
      </div>

      <div className="build-section-label">
        <span>VERTICALS</span>
        <span>{entries.length} TO CHOOSE FROM</span>
      </div>

      {isLoading ? (
        <section
          aria-label="Loading workflow verticals"
          data-testid="workflow-build-vertical-picker-skeleton"
          className="build-grid"
        >
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="build-card build-card--skeleton" />
          ))}
        </section>
      ) : (
        <section
          aria-label="Choose a workflow vertical"
          data-testid="workflow-build-vertical-picker"
          data-source={useGateway ? "gateway" : "fallback"}
          className="build-grid"
        >
          {entries.map(([id, label, description], idx) => {
            const accent = accentFor(idx);
            return (
              <Link
                key={id}
                href={`/workflows/${encodeURIComponent(id)}/untitled/build`}
                data-testid={`workflow-build-vertical-${id}`}
                className={`build-card${idx === 0 ? " build-card--lead" : ""}`}
              >
                <div className="build-card-top">
                  <span className="build-card-vertical" style={{ color: accent }}>
                    {label.toUpperCase()}
                  </span>
                  <span className="build-card-stage">STEP 1 OF 3</span>
                </div>
                <h2 className="build-card-name">{label}</h2>
                <p className="build-card-desc">{description}</p>
                <div
                  className="build-card-accent"
                  style={{
                    background: `linear-gradient(90deg, ${accent} 0%, var(--brand-pink) 60%, transparent 100%)`,
                  }}
                />
                <ArrowRight size={18} className="build-card-arrow" />
              </Link>
            );
          })}
        </section>
      )}
    </div>
  );
}
