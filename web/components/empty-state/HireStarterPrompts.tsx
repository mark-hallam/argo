"use client";

import { TeamEmptyState } from "@/components/skycot/TeamEmptyState";

/**
 * Shared "hire your first teammate" empty state. Renders the locked
 * 5 starter-prompt grid + "Or describe your own" link + feature-list
 * disclosure that `/team` ships, so any surface that lands the operator
 * on an empty workforce (today: `/workers`, `/team`) converges on the
 * same first-touch CTA pattern.
 *
 * Thin wrapper around `<TeamEmptyState>` — copy, layout, and `?example=`
 * deep-link hrefs are owned there. Collocated under `components/
 * empty-state/` so future consumers find the pattern by intent rather
 * than by `/team`'s path.
 */
export function HireStarterPrompts() {
  return <TeamEmptyState />;
}
