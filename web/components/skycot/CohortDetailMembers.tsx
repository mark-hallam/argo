"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Check, UsersRound } from "lucide-react";
import { bulkAssignSkill, type CohortDetail } from "@/lib/api/cohorts";
import { BulkAssignSkillModal } from "./BulkAssignSkillModal";

/**
 * MembersSection — extracted from `CohortDetailDrawer` so the drawer
 * stays under the 500-LOC per-file ceiling and so this section can
 * grow the bulk-select + bulk-assign-skill machinery the
 * `mission-cohort-detail-bulk-assign-skill` brief calls for.
 *
 * Selection state is local — closing the drawer naturally drops it.
 * Optimistic "✓ skill X added" badge is shown per-row for ~3s after a
 * successful bulk-assign; the React Query cohort-detail query is
 * invalidated on the same tick so the real skill chip lands shortly
 * after the badge fades.
 */
export interface CohortDetailMembersProps {
  detail: CohortDetail;
}

/** How long the optimistic "✓ skill X added" badge stays visible. */
const OPTIMISTIC_BADGE_MS = 3000;

interface OptimisticBadge {
  skillName: string;
  workerIds: Set<string>;
}

export function CohortDetailMembers({ detail }: CohortDetailMembersProps) {
  const queryClient = useQueryClient();
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [modalOpen, setModalOpen] = useState(false);
  const [badge, setBadge] = useState<OptimisticBadge | null>(null);

  const memberIds = useMemo(
    () => detail.members.map((m) => m.worker_id),
    [detail.members],
  );

  // When the operator switches cohorts (different detail.cohort_id) drop
  // any stale selection — the worker ids no longer apply.
  useEffect(() => {
    setSelected(new Set());
    setBadge(null);
  }, [detail.cohort_id]);

  useEffect(() => {
    if (badge === null) return;
    const handle = window.setTimeout(() => setBadge(null), OPTIMISTIC_BADGE_MS);
    return () => window.clearTimeout(handle);
  }, [badge]);

  const allSelected = selected.size > 0 && selected.size === memberIds.length;
  const someSelected = selected.size > 0 && !allSelected;

  const toggleAll = useCallback(() => {
    setSelected((prev) =>
      prev.size === memberIds.length ? new Set() : new Set(memberIds),
    );
  }, [memberIds]);

  const toggleOne = useCallback((workerId: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(workerId)) {
        next.delete(workerId);
      } else {
        next.add(workerId);
      }
      return next;
    });
  }, []);

  const handleConfirm = useCallback(
    async (skillId: string, skillName: string) => {
      const workerIds = Array.from(selected);
      const result = await bulkAssignSkill(detail.cohort_id, workerIds, skillId);
      setBadge({ skillName, workerIds: new Set(workerIds) });
      setSelected(new Set());
      // The cohort detail query carries members + skills + recent
      // activity. Invalidate so the new audit rows + skill chips show
      // up; the optimistic badge bridges the 200-400ms refetch.
      void queryClient.invalidateQueries({
        queryKey: ["cohort", detail.cohort_id],
      });
      return { assigned: result.assigned, already_had: result.already_had };
    },
    [detail.cohort_id, selected, queryClient],
  );

  return (
    <section aria-labelledby="cohort-detail-members-heading" className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        <h3
          id="cohort-detail-members-heading"
          className="text-xs font-medium uppercase tracking-wider text-[var(--muted-foreground)]"
        >
          Members ({detail.members.length})
        </h3>
        {selected.size > 0 ? (
          <button
            type="button"
            data-testid="cohort-detail-bulk-assign-button"
            onClick={() => setModalOpen(true)}
            className="inline-flex items-center gap-1.5 rounded-[var(--radius-md)] bg-[var(--foreground)] px-2.5 py-1 text-[11px] font-medium text-[var(--background)] hover:opacity-90"
          >
            <UsersRound className="size-3.5" aria-hidden="true" />
            Assign skill to {selected.size}
          </button>
        ) : null}
      </div>
      <table
        data-testid="cohort-detail-members"
        className="w-full divide-y divide-[var(--border)] rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] text-sm"
      >
        <thead>
          <tr className="text-left text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
            <th scope="col" className="w-9 px-3 py-2">
              <input
                type="checkbox"
                data-testid="cohort-detail-select-all"
                aria-label="Select all members"
                checked={allSelected}
                ref={(el) => {
                  if (el) el.indeterminate = someSelected;
                }}
                onChange={toggleAll}
                className="size-4 accent-[var(--foreground)]"
              />
            </th>
            <th scope="col" className="px-3 py-2">
              Worker
            </th>
            <th scope="col" className="px-3 py-2 text-right">
              Grade
            </th>
          </tr>
        </thead>
        <tbody className="divide-y divide-[var(--border)]">
          {detail.members.map((m) => {
            const isSelected = selected.has(m.worker_id);
            const showBadge = badge?.workerIds.has(m.worker_id) ?? false;
            return (
              <tr
                key={m.worker_id}
                data-testid="cohort-detail-member-row"
                data-worker-id={m.worker_id}
                className="align-middle"
              >
                <td className="w-9 px-3 py-3">
                  <input
                    type="checkbox"
                    data-testid="cohort-detail-select-row"
                    data-worker-id={m.worker_id}
                    aria-label={`Select ${m.worker_id}`}
                    checked={isSelected}
                    onChange={() => toggleOne(m.worker_id)}
                    className="size-4 accent-[var(--foreground)]"
                  />
                </td>
                <td className="min-w-0 px-3 py-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <Link
                      href={`/workers/${encodeURIComponent(m.worker_id)}`}
                      className="block max-w-[18rem] truncate text-sm font-medium text-[var(--foreground)] hover:underline"
                    >
                      {m.worker_id}
                    </Link>
                    {showBadge && badge ? (
                      <span
                        data-testid="cohort-detail-row-badge"
                        data-worker-id={m.worker_id}
                        data-skill-name={badge.skillName}
                        className="inline-flex items-center gap-1 rounded-full bg-emerald-50 px-2 py-0.5 text-[11px] font-medium text-emerald-700"
                      >
                        <Check className="size-3" aria-hidden="true" />
                        {badge.skillName} added
                      </span>
                    ) : null}
                  </div>
                  <p className="text-xs text-[var(--muted-foreground)]">
                    {m.skill_ids.length}{" "}
                    {m.skill_ids.length === 1 ? "skill" : "skills"}
                  </p>
                </td>
                <td className="px-3 py-3 text-right">
                  <span className="inline-flex items-center gap-1 rounded-full bg-[var(--muted)] px-2 py-0.5 text-[11px] font-medium tabular-nums text-[var(--foreground)]">
                    grade {m.trust_grade}
                  </span>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <BulkAssignSkillModal
        open={modalOpen}
        onOpenChange={setModalOpen}
        workerIds={Array.from(selected)}
        onConfirm={handleConfirm}
      />
    </section>
  );
}
