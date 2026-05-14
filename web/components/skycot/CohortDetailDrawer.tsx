"use client";

import { useEffect, useMemo, useRef } from "react";
import { X } from "lucide-react";
import { useCohortDetail } from "@/lib/hooks/use-cohorts";
import { cn } from "@/lib/utils";
import type { CohortDetail, CohortMember, SkillCoverage } from "@/lib/api/cohorts";

export interface CohortDetailDrawerProps {
  cohortId: string | null;
  onClose: () => void;
}

export function CohortDetailDrawer({ cohortId, onClose }: CohortDetailDrawerProps) {
  const query = useCohortDetail(cohortId);
  const closeRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (cohortId === null) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };
    document.addEventListener("keydown", handler);
    closeRef.current?.focus();
    return () => document.removeEventListener("keydown", handler);
  }, [cohortId, onClose]);

  if (cohortId === null) return null;
  const detail = query.data?.detail ?? null;

  return (
    <div
      data-testid="cohort-detail-overlay"
      className="fixed inset-0 z-40 flex justify-end bg-[rgb(0,0,0)]/40"
      onClick={onClose}
      role="presentation"
    >
      <aside
        data-testid="cohort-detail-drawer"
        role="dialog"
        aria-label={`Cohort ${cohortId} detail`}
        className="w-full max-w-3xl h-full overflow-y-auto bg-[var(--background)] border-l border-[var(--border)] shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <DrawerHeader cohortId={cohortId} onClose={onClose} closeRef={closeRef} />
        <div className="px-6 py-5 space-y-6">
          {query.isPending ? (
            <p
              data-testid="cohort-detail-loading"
              className="text-sm text-[var(--muted-foreground)]"
            >
              Loading…
            </p>
          ) : detail === null ? (
            <CohortMissingState />
          ) : (
            <>
              <SummarySection detail={detail} />
              <SkillCoverageMatrix detail={detail} />
            </>
          )}
        </div>
      </aside>
    </div>
  );
}

interface DrawerHeaderProps {
  cohortId: string;
  onClose: () => void;
  closeRef: React.RefObject<HTMLButtonElement | null>;
}

function DrawerHeader({ cohortId, onClose, closeRef }: DrawerHeaderProps) {
  return (
    <div className="sticky top-0 z-10 flex items-start justify-between gap-3 border-b border-[var(--border)] bg-[var(--background)]/95 px-6 py-4 backdrop-blur-md">
      <div className="space-y-1">
        <h2 className="text-base font-semibold tracking-tight text-[var(--foreground)]">
          {cohortId}
        </h2>
        <p className="text-xs text-[var(--muted-foreground)]">
          Cohort detail · member skill-coverage matrix
        </p>
      </div>
      <button
        ref={closeRef}
        type="button"
        aria-label="Close cohort detail"
        data-testid="cohort-detail-close"
        onClick={onClose}
        className="grid size-8 place-items-center rounded-[var(--radius-md)] text-[var(--muted-foreground)] hover:bg-[var(--muted)] hover:text-[var(--foreground)] transition-colors"
      >
        <X className="size-4" />
      </button>
    </div>
  );
}

function CohortMissingState() {
  return (
    <div
      data-testid="cohort-detail-empty"
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-10 px-6 text-center"
    >
      <h3 className="text-sm font-semibold text-[var(--foreground)]">
        Cohort not found
      </h3>
      <p className="mt-1 text-xs text-[var(--muted-foreground)]">
        It may have been emptied since the page loaded.
      </p>
    </div>
  );
}

function SummarySection({ detail }: { detail: CohortDetail }) {
  return (
    <section aria-labelledby="cohort-detail-summary-heading" className="space-y-2">
      <h3
        id="cohort-detail-summary-heading"
        className="text-xs font-medium uppercase tracking-wider text-[var(--muted-foreground)]"
      >
        Summary
      </h3>
      <dl className="grid grid-cols-3 gap-4 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-4">
        <Stat label="Members" value={String(detail.member_count)} />
        <Stat label="Skills" value={String(detail.skill_count)} />
        <Stat
          label="Trust band"
          value={detail.trust_grade_band}
          sub={`grade ${detail.trust_grade_min}–${detail.trust_grade_max}`}
        />
      </dl>
    </section>
  );
}

function Stat({
  label,
  value,
  sub,
}: {
  label: string;
  value: string;
  sub?: string;
}) {
  return (
    <div>
      <dt className="text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
        {label}
      </dt>
      <dd className="mt-1 text-xl font-semibold text-[var(--foreground)] capitalize">
        {value}
      </dd>
      {sub ? (
        <dd className="text-xs text-[var(--muted-foreground)]">{sub}</dd>
      ) : null}
    </div>
  );
}

/**
 * Skill ✕ member coverage matrix — the load-bearing operator-insight
 * surface. Cell states (see brief): ✓ when the member holds the
 * skill, ✗ when missing, version-mismatch reserved for future
 * per-worker version data. v1 marks cells "missing" via the
 * `cohort-cell-missing` testid which the matrix below renders in
 * the warning tone.
 */
function SkillCoverageMatrix({ detail }: { detail: CohortDetail }) {
  const byWorker = useMemo(() => buildWorkerSkillIndex(detail.members), [detail.members]);
  return (
    <section aria-labelledby="cohort-detail-matrix-heading" className="space-y-2">
      <h3
        id="cohort-detail-matrix-heading"
        className="text-xs font-medium uppercase tracking-wider text-[var(--muted-foreground)]"
      >
        Skill coverage
      </h3>
      <p className="text-xs text-[var(--muted-foreground)]">
        Highlights workers missing a cohort-common skill so operators can spot
        graph holes at a glance.
      </p>
      <div
        data-testid="cohort-coverage-matrix"
        className="overflow-x-auto rounded-[var(--radius-lg)] border border-[var(--border)]"
      >
        <table className="min-w-full text-sm">
          <thead className="bg-[var(--muted)]/40">
            <tr>
              <th
                scope="col"
                className="sticky left-0 z-10 bg-[var(--muted)]/40 px-3 py-2 text-left text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]"
              >
                Skill
              </th>
              {detail.members.map((m) => (
                <th
                  key={m.worker_id}
                  scope="col"
                  className="px-3 py-2 text-left text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]"
                >
                  {m.worker_id}
                </th>
              ))}
              <th
                scope="col"
                className="px-3 py-2 text-right text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]"
              >
                Coverage
              </th>
            </tr>
          </thead>
          <tbody>
            {detail.skills.map((skill) => (
              <SkillRow
                key={skill.skill_name}
                skill={skill}
                members={detail.members}
                byWorker={byWorker}
              />
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

interface SkillRowProps {
  skill: SkillCoverage;
  members: CohortMember[];
  byWorker: Map<string, Set<string>>;
}

function SkillRow({ skill, members, byWorker }: SkillRowProps) {
  return (
    <tr className="border-t border-[var(--border)]">
      <th
        scope="row"
        className="sticky left-0 z-10 bg-[var(--background)] px-3 py-2 text-left align-top"
      >
        <span className="font-medium text-[var(--foreground)]">{skill.skill_name}</span>
        {skill.version ? (
          <span className="ml-1 text-[11px] text-[var(--muted-foreground)]">
            v{skill.version}
          </span>
        ) : null}
      </th>
      {members.map((m) => {
        const has = byWorker.get(m.worker_id)?.has(skill.skill_name) ?? false;
        const tone = cellState(has);
        return (
          <td
            key={m.worker_id}
            data-testid={`cohort-cell-${tone}`}
            data-skill={skill.skill_name}
            data-worker={m.worker_id}
            className={cn(
              "px-3 py-2 text-center text-sm tabular-nums",
              tone === "present" && "text-emerald-700",
              tone === "missing" && "text-rose-700 bg-rose-50",
              tone === "mismatch" && "text-amber-700 bg-amber-50",
            )}
          >
            {tone === "present" ? "✓" : tone === "missing" ? "✗" : "≠"}
          </td>
        );
      })}
      <td className="px-3 py-2 text-right tabular-nums text-[var(--muted-foreground)]">
        {skill.member_coverage_pct.toFixed(1)}%
      </td>
    </tr>
  );
}

type CellState = "present" | "missing" | "mismatch";

function cellState(has: boolean): CellState {
  // v1 stores skills at the (worker, skill_name) level — no per-worker
  // version is tracked yet, so the "mismatch" state is reserved for
  // future schema growth. The rendering path stays forward-compatible.
  return has ? "present" : "missing";
}

function buildWorkerSkillIndex(members: CohortMember[]): Map<string, Set<string>> {
  const out = new Map<string, Set<string>>();
  for (const m of members) {
    out.set(m.worker_id, new Set(m.skill_ids));
  }
  return out;
}
