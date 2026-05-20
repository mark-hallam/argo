"use client";

import { EmptyState } from "@/components/ui/EmptyState";

export type ScopedResource = "worker" | "workflow" | "audit entry";

interface CohortScopeDeniedEmptyStateProps {
  /** The kind of thing the viewer was trying to look at. */
  resource: ScopedResource;
  /** Optional cohort label, e.g. "Sales · Established". */
  cohortLabel?: string;
  /** Optional admin contact callback (renders a "Request access" CTA). */
  onRequestAccess?: () => void;
}

/**
 * Friendly empty state rendered when a viewer hits a resource
 * (worker / workflow / audit entry) outside their cohort scope.
 *
 * Used by per-resource pages once the gateway exposes a
 * `cohort-scope-denied` signal (currently a `403` with a
 * `denied_reason: "cohort_scope"` body field). Until that signal
 * lands, this component is exercised through its tests + Storybook
 * surface so the visual + copy ship with the rest of the brief.
 *
 * Soft 403 by design — never use this for a hard "not found"; that's
 * `EmptyState` with the standard not-found illustration.
 */
export function CohortScopeDeniedEmptyState({
  resource,
  cohortLabel,
  onRequestAccess,
}: CohortScopeDeniedEmptyStateProps) {
  const cohortClause = cohortLabel ? ` in the ${cohortLabel} cohort` : "";
  return (
    <EmptyState
      data-testid="cohort-scope-denied"
      headline={`This ${resource} isn't in a cohort you have access to`}
      body={
        <>
          Ask your admin if you need access to this {resource}
          {cohortClause}. Your current scope was set on{" "}
          <a href="/settings/members" className="underline">
            Team & roles
          </a>
          .
        </>
      }
      cta={
        onRequestAccess
          ? { label: "Request access", onClick: onRequestAccess }
          : undefined
      }
    />
  );
}
