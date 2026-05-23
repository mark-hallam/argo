"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Plus, X } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { DutyEditorRow } from "@/components/skycot/hire/DutyEditorRow";
import { VoiceProfileEditor } from "@/components/skycot/hire/VoiceProfileEditor";
import type {
  WorkerDraft,
  WorkerDuty,
  WorkerVoiceProfile,
} from "@/lib/hire/draft-types";
import { parseCron } from "@/lib/cron/parse";
import { isPhraseAcceptable } from "@/lib/moderation/check-phrase";
import { cn } from "@/lib/utils";

const MIN_CHARTER_LEN = 20;

/**
 * Right-side drawer for structured editing of the Step 4 worker
 * draft.
 *
 * Distinct from `AdjustDrawer` (E2-e): that one is free-text and
 * re-runs Genesis. This one mutates `workerDraft` directly with no
 * LLM call. The two coexist on the worker card.
 *
 * On Save the drawer:
 *  - validates all duty cron strings + charter min-length + at least
 *    one duty enabled;
 *  - filters signature phrases through the wordlist (rejected phrases
 *    show a toast and are dropped from the saved set);
 *  - emits the merged `WorkerDraft` via `onSave`, plus a cleared
 *    `exampleReplies` array so the cached Genesis examples don't
 *    leak as stale UI.
 *
 * Validation surfaces in two places: inline cron error under each
 * duty row, and the Save button itself is disabled while any
 * blocking condition holds. Cancel discards the draft state.
 */
export interface StructuredAdjustDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  draft: WorkerDraft;
  /**
   * Fires once when the user clicks Save. The parent owns closing
   * the drawer + surfacing the rejected-phrase toast (if any).
   */
  onSave: (next: WorkerDraft, rejectedPhrases: string[]) => void;
}

interface DraftState {
  charter: string;
  duties: WorkerDuty[];
  voice: WorkerVoiceProfile;
  phrasesText: string;
}

function initialDraftState(draft: WorkerDraft): DraftState {
  return {
    charter: draft.charter,
    duties: draft.duties.map((d) => ({ ...d, schedule: { ...d.schedule } })),
    voice: { ...draft.persona.voiceProfile },
    phrasesText: draft.persona.voiceProfile.signaturePhrases.join("\n"),
  };
}

function makeEmptyDuty(): WorkerDuty {
  return {
    label: "",
    schedule: { kind: "cron", spec: "0 9 * * *" },
    defaultEnabled: true,
  };
}

function dutiesAllValid(duties: ReadonlyArray<WorkerDuty>): boolean {
  return duties.every((d) => {
    if (d.label.trim().length === 0) return false;
    if (d.schedule.kind === "cron") {
      return parseCron(d.schedule.spec).ok;
    }
    return d.schedule.spec.trim().length > 0;
  });
}

export function StructuredAdjustDrawer({
  open,
  onOpenChange,
  draft,
  onSave,
}: StructuredAdjustDrawerProps) {
  const [state, setState] = useState<DraftState>(() => initialDraftState(draft));

  // Reset draft state every time the drawer opens so stale edits
  // don't leak between sessions.
  useEffect(() => {
    if (open) {
      setState(initialDraftState(draft));
    }
  }, [open, draft]);

  const charterTooShort = state.charter.trim().length < MIN_CHARTER_LEN;
  const anyDutyEnabled = state.duties.some((d) => d.defaultEnabled);
  const dutiesValid = dutiesAllValid(state.duties);

  const saveDisabled =
    charterTooShort || !anyDutyEnabled || !dutiesValid || state.duties.length === 0;

  const updateCharter = (next: string) =>
    setState((prev) => ({ ...prev, charter: next }));

  const updateDuty = useCallback((index: number, next: WorkerDuty) => {
    setState((prev) => ({
      ...prev,
      duties: prev.duties.map((d, i) => (i === index ? next : d)),
    }));
  }, []);

  const removeDuty = useCallback((index: number) => {
    setState((prev) => ({
      ...prev,
      duties: prev.duties.filter((_, i) => i !== index),
    }));
  }, []);

  const addDuty = () =>
    setState((prev) => ({ ...prev, duties: [...prev.duties, makeEmptyDuty()] }));

  const updateVoice = (voice: WorkerVoiceProfile) =>
    setState((prev) => ({ ...prev, voice }));

  const updatePhrasesText = (phrasesText: string) =>
    setState((prev) => ({ ...prev, phrasesText }));

  // Row-level validity tracking. The DutyEditorRow notifies us via
  // `onValidationChange`; we keep the canonical check inside
  // `dutiesAllValid` so the dropdown isn't strictly necessary, but
  // surfacing inline-row error UI still requires the per-row callback.
  const handleRowValidation = useCallback(() => {
    // The state-level `dutiesValid` is recomputed each render via
    // `dutiesAllValid`; no need to mirror it in state.
  }, []);

  const buildSavedPhrases = (): { kept: string[]; rejected: string[] } => {
    const lines = state.phrasesText
      .split("\n")
      .map((l) => l.trim())
      .filter((l) => l.length > 0);
    const kept: string[] = [];
    const rejected: string[] = [];
    for (const phrase of lines) {
      const check = isPhraseAcceptable(phrase);
      if (check.ok) kept.push(phrase);
      else rejected.push(phrase);
    }
    return { kept, rejected };
  };

  const handleSave = () => {
    if (saveDisabled) return;
    const { kept, rejected } = buildSavedPhrases();
    const merged: WorkerDraft = {
      ...draft,
      charter: state.charter.trim(),
      duties: state.duties.map((d) => ({
        ...d,
        label: d.label.trim(),
        schedule: { ...d.schedule, spec: d.schedule.spec.trim() },
      })),
      persona: {
        ...draft.persona,
        voiceProfile: {
          tone: state.voice.tone,
          formality: state.voice.formality,
          signaturePhrases: kept,
        },
      },
      // Cached Genesis examples no longer match the structurally
      // edited draft — clear so the surface invalidates cleanly.
      // The wireframe-mockup holding card sits inside Step4Introduce
      // which is locked from E2-e; clearing the array is the safest
      // available signal that the cached examples are stale.
      exampleReplies: [],
    };
    onSave(merged, rejected);
  };

  const charterErrorId = "structured-adjust-charter-error";

  const dutiesEnabledLabel = useMemo(() => {
    const total = state.duties.length;
    const enabled = state.duties.filter((d) => d.defaultEnabled).length;
    return `${enabled} of ${total} duty${total === 1 ? "" : "ies"} enabled`;
  }, [state.duties]);

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          className={cn(
            "fixed inset-0 z-40 bg-black/40",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
          )}
        />
        <DialogPrimitive.Content
          data-testid="hire-step4-structured-drawer"
          className={cn(
            "fixed inset-y-0 right-0 z-50 flex h-full w-full max-w-lg flex-col",
            "border-l border-[var(--border)] bg-[var(--background)] shadow-xl",
            "focus:outline-none",
            // mobile: full-screen overlay.
            "sm:max-w-lg",
          )}
        >
          <header className="flex items-start justify-between gap-3 border-b border-[var(--border)] px-5 py-4">
            <div className="space-y-1">
              <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
                Edit teammate fields
              </DialogPrimitive.Title>
              <DialogPrimitive.Description className="text-xs text-[var(--muted-foreground)]">
                Direct edits to charter, schedules, and voice — no Genesis
                re-run.
              </DialogPrimitive.Description>
            </div>
            <DialogPrimitive.Close
              data-testid="hire-step4-structured-close"
              className="rounded-[var(--radius-sm)] p-1 text-[var(--muted-foreground)] hover:bg-[var(--accent)] hover:text-[var(--accent-foreground)]"
              aria-label="Close structured edit drawer"
            >
              <X className="size-4" />
            </DialogPrimitive.Close>
          </header>

          <div className="flex-1 space-y-6 overflow-y-auto px-5 py-4">
            <section
              data-testid="hire-step4-structured-charter"
              aria-label="Charter"
              className="space-y-2"
            >
              <label className="block text-xs font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">
                Charter
              </label>
              <textarea
                data-testid="hire-step4-structured-charter-input"
                value={state.charter}
                onChange={(e) => updateCharter(e.target.value)}
                rows={5}
                aria-invalid={charterTooShort || undefined}
                aria-describedby={charterTooShort ? charterErrorId : undefined}
                className={cn(
                  "block w-full resize-none rounded-[var(--radius-md)] border bg-[var(--background)] p-2 text-sm",
                  "outline-none focus:ring-2 focus:ring-[var(--ring)]",
                  charterTooShort
                    ? "border-[var(--destructive)]"
                    : "border-[var(--border)]",
                )}
              />
              {charterTooShort ? (
                <p
                  id={charterErrorId}
                  role="alert"
                  className="text-xs text-[var(--destructive)]"
                >
                  Charter must be at least {MIN_CHARTER_LEN} characters.
                </p>
              ) : null}
            </section>

            <section
              data-testid="hire-step4-structured-duties"
              aria-label="Schedules"
              className="space-y-2"
            >
              <div className="flex items-center justify-between">
                <label className="block text-xs font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">
                  Schedules
                </label>
                <span className="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">
                  {dutiesEnabledLabel}
                </span>
              </div>
              <div className="space-y-2">
                {state.duties.map((duty, idx) => (
                  <DutyEditorRow
                    key={idx}
                    duty={duty}
                    index={idx}
                    onChange={updateDuty}
                    onRemove={removeDuty}
                    onValidationChange={handleRowValidation}
                  />
                ))}
              </div>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                data-testid="hire-step4-structured-add-duty"
                onClick={addDuty}
                className="w-full justify-center border border-dashed border-[var(--border)]"
              >
                <Plus className="size-3.5" aria-hidden />
                Add schedule
              </Button>
              {!anyDutyEnabled ? (
                <p
                  role="alert"
                  data-testid="hire-step4-structured-no-duty-error"
                  className="text-xs text-[var(--destructive)]"
                >
                  Enable at least one schedule before saving.
                </p>
              ) : null}
            </section>

            <VoiceProfileEditor
              voice={state.voice}
              phrasesText={state.phrasesText}
              onVoiceChange={updateVoice}
              onPhrasesTextChange={updatePhrasesText}
            />
          </div>

          <footer className="flex justify-end gap-2 border-t border-[var(--border)] px-5 py-4">
            <Button
              type="button"
              variant="ghost"
              data-testid="hire-step4-structured-cancel"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="primary"
              data-testid="hire-step4-structured-save"
              onClick={handleSave}
              disabled={saveDisabled}
            >
              Save changes
            </Button>
          </footer>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
