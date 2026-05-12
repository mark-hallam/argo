"use client";

import Link from "next/link";
import { ChevronDown } from "lucide-react";
import { ExampleRoleTile } from "@/components/skycot/ExampleRoleTile";
import { EXAMPLE_ROLES } from "@/lib/hire/examples";

/**
 * First-hire empty state for the `/team` surface (E3-e).
 *
 * Replaces E3-a's minimal placeholder. Renders five
 * `<ExampleRoleTile>`s in a responsive grid (1 col mobile, 2 col
 * tablet, 3 col desktop) and a secondary "Or describe your own" link
 * that routes to `/hire` with no `?example=` param. A closed
 * `<details>` disclosure below the grid reveals the locked
 * five-bullet feature summary on demand.
 *
 * Copy is locked by the brief (heading, sub-line, disclosure summary,
 * five bullets); do not drift the strings without a copy review.
 */
const FEATURE_BULLETS: readonly string[] = [
  "Replies that sound like you — voice profile inferred from how you write.",
  "Approvals via Slack, iMessage, SMS, email — your pick.",
  "Learns from your corrections without you editing a config file.",
  "Audit trail for every action — exportable for compliance.",
  "Pause or adjust her anytime — she's your teammate, not your tool.",
];

export function TeamEmptyState() {
  return (
    <section
      data-testid="team-empty-state"
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] px-4 py-10 sm:px-8 sm:py-12 space-y-8"
    >
      <header className="space-y-2 text-center">
        <h2 className="text-2xl font-semibold tracking-tight text-[var(--foreground)]">
          Hire your first AI teammate.
        </h2>
        <p className="mx-auto max-w-xl text-sm text-[var(--muted-foreground)]">
          Pick a starting point — you can adjust everything in the next minute.
        </p>
      </header>

      <ul
        data-testid="team-empty-state-tiles"
        className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3"
      >
        {EXAMPLE_ROLES.map((role) => (
          <li key={role.id} className="h-full">
            <ExampleRoleTile role={role} />
          </li>
        ))}
      </ul>

      <div className="flex justify-center">
        <Link
          href="/hire"
          data-testid="team-empty-state-describe-own"
          className="text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] rounded-[var(--radius-sm)]"
        >
          Or describe your own →
        </Link>
      </div>

      <details
        data-testid="team-empty-state-disclosure"
        className="group mx-auto max-w-xl rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)]/40"
      >
        <summary
          data-testid="team-empty-state-disclosure-summary"
          className="flex cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 text-sm font-medium text-[var(--foreground)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] rounded-[var(--radius-md)]"
        >
          What can SKYCOT teammates do?
          <ChevronDown
            aria-hidden
            className="size-4 text-[var(--muted-foreground)] transition-transform duration-[var(--duration-fast)] ease-out group-open:rotate-180"
          />
        </summary>
        <ul
          data-testid="team-empty-state-feature-list"
          className="space-y-2 border-t border-[var(--border)] px-4 py-3 text-sm text-[var(--muted-foreground)]"
        >
          {FEATURE_BULLETS.map((bullet) => (
            <li key={bullet} className="flex gap-2">
              <span aria-hidden className="select-none text-[var(--primary)]">
                •
              </span>
              <span>{bullet}</span>
            </li>
          ))}
        </ul>
      </details>
    </section>
  );
}
