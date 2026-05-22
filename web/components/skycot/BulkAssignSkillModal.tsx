"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { Search, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { fetchSkills, type SkillRow } from "@/lib/api/skills";
import { useToast } from "@/lib/ui/use-toast";

/**
 * BulkAssignSkillModal — search + pick a skill, then confirm. The
 * parent owns the worker-id selection and the `onConfirm` callback
 * fires the gateway POST. The modal closes itself after a successful
 * confirm; the parent then refreshes the cohort detail query so the
 * optimistic per-row badge can settle into real audit-feed rows.
 *
 * `skillsOverride` is a test seam — when provided the modal skips
 * the skills fetch so unit tests can drive the picker deterministically.
 */
export interface BulkAssignSkillModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Worker ids the bulk action applies to. */
  workerIds: string[];
  /**
   * Fires when the operator confirms. Returns the assigned/already_had
   * counts (echoed from the gateway). Errors surface via toast.
   */
  onConfirm: (
    skillId: string,
    skillName: string,
  ) => Promise<{ assigned: number; already_had: number }>;
  /** Test seam — pre-populate the skill list and skip the fetch. */
  skillsOverride?: SkillRow[];
}

export function BulkAssignSkillModal({
  open,
  onOpenChange,
  workerIds,
  onConfirm,
  skillsOverride,
}: BulkAssignSkillModalProps) {
  const toast = useToast();
  const [query, setQuery] = useState("");
  const [selectedSkillId, setSelectedSkillId] = useState<string | null>(null);
  const [skills, setSkills] = useState<SkillRow[]>(skillsOverride ?? []);
  const [loading, setLoading] = useState<boolean>(skillsOverride === undefined);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setSelectedSkillId(null);
  }, [open]);

  useEffect(() => {
    if (!open || skillsOverride !== undefined) return;
    let cancelled = false;
    setLoading(true);
    fetchSkills()
      .then((res) => {
        if (cancelled) return;
        setSkills(res.skills);
        setLoading(false);
      })
      .catch(() => {
        if (cancelled) return;
        setSkills([]);
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open, skillsOverride]);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return skills;
    return skills.filter(
      (s) =>
        s.name.toLowerCase().includes(needle) ||
        s.description.toLowerCase().includes(needle),
    );
  }, [skills, query]);

  const selectedSkill = useMemo(
    () => skills.find((s) => s.id === selectedSkillId) ?? null,
    [skills, selectedSkillId],
  );

  async function handleConfirm() {
    if (selectedSkill === null) return;
    setSubmitting(true);
    try {
      const result = await onConfirm(selectedSkill.id, selectedSkill.name);
      toast.success(
        `Assigned ${selectedSkill.name} to ${result.assigned} worker${
          result.assigned === 1 ? "" : "s"
        }`,
        {
          description:
            result.already_had > 0
              ? `${result.already_had} already had this skill — no change.`
              : undefined,
        },
      );
      onOpenChange(false);
    } catch (err) {
      toast.error("Bulk assign failed", {
        description: err instanceof Error ? err.message : "Unknown error.",
      });
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-[rgb(0,0,0)]/40 data-[state=open]:animate-in data-[state=open]:fade-in" />
        <DialogPrimitive.Content
          data-testid="bulk-assign-skill-modal"
          className="fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-5 shadow-xl"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="space-y-1">
              <DialogPrimitive.Title className="text-sm font-semibold text-[var(--foreground)]">
                Bulk assign skill
              </DialogPrimitive.Title>
              <DialogPrimitive.Description className="text-xs text-[var(--muted-foreground)]">
                Apply one skill to {workerIds.length} selected worker
                {workerIds.length === 1 ? "" : "s"}.
              </DialogPrimitive.Description>
            </div>
            <DialogPrimitive.Close
              aria-label="Close"
              className="grid size-7 place-items-center rounded-[var(--radius-md)] text-[var(--muted-foreground)] hover:bg-[var(--muted)] hover:text-[var(--foreground)]"
            >
              <X className="size-4" />
            </DialogPrimitive.Close>
          </div>
          <div className="mt-4 space-y-3">
            <label className="flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-2">
              <Search
                className="size-4 text-[var(--muted-foreground)]"
                aria-hidden="true"
              />
              <input
                data-testid="bulk-assign-skill-search"
                type="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search skills…"
                className="w-full bg-transparent text-sm text-[var(--foreground)] outline-none placeholder:text-[var(--muted-foreground)]"
                autoFocus
              />
            </label>
            <ul
              data-testid="bulk-assign-skill-options"
              className="max-h-64 overflow-y-auto divide-y divide-[var(--border)] rounded-[var(--radius-md)] border border-[var(--border)]"
            >
              {loading ? (
                <li className="px-3 py-4 text-center text-xs text-[var(--muted-foreground)]">
                  Loading skills…
                </li>
              ) : filtered.length === 0 ? (
                <li className="px-3 py-4 text-center text-xs text-[var(--muted-foreground)]">
                  No skills match {query ? `"${query}"` : "this workspace"}.
                </li>
              ) : (
                filtered.map((skill) => {
                  const isSelected = skill.id === selectedSkillId;
                  return (
                    <li key={skill.id}>
                      <button
                        type="button"
                        data-testid="bulk-assign-skill-option"
                        data-skill-id={skill.id}
                        data-skill-name={skill.name}
                        aria-pressed={isSelected}
                        onClick={() => setSelectedSkillId(skill.id)}
                        className={
                          "w-full px-3 py-2 text-left transition-colors hover:bg-[var(--muted)]/40" +
                          (isSelected
                            ? " bg-[var(--muted)]/60 text-[var(--foreground)]"
                            : " text-[var(--foreground)]")
                        }
                      >
                        <div className="text-sm font-medium">{skill.name}</div>
                        <div className="text-[11px] text-[var(--muted-foreground)]">
                          v{skill.version} · {skill.layer}
                        </div>
                      </button>
                    </li>
                  );
                })
              )}
            </ul>
          </div>
          <div className="mt-5 flex items-center justify-end gap-2">
            <DialogPrimitive.Close asChild>
              <button
                type="button"
                data-testid="bulk-assign-skill-cancel"
                className="rounded-[var(--radius-md)] px-3 py-1.5 text-xs font-medium text-[var(--muted-foreground)] hover:bg-[var(--muted)]"
              >
                Cancel
              </button>
            </DialogPrimitive.Close>
            <button
              type="button"
              data-testid="bulk-assign-skill-confirm"
              disabled={selectedSkill === null || submitting}
              onClick={handleConfirm}
              className="inline-flex items-center gap-2 rounded-[var(--radius-md)] bg-[var(--foreground)] px-3 py-1.5 text-xs font-medium text-[var(--background)] enabled:hover:opacity-90 disabled:opacity-40"
            >
              {submitting ? "Assigning…" : "Assign skill"}
            </button>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
