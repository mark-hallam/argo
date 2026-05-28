"use client";

import { ArrowLeft, Plus } from "lucide-react";
import Link from "next/link";
import { useVerticals } from "@/lib/api/verticals";

/**
 * 7-entry fallback used when the gateway `/verticals` endpoint is
 * unreachable in dev/mock-offline. Matches the cohorts dept taxonomy
 * in `web/components/skycot/cohorts/dept-registry.ts` so the picker
 * always covers every department tab the operator sees on /cohorts —
 * notably Marketing and Compliance, which the original 5-element
 * scaffold omitted.
 */
const FALLBACK_VERTICALS: ReadonlyArray<readonly [string, string, string]> = [
  ["sales", "Sales", "Lead follow-up, research, qualification, and CRM handoffs."],
  ["support", "Support", "Ticket triage, customer follow-up, and escalation workflows."],
  ["gtm-engineer", "GTM", "Campaign operations, launch checklists, and market scans."],
  ["people-ops", "People Ops", "Candidate screens, onboarding checklists, and policy reviews."],
  ["finance", "Finance", "Invoice review, approval routing, and month-end controls."],
  ["marketing", "Marketing", "Content drafts, social posts, and competitive scans with brand-voice checks."],
  ["compliance", "Compliance", "Policy reviews, audit prep, and regulatory deadline tracking."],
];

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
    <main className="mx-auto flex min-h-[calc(100vh-6rem)] w-full max-w-5xl flex-col gap-8 px-4 py-8 sm:px-6 lg:px-8">
      <Link
        href="/workflows"
        className="inline-flex w-fit items-center gap-2 text-sm font-medium text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
      >
        <ArrowLeft className="size-3.5" />
        All workflows
      </Link>

      <header className="max-w-3xl space-y-3">
        <p className="text-xs font-semibold uppercase text-[var(--muted-foreground)]">
          Workflow builder
        </p>
        <h1 className="text-3xl font-semibold tracking-normal text-[var(--foreground)] sm:text-4xl">
          Start from a blank canvas
        </h1>
        <p className="text-base leading-7 text-[var(--muted-foreground)]">
          Pick the vertical that owns the workflow. The builder opens with an
          empty canvas and a draft name ready to edit.
        </p>
      </header>

      {isLoading ? (
        <section
          aria-label="Loading workflow verticals"
          data-testid="workflow-build-vertical-picker-skeleton"
          className="grid gap-3 sm:grid-cols-2"
        >
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="min-h-32 animate-pulse rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-5"
            />
          ))}
        </section>
      ) : (
        <section
          aria-label="Choose a workflow vertical"
          data-testid="workflow-build-vertical-picker"
          data-source={useGateway ? "gateway" : "fallback"}
          className="grid gap-3 sm:grid-cols-2"
        >
          {entries.map(([id, label, description]) => (
            <Link
              key={id}
              href={`/workflows/${encodeURIComponent(id)}/untitled/build`}
              data-testid={`workflow-build-vertical-${id}`}
              className="group flex min-h-32 items-start justify-between gap-4 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-5 text-left transition hover:-translate-y-0.5 hover:border-[var(--primary)] hover:shadow-sm"
            >
              <span className="space-y-2">
                <span className="block text-lg font-semibold text-[var(--foreground)]">
                  {label}
                </span>
                <span className="block text-sm leading-6 text-[var(--muted-foreground)]">
                  {description}
                </span>
              </span>
              <span className="mt-1 inline-flex size-9 shrink-0 items-center justify-center rounded-full border border-[var(--border)] text-[var(--muted-foreground)] transition group-hover:border-[var(--primary)] group-hover:text-[var(--primary)]">
                <Plus className="size-4" />
              </span>
            </Link>
          ))}
        </section>
      )}
    </main>
  );
}
