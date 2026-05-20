/**
 * Cohort label + lookup helpers shared by the cohort-scope cell on
 * the members list and the picker inside the Edit / Invite modals.
 *
 * Cohort ids look like `support::trusted`; the human-friendly label
 * is `Support · Trusted`. Falls back to the raw id if the band code
 * isn't in BAND_COPY (matches CohortCard's behavior).
 */

import type { CohortSummary } from "@/lib/api/cohorts";

const BAND_COPY: Record<string, string> = {
  unknown: "Pre-trust",
  developing: "Developing",
  established: "Established",
  trusted: "Trusted",
  principal: "Principal",
};

function capitalize(word: string): string {
  return word.length === 0 ? word : word[0].toUpperCase() + word.slice(1);
}

export function cohortLabel(cohort: CohortSummary): string {
  const band = BAND_COPY[cohort.trust_grade_band] ?? cohort.trust_grade_band;
  return `${capitalize(cohort.vertical)} · ${band}`;
}

export function cohortLabelFromId(
  cohortId: string,
  cohorts: CohortSummary[],
): string {
  const match = cohorts.find((c) => c.cohort_id === cohortId);
  return match ? cohortLabel(match) : cohortId;
}
