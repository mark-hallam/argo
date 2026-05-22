"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Check, UsersRound, X } from "lucide-react";
import {
  bulkAssignSkill,
  type CohortDetail,
  type CohortMember,
  type SkillCoverage,
} from "@/lib/api/cohorts";
import { ApiError } from "@/lib/api/types";
import { disableSkill } from "@/lib/api/worker-skills";
import { useToast } from "@/lib/ui/use-toast";
import { cn } from "@/lib/utils";
import { BulkAssignSkillModal } from "./BulkAssignSkillModal";
import { RemoveSkillConfirmModal } from "./RemoveSkillConfirmModal";

/**
 * MembersSection — extracted from `CohortDetailDrawer` so the drawer
 * stays under the 500-LOC per-file ceiling. Carries three operator
 * affordances on the same panel:
 *
 *   1. Bulk-select + bulk-assign-skill (sibling brief
 *      `mission-cohort-detail-bulk-assign-skill`): per-row checkbox +
 *      a "✓ skill X added" optimistic badge for ~3s on confirm.
 *   2. Per-row skill chips (this brief, `web-cohort-skill-removal`):
 *      one chip per skill the member holds, each carrying an × that
 *      opens [`RemoveSkillConfirmModal`].
 *   3. Confirm-modal-driven DELETE that calls `disableSkill` (the
 *      existing `DELETE /api/v1/workers/<id>/skills/<sid>` wrapper)
 *      and invalidates the cohort query so the activity feed picks
 *      up the new `worker.skills.removed` audit row within ~3s.
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

interface PendingRemoval {
  workerId: string;
  skillId: string;
  skillLabel: string;
}

export function CohortDetailMembers({ detail }: CohortDetailMembersProps) {
  const queryClient = useQueryClient();
  const toast = useToast();
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [modalOpen, setModalOpen] = useState(false);
  const [badge, setBadge] = useState<OptimisticBadge | null>(null);
  const [pending, setPending] = useState<PendingRemoval | null>(null);
  // Locally hide chips once the DELETE has succeeded so the operator
  // sees the change before the cohort query refetches. Keyed by
  // `${worker_id}|${skill_id}` so two members holding the same skill
  // stay independent.
  const [removedKeys, setRemovedKeys] = useState<Set<string>>(new Set());

  const memberIds = useMemo(
    () => detail.members.map((m) => m.worker_id),
    [detail.members],
  );

  const skillLabels = useMemo(
    () => buildSkillLabelLookup(detail.skills),
    [detail.skills],
  );

  // When the operator switches cohorts (different detail.cohort_id) drop
  // any stale selection — the worker ids no longer apply.
  useEffect(() => {
    setSelected(new Set());
    setBadge(null);
    setPending(null);
    setRemovedKeys(new Set());
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

  const handleBulkConfirm = useCallback(
    async (skillId: string, skillName: string) => {
      const workerIds = Array.from(selected);
      const result = await bulkAssignSkill(detail.cohort_id, workerIds, skillId);
      setBadge({ skillName, workerIds: new Set(workerIds) });
      setSelected(new Set());
      void queryClient.invalidateQueries({
        queryKey: ["cohort", detail.cohort_id],
      });
      return { assigned: result.assigned, already_had: result.already_had };
    },
    [detail.cohort_id, selected, queryClient],
  );

  const handleRemoveClick = useCallback(
    (workerId: string, skillId: string) => {
      setPending({
        workerId,
        skillId,
        skillLabel: skillLabels.get(skillId) ?? skillId,
      });
    },
    [skillLabels],
  );

  const handleRemoveConfirm = useCallback(async () => {
    if (pending === null) return;
    const { workerId, skillId, skillLabel } = pending;
    const key = chipKey(workerId, skillId);
    try {
      await disableSkill(workerId, skillId);
      setRemovedKeys((prev) => {
        const next = new Set(prev);
        next.add(key);
        return next;
      });
      toast.success(`Removed ${skillLabel}`);
      setPending(null);
      void queryClient.invalidateQueries({
        queryKey: ["cohort", detail.cohort_id],
      });
    } catch (err) {
      if (err instanceof ApiError && err.status === 404) {
        // Idempotent UX: the skill is already gone (racing tab /
        // actor). Hide the chip and refresh — no error surface.
        setRemovedKeys((prev) => {
          const next = new Set(prev);
          next.add(key);
          return next;
        });
        setPending(null);
        void queryClient.invalidateQueries({
          queryKey: ["cohort", detail.cohort_id],
        });
        return;
      }
      if (err instanceof ApiError && err.status === 403) {
        toast.error("You don't have permission to modify that worker.");
        // Rethrow so the modal surfaces the inline error + keeps focus.
        throw err;
      }
      toast.error(
        err instanceof Error ? err.message : "Failed to remove skill.",
      );
      throw err;
    }
  }, [pending, queryClient, detail.cohort_id, toast]);

  return (
    <>
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
            {detail.members.map((m) => (
              <MemberRow
                key={m.worker_id}
                member={m}
                isSelected={selected.has(m.worker_id)}
                badge={
                  (badge?.workerIds.has(m.worker_id) ?? false) ? badge : null
                }
                skillLabels={skillLabels}
                removedKeys={removedKeys}
                onToggle={() => toggleOne(m.worker_id)}
                onRemoveSkill={(skillId) =>
                  handleRemoveClick(m.worker_id, skillId)
                }
              />
            ))}
          </tbody>
        </table>
      </section>
      <BulkAssignSkillModal
        open={modalOpen}
        onOpenChange={setModalOpen}
        workerIds={Array.from(selected)}
        onConfirm={handleBulkConfirm}
      />
      <RemoveSkillConfirmModal
        open={pending !== null}
        onClose={() => setPending(null)}
        onConfirm={handleRemoveConfirm}
        skillLabel={pending?.skillLabel ?? ""}
        workerLabel={pending?.workerId ?? ""}
      />
    </>
  );
}

interface MemberRowProps {
  member: CohortMember;
  isSelected: boolean;
  badge: OptimisticBadge | null;
  skillLabels: Map<string, string>;
  removedKeys: Set<string>;
  onToggle: () => void;
  onRemoveSkill: (skillId: string) => void;
}

function MemberRow({
  member,
  isSelected,
  badge,
  skillLabels,
  removedKeys,
  onToggle,
  onRemoveSkill,
}: MemberRowProps) {
  const visibleSkills = member.skill_ids.filter(
    (sid) => !removedKeys.has(chipKey(member.worker_id, sid)),
  );
  return (
    <tr
      data-testid="cohort-detail-member-row"
      data-worker-id={member.worker_id}
      className="align-middle"
    >
      <td className="w-9 px-3 py-3">
        <input
          type="checkbox"
          data-testid="cohort-detail-select-row"
          data-worker-id={member.worker_id}
          aria-label={`Select ${member.worker_id}`}
          checked={isSelected}
          onChange={onToggle}
          className="size-4 accent-[var(--foreground)]"
        />
      </td>
      <td className="min-w-0 px-3 py-3">
        <div className="flex flex-wrap items-center gap-2">
          <Link
            href={`/workers/${encodeURIComponent(member.worker_id)}`}
            className="block max-w-[18rem] truncate text-sm font-medium text-[var(--foreground)] hover:underline"
          >
            {member.worker_id}
          </Link>
          {badge ? (
            <span
              data-testid="cohort-detail-row-badge"
              data-worker-id={member.worker_id}
              data-skill-name={badge.skillName}
              className="inline-flex items-center gap-1 rounded-full bg-emerald-50 px-2 py-0.5 text-[11px] font-medium text-emerald-700"
            >
              <Check className="size-3" aria-hidden="true" />
              {badge.skillName} added
            </span>
          ) : null}
        </div>
        <p className="text-xs text-[var(--muted-foreground)]">
          {visibleSkills.length}{" "}
          {visibleSkills.length === 1 ? "skill" : "skills"}
        </p>
        {visibleSkills.length === 0 ? null : (
          <div
            data-testid="cohort-detail-member-skill-chips"
            className="mt-2 flex flex-wrap gap-1.5"
          >
            {visibleSkills.map((sid) => (
              <SkillChip
                key={sid}
                workerLabel={member.worker_id}
                skillId={sid}
                label={skillLabels.get(sid) ?? sid}
                onRemove={() => onRemoveSkill(sid)}
              />
            ))}
          </div>
        )}
      </td>
      <td className="px-3 py-3 text-right">
        <span className="inline-flex items-center gap-1 rounded-full bg-[var(--muted)] px-2 py-0.5 text-[11px] font-medium tabular-nums text-[var(--foreground)]">
          grade {member.trust_grade}
        </span>
      </td>
    </tr>
  );
}

interface SkillChipProps {
  workerLabel: string;
  skillId: string;
  label: string;
  onRemove: () => void;
}

function SkillChip({ workerLabel, skillId, label, onRemove }: SkillChipProps) {
  return (
    <span
      data-testid="cohort-skill-chip"
      data-skill-id={skillId}
      className={cn(
        "inline-flex items-center gap-1 rounded-full border border-[var(--border)]",
        "bg-[var(--muted)]/40 py-0.5 pl-2 pr-1 text-[11px] font-medium",
        "text-[var(--foreground)]",
      )}
    >
      <span className="max-w-[180px] truncate">{label}</span>
      <button
        type="button"
        data-testid="cohort-skill-chip-remove"
        data-worker-id={workerLabel}
        data-skill-id={skillId}
        aria-label={`Remove skill ${label} from worker ${workerLabel}`}
        onClick={onRemove}
        className={cn(
          "grid size-4 place-items-center rounded-full text-[var(--muted-foreground)]",
          "hover:bg-[var(--border)] hover:text-[var(--foreground)]",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
        )}
      >
        <X aria-hidden className="size-3" />
      </button>
    </span>
  );
}

function chipKey(workerId: string, skillId: string): string {
  return `${workerId}|${skillId}`;
}

/**
 * Best-effort id→friendly-label map. Mock data + the cohort detail
 * gateway today populate `member.skill_ids[]` with the same string
 * `SkillCoverage.skill_name` carries, so the mapping is one-to-one
 * and humans see "follow_up" not a UUID. In a future where
 * `skill_ids[]` ever holds UUIDs, we'd extend `SkillCoverage` with a
 * real id field; until then chips fall back to the raw id when no
 * match exists.
 */
function buildSkillLabelLookup(skills: SkillCoverage[]): Map<string, string> {
  const out = new Map<string, string>();
  for (const s of skills) {
    out.set(s.skill_name, s.skill_name);
  }
  return out;
}
