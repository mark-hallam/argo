"use client";

import { useState } from "react";
import { Loader2, Sparkles, AlertCircle } from "lucide-react";
import { draftSkillFromPrompt } from "@/lib/api/ghostwriter";
import type { GhostwriterDraft } from "@/lib/api/ghostwriter";
import { cn } from "@/lib/utils";

/**
 * Ghostwriter prompt-entry surface (L6).
 *
 * Operator types a natural-language description of a skill; on submit
 * the gateway returns a validated IR + new `skillId`. The page wraps
 * this component with a redirect to the L4 canvas so the canvas is
 * never blank — the canonical UX answer to "blank-grid paralysis".
 *
 * Client-side validation mirrors the gateway (`MIN_PROMPT_LEN = 15`)
 * so the user gets immediate feedback instead of a 400 round-trip.
 */

const MIN_PROMPT_LEN = 15;
const SAMPLE_PROMPT =
  "Build a skill that reconciles bank statements weekly: fetch the bank export, match line items against the GL, flag deltas over $500 for CFO approval.";

export interface GhostwriterFormProps {
  /** Called once the gateway returns a fresh draft. The page redirects
   *  to the L4 canvas using the `skillId` + `version` from the draft. */
  onDraftReady: (draft: GhostwriterDraft) => void;
  /** Optional initial prompt; defaults to empty. */
  initialPrompt?: string;
}

export function GhostwriterForm({ onDraftReady, initialPrompt }: GhostwriterFormProps) {
  const [prompt, setPrompt] = useState(initialPrompt ?? "");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const trimmed = prompt.trim();
  const tooShort = trimmed.length > 0 && trimmed.length < MIN_PROMPT_LEN;
  const canSubmit = !submitting && trimmed.length >= MIN_PROMPT_LEN;

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    setSubmitting(true);
    setError(null);
    try {
      const draft = await draftSkillFromPrompt(trimmed);
      onDraftReady(draft);
    } catch (err) {
      const message =
        err instanceof Error ? err.message : "Couldn't draft the skill";
      setError(message);
      setSubmitting(false);
    }
  }

  return (
    <form
      data-testid="ghostwriter-form"
      onSubmit={handleSubmit}
      className="space-y-4 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-6"
    >
      <header className="space-y-2">
        <h2 className="text-lg font-semibold text-[var(--foreground)]">
          Describe the skill
        </h2>
        <p className="text-sm text-[var(--muted-foreground)]">
          Plain English works best. We&apos;ll draft the workflow on the
          canvas, then you can edit it node-by-node.
        </p>
      </header>

      <label
        htmlFor="ghostwriter-prompt"
        className="block text-sm font-medium text-[var(--foreground)]"
      >
        What should this skill do?
      </label>
      <textarea
        id="ghostwriter-prompt"
        data-testid="ghostwriter-prompt"
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        rows={6}
        placeholder={SAMPLE_PROMPT}
        disabled={submitting}
        aria-invalid={tooShort || Boolean(error)}
        aria-describedby="ghostwriter-help"
        className={cn(
          "w-full rounded-[var(--radius-md)] border bg-[var(--background)] px-3 py-2 text-sm",
          "border-[var(--border)] text-[var(--foreground)]",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
          "disabled:opacity-60",
          tooShort && "border-[var(--destructive)]",
        )}
      />
      <p
        id="ghostwriter-help"
        className="text-xs text-[var(--muted-foreground)]"
      >
        {tooShort
          ? `At least ${MIN_PROMPT_LEN} characters needed (you have ${trimmed.length}).`
          : `Tip: include the trigger, the inputs, the decision, and who approves.`}
      </p>

      {error ? (
        <div
          role="alert"
          data-testid="ghostwriter-error"
          className="flex items-start gap-2 rounded-[var(--radius-md)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 p-3 text-sm text-[var(--foreground)]"
        >
          <AlertCircle className="mt-0.5 size-4 shrink-0 text-[var(--destructive)]" />
          <span>{error}</span>
        </div>
      ) : null}

      <div className="flex items-center justify-end gap-2">
        <button
          type="button"
          data-testid="ghostwriter-use-sample"
          onClick={() => setPrompt(SAMPLE_PROMPT)}
          className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
        >
          Use sample
        </button>
        <button
          type="submit"
          data-testid="ghostwriter-submit"
          disabled={!canSubmit}
          className={cn(
            "inline-flex items-center gap-2 rounded-[var(--radius-md)] bg-[var(--primary)] px-4 py-2 text-sm font-medium text-[var(--primary-foreground)] shadow-sm",
            "hover:bg-[var(--primary)]/90 disabled:opacity-60 disabled:cursor-not-allowed",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--background)]",
          )}
        >
          {submitting ? (
            <Loader2 className="size-4 animate-spin" aria-hidden="true" />
          ) : (
            <Sparkles className="size-4" aria-hidden="true" />
          )}
          {submitting ? "Drafting…" : "Draft skill"}
        </button>
      </div>
    </form>
  );
}
