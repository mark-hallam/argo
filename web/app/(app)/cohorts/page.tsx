"use client";

import { useCallback, useState } from "react";
import { CohortCard } from "@/components/skycot/CohortCard";
import { CohortDetailDrawer } from "@/components/skycot/CohortDetailDrawer";
import { EmptyState } from "@/components/ui/EmptyState";
import { CohortsIllustration } from "@/components/empty-state/illustrations/CohortsIllustration";
import { useCohorts } from "@/lib/hooks/use-cohorts";

export default function CohortsPage() {
  const cohortsQuery = useCohorts();
  const [openCohortId, setOpenCohortId] = useState<string | null>(null);

  const onSelect = useCallback((id: string) => setOpenCohortId(id), []);
  const onClose = useCallback(() => setOpenCohortId(null), []);

  const cohorts = cohortsQuery.data?.cohorts ?? [];
  const total = cohorts.length;

  // Cohort aggregate KPIs for the brand-loud hero banner.
  // CohortSummary uses snake_case fields (member_count / skill_count).
  const totalMembers = cohorts.reduce((sum, c) => sum + (c.member_count ?? 0), 0);
  const totalSkills = cohorts.reduce((sum, c) => sum + (c.skill_count ?? 0), 0);

  return (
    <>
      <CohortsHero total={total} members={totalMembers} skills={totalSkills} />

      <div className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
        <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-5">
          <div className="space-y-1">
            <h1 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
              Cohorts
            </h1>
            <p
              className="text-sm text-[var(--muted-foreground)]"
              data-testid="cohorts-subtitle"
            >
              {cohortsQuery.isPending ? (
                "Loading cohorts…"
              ) : (
                <>
                  {total} {total === 1 ? "cohort" : "cohorts"} across the fleet
                </>
              )}
            </p>
          </div>
        </div>
      </div>

      <div
        className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8"
        data-testid="cohorts-page"
      >
        {cohortsQuery.isPending ? (
          <LoadingGrid />
        ) : cohorts.length === 0 ? (
          <EmptyState
            data-testid="cohorts-empty"
            illustration={CohortsIllustration}
            headline="No cohorts"
            body="Group similar tasks into a cohort to share context across workers."
            cta={{ label: "Create cohort", href: "/cohorts/new" }}
          />
        ) : (
          <div
            className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4"
            data-testid="cohort-grid"
            role="list"
          >
            {cohorts.map((c) => (
              <div role="listitem" key={c.cohort_id}>
                <CohortCard
                  cohort={c}
                  onSelect={onSelect}
                  selected={c.cohort_id === openCohortId}
                />
              </div>
            ))}
          </div>
        )}
      </div>

      <CohortDetailDrawer cohortId={openCohortId} onClose={onClose} />
    </>
  );
}

function LoadingGrid() {
  return (
    <div
      data-testid="cohorts-loading"
      className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4"
    >
      {Array.from({ length: 6 }).map((_, i) => (
        <div
          key={i}
          className="h-40 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)]/40 animate-pulse"
        />
      ))}
    </div>
  );
}

/**
 * Cohorts hero — D3 magazine-tiles brand-loud banner.
 *
 * Same gradient-hero language as TeamHero + ApprovalsHero. The locked
 * D3 magazine-tiles design lives in the cohort grid below; the hero
 * gives the page its "Bloomberg-feature editorial" register.
 */
function CohortsHero({ total, members, skills }: { total: number; members: number; skills: number }) {
  let title: string;
  let sub: string;
  if (total === 0) {
    title = "Build your first cohort.";
    sub = "Cohorts group teammates by department, skill family, or trust grade.";
  } else if (total === 1) {
    title = "1 cohort across your fleet.";
    sub = "Trust grades earn workers into more responsibility over time.";
  } else {
    title = `${total} cohorts across your fleet.`;
    sub = "Each cohort earns its trust grade by shipping consistently. Promotions happen automatically.";
  }
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .cohorts-hero {
          position: relative;
          margin: 24px 16px 0;
          padding: 28px 32px;
          border-radius: 18px;
          background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 50%, #ff6b6b 100%);
          color: white;
          overflow: hidden;
          box-shadow: 0 16px 40px -16px rgba(139, 92, 246, 0.45);
        }
        .cohorts-hero::before {
          content: "";
          position: absolute;
          inset: 0;
          background: linear-gradient(
            115deg,
            transparent 30%,
            rgba(255, 255, 255, 0.18) 50%,
            transparent 70%
          );
          transform: translateX(-100%);
          animation: cohorts-hero-shine 6s ease-in-out infinite;
          pointer-events: none;
        }
        @keyframes cohorts-hero-shine {
          0%, 80%, 100% {
            transform: translateX(-100%);
          }
          90% {
            transform: translateX(100%);
          }
        }
        .cohorts-hero-grid {
          position: relative;
          display: flex;
          align-items: flex-end;
          justify-content: space-between;
          gap: 32px;
          flex-wrap: wrap;
        }
        .cohorts-hero-left {
          flex: 1;
          min-width: 260px;
        }
        .cohorts-hero-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          color: rgba(255, 255, 255, 0.85);
          padding: 4px 12px;
          border-radius: 999px;
          background: rgba(255, 255, 255, 0.16);
          backdrop-filter: blur(8px);
          margin-bottom: 14px;
        }
        .cohorts-hero-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: #ddd6fe;
        }
        .cohorts-hero-title {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 36px;
          line-height: 1.05;
          font-weight: 400;
          letter-spacing: 0.005em;
          margin: 0;
          color: white;
        }
        .cohorts-hero-sub {
          margin-top: 8px;
          font-size: 14px;
          font-weight: 400;
          color: rgba(255, 255, 255, 0.85);
          max-width: 480px;
        }
        .cohorts-hero-kpis {
          display: flex;
          gap: 28px;
        }
        .cohorts-kpi {
          text-align: right;
        }
        .cohorts-kpi-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 600;
          color: rgba(255, 255, 255, 0.75);
          margin-bottom: 4px;
        }
        .cohorts-kpi-value {
          font-family: "Inter", sans-serif;
          font-size: 36px;
          font-weight: 900;
          letter-spacing: -0.02em;
          color: white;
          line-height: 1;
        }
        @media (max-width: 720px) {
          .cohorts-hero {
            margin: 16px 12px 0;
            padding: 22px 22px;
          }
          .cohorts-hero-title {
            font-size: 28px;
          }
          .cohorts-hero-kpis {
            gap: 18px;
            width: 100%;
            justify-content: flex-start;
            margin-top: 16px;
          }
          .cohorts-kpi {
            text-align: left;
          }
          .cohorts-kpi-value {
            font-size: 28px;
          }
        }
      `}</style>
      <div className="cohorts-hero">
        <div className="cohorts-hero-grid">
          <div className="cohorts-hero-left">
            <div className="cohorts-hero-eyebrow">
              <span className="cohorts-hero-eyebrow-dot" />
              COHORTS
            </div>
            <h1 className="cohorts-hero-title">{title}</h1>
            <p className="cohorts-hero-sub">{sub}</p>
          </div>
          {total > 0 ? (
            <div className="cohorts-hero-kpis">
              <div className="cohorts-kpi">
                <div className="cohorts-kpi-label">Cohorts</div>
                <div className="cohorts-kpi-value">{total}</div>
              </div>
              <div className="cohorts-kpi">
                <div className="cohorts-kpi-label">Members</div>
                <div className="cohorts-kpi-value">{members}</div>
              </div>
              <div className="cohorts-kpi">
                <div className="cohorts-kpi-label">Skills</div>
                <div className="cohorts-kpi-value">{skills}</div>
              </div>
            </div>
          ) : null}
        </div>
      </div>
    </>
  );
}

