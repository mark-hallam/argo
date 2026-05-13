"use client";

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { Pencil } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { DutyToggleRow } from "@/components/skycot/hire/DutyToggleRow";
import type { WorkerDraft } from "@/lib/hire/draft-types";
import { cn } from "@/lib/utils";

/**
 * Step 4's primary surface — the proposed teammate profile.
 *
 * Sections (top → bottom):
 *  - Header row: avatar + name + tagline (both inline-editable) on
 *    the left, "Adjust" ghost button on the right.
 *  - Charter block: 2–3 sentence summary, inline-editable as textarea.
 *  - Duties block: 3–5 `DutyToggleRow` rows controlled by
 *    `onToggleDuty`.
 *
 * Inline editing: click the field to swap into an input/textarea.
 * Enter commits (Cmd/Ctrl+Enter for textarea), blur commits, Esc
 * cancels. Inputs auto-focus + auto-size to match the displayed span.
 */
export interface WorkerDraftCardProps {
  draft: WorkerDraft;
  onChange: (next: WorkerDraft) => void;
  onAdjust: () => void;
  /** Disable the Adjust button while a redraft is in flight. */
  adjustDisabled?: boolean;
}

function initialsFor(name: string): string {
  const trimmed = name.trim();
  if (!trimmed) return "??";
  const parts = trimmed.split(/\s+/);
  if (parts.length === 1) return parts[0]!.slice(0, 2).toUpperCase();
  return `${parts[0]!.charAt(0)}${parts[parts.length - 1]!.charAt(0)}`.toUpperCase();
}

export function WorkerDraftCard({
  draft,
  onChange,
  onAdjust,
  adjustDisabled = false,
}: WorkerDraftCardProps) {
  const updateName = (next: string) =>
    onChange({ ...draft, suggestedName: next });
  const updateTagline = (next: string) =>
    onChange({ ...draft, persona: { ...draft.persona, tagline: next } });
  const updateCharter = (next: string) => onChange({ ...draft, charter: next });
  const toggleDuty = (index: number, enabled: boolean) => {
    onChange({
      ...draft,
      duties: draft.duties.map((d, i) =>
        i === index ? { ...d, defaultEnabled: enabled } : d,
      ),
    });
  };

  return (
    <article
      data-testid="hire-step4-worker-card"
      className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 shadow-sm space-y-5"
    >
      <div className="flex items-start gap-4">
        <div
          data-testid="hire-step4-avatar"
          className="flex size-14 shrink-0 items-center justify-center rounded-full bg-[var(--primary)] text-lg font-semibold text-[var(--primary-foreground)]"
          aria-hidden
        >
          {initialsFor(draft.suggestedName)}
        </div>
        <div className="flex-1 min-w-0 space-y-1">
          <InlineEditable
            kind="text"
            value={draft.suggestedName}
            onChange={updateName}
            testId="hire-step4-name"
            ariaLabel="Edit teammate name"
            minLength={1}
            displayClassName="text-xl font-semibold text-[var(--foreground)]"
          />
          <InlineEditable
            kind="text"
            value={draft.persona.tagline}
            onChange={updateTagline}
            testId="hire-step4-tagline"
            ariaLabel="Edit role tagline"
            placeholder="Role tagline"
            displayClassName="text-sm text-[var(--muted-foreground)]"
          />
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          data-testid="hire-step4-adjust"
          onClick={onAdjust}
          disabled={adjustDisabled}
          aria-label={`Adjust ${draft.suggestedName}`}
        >
          <Pencil className="size-3.5" aria-hidden />
          <span>Adjust</span>
        </Button>
      </div>

      <InlineEditable
        kind="textarea"
        value={draft.charter}
        onChange={updateCharter}
        testId="hire-step4-charter"
        ariaLabel="Edit charter"
        placeholder="Two to three sentences describing the role."
        displayClassName="text-sm leading-relaxed text-[var(--foreground)]"
      />

      {draft.duties.length > 0 ? (
        <section
          data-testid="hire-step4-duties"
          className="space-y-2"
          aria-label="Default duties"
        >
          <h3 className="text-xs font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">
            Default duties
          </h3>
          <div className="space-y-2">
            {draft.duties.map((duty, idx) => (
              <DutyToggleRow
                key={`${duty.label}-${idx}`}
                duty={duty}
                index={idx}
                onToggle={toggleDuty}
              />
            ))}
          </div>
        </section>
      ) : null}
    </article>
  );
}

interface InlineEditableProps {
  kind: "text" | "textarea";
  value: string;
  onChange: (next: string) => void;
  testId: string;
  ariaLabel: string;
  placeholder?: string;
  minLength?: number;
  displayClassName?: string;
}

function InlineEditable({
  kind,
  value,
  onChange,
  testId,
  ariaLabel,
  placeholder,
  minLength = 0,
  displayClassName,
}: InlineEditableProps) {
  const [editing, setEditing] = useState(false);
  const [draftValue, setDraftValue] = useState(value);
  const inputRef = useRef<HTMLInputElement | HTMLTextAreaElement | null>(null);

  useEffect(() => {
    if (!editing) setDraftValue(value);
  }, [editing, value]);
  useEffect(() => {
    if (editing) inputRef.current?.focus();
  }, [editing]);

  const commit = useCallback(() => {
    const next = draftValue.trim();
    if (next.length < minLength) {
      setDraftValue(value);
      setEditing(false);
      return;
    }
    if (next !== value) onChange(next);
    setEditing(false);
  }, [draftValue, minLength, onChange, value]);

  const cancel = useCallback(() => {
    setDraftValue(value);
    setEditing(false);
  }, [value]);

  const handleKey = (e: KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    if (e.key === "Escape") {
      e.preventDefault();
      cancel();
      return;
    }
    if (e.key === "Enter") {
      if (kind === "textarea" && !(e.metaKey || e.ctrlKey)) return;
      e.preventDefault();
      commit();
    }
  };

  if (editing) {
    const sharedProps = {
      value: draftValue,
      onBlur: commit,
      onKeyDown: handleKey,
      placeholder,
      "data-testid": `${testId}-input`,
      "aria-label": ariaLabel,
    } as const;
    if (kind === "textarea") {
      return (
        <textarea
          ref={(el) => {
            inputRef.current = el;
          }}
          rows={4}
          onChange={(e) => setDraftValue(e.target.value)}
          {...sharedProps}
          className={cn(
            "block w-full resize-none rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] p-2 outline-none focus:ring-2 focus:ring-[var(--ring)]",
            displayClassName,
          )}
        />
      );
    }
    return (
      <input
        ref={(el) => {
          inputRef.current = el;
        }}
        type="text"
        onChange={(e) => setDraftValue(e.target.value)}
        {...sharedProps}
        className={cn(
          "block w-full rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--background)] px-1.5 py-0.5 outline-none focus:ring-2 focus:ring-[var(--ring)]",
          displayClassName,
        )}
      />
    );
  }

  return (
    <button
      type="button"
      data-testid={testId}
      onClick={() => setEditing(true)}
      aria-label={ariaLabel}
      className={cn(
        "block w-full text-left rounded-[var(--radius-sm)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
        kind === "textarea"
          ? "whitespace-pre-line border border-transparent px-2 py-1 hover:border-[var(--border)]"
          : "truncate hover:underline",
        displayClassName,
      )}
    >
      {value || placeholder || ""}
    </button>
  );
}
