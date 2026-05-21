"use client";

/**
 * Cold-load shimmer fallback for `/cohorts`. Mirrors the locked D3
 * magazine layout (1 lead + 2 stacked + 4 compact) so the asymmetric
 * grid is reserved before real data hydrates — no layout shift when
 * the cohorts query resolves. Shimmer is a single linear-gradient
 * sweep on each placeholder; reduced-motion swaps it for a static
 * muted fill via the styles file.
 */
export function CohortMagazineSkeleton() {
  return (
    <div
      className="skycot-cohort-mag"
      data-testid="cohort-mag-skeleton"
      role="status"
      aria-live="polite"
      aria-label="Loading cohorts"
    >
      <div className="skycot-cohort-mag-row1">
        <div
          className="skycot-cohort-skel skycot-cohort-skel-lead"
          data-testid="cohort-skel-lead"
        >
          <div className="skycot-cohort-skel-av skycot-cohort-skel-av-lead" />
          <div className="skycot-cohort-skel-line skycot-cohort-skel-kicker" />
          <div className="skycot-cohort-skel-line skycot-cohort-skel-title-1" />
          <div className="skycot-cohort-skel-line skycot-cohort-skel-title-2" />
          <div className="skycot-cohort-skel-line skycot-cohort-skel-desc" />
          <div className="skycot-cohort-skel-line skycot-cohort-skel-desc skycot-cohort-skel-desc-2" />
          <div className="skycot-cohort-skel-stats" />
        </div>
        <div className="skycot-cohort-mag-rightcol">
          {[0, 1].map((i) => (
            <div
              key={i}
              className="skycot-cohort-skel skycot-cohort-skel-stacked"
              data-testid="cohort-skel-stacked"
            >
              <div className="skycot-cohort-skel-av skycot-cohort-skel-av-stacked" />
              <div className="skycot-cohort-skel-line skycot-cohort-skel-kicker" />
              <div className="skycot-cohort-skel-line skycot-cohort-skel-title-2" />
              <div className="skycot-cohort-skel-line skycot-cohort-skel-desc" />
              <div className="skycot-cohort-skel-stats" />
            </div>
          ))}
        </div>
      </div>
      <div className="skycot-cohort-mag-row2">
        {[0, 1, 2, 3].map((i) => (
          <div
            key={i}
            className="skycot-cohort-skel skycot-cohort-skel-compact"
            data-testid="cohort-skel-compact"
          >
            <div className="skycot-cohort-skel-av skycot-cohort-skel-av-compact" />
            <div className="skycot-cohort-skel-line skycot-cohort-skel-kicker" />
            <div className="skycot-cohort-skel-line skycot-cohort-skel-title-2" />
            <div className="skycot-cohort-skel-line skycot-cohort-skel-desc" />
            <div className="skycot-cohort-skel-stats" />
          </div>
        ))}
      </div>
      <span className="skycot-cohort-skel-sr">Loading cohorts…</span>
    </div>
  );
}
