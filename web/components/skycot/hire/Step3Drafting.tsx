"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import {
  DRAFTING_STAGES,
  DraftingProgressList,
} from "@/components/skycot/hire/DraftingProgressList";
import { EXAMPLE_ROLES, type ExampleRole } from "@/lib/hire/examples";
import type { WorkerDraft } from "@/lib/hire/draft-types";
import type { QuestionAnswer } from "@/lib/hire/genesis-types";
import { useGenesisWorkerDraft } from "@/lib/hire/use-genesis-worker-draft";

/**
 * Hire flow — Step 3 (Drafting transition).
 *
 * Fires `useGenesisWorkerDraft` on mount and runs a 5-stage cosmetic
 * progress checklist alongside. The view is non-interactive except
 * for the retry panel (>30s wall) and the skip-to-template fallback
 * (>3 retries). Behavior:
 *
 *  - Cosmetic timer advances every `STAGE_INTERVAL_MS`, stopping at
 *    stage 4 ("Almost done…").
 *  - Completion fires when BOTH the cosmetic timer has reached
 *    "Almost done" AND the backend has returned `workerDraft`. The
 *    parent's `onComplete` writes the draft + navigates.
 *  - Backend faster than timer → timer plays out, then `onComplete`.
 *  - Backend slower than timer → hold at "Almost done…" with spinner.
 *  - 8s mark → muted "Still working…" hint below the checklist.
 *  - 30s mark → swap to retry panel. Up to 3 retries reset the
 *    stage clock + re-fire the mutation. After 3 retries surface the
 *    skip-to-template fallback (same UI shape as Step 2).
 */
export interface Step3DraftingProps {
  /** Step 1 problem statement. Drives the Genesis call. */
  problem: string;
  /** Step 2 answers, keyed by question id. */
  answers: Record<string, QuestionAnswer>;
  /** Fired after backend success + cosmetic timer completes. */
  onComplete: (draft: WorkerDraft) => void;
  /** Fired when the user picks a template after 3 retries fail. */
  onSkipToTemplate: (example: ExampleRole) => void;
}

const STAGE_INTERVAL_MS = 700;
const ALMOST_DONE_STAGE = DRAFTING_STAGES.length - 1; // 4
const STILL_WORKING_HINT_MS = 8_000;
const RETRY_TIMEOUT_MS = 30_000;
const MAX_RETRIES = 3;

export function Step3Drafting({
  problem,
  answers,
  onComplete,
  onSkipToTemplate,
}: Step3DraftingProps) {
  const mutation = useGenesisWorkerDraft();
  const [currentStage, setCurrentStage] = useState(0);
  const [retryCount, setRetryCount] = useState(0);
  const [eightSecondHint, setEightSecondHint] = useState(false);
  const [timeoutReached, setTimeoutReached] = useState(false);
  // Set by the retry panel's "Pick a template instead" button. The
  // derived `showSkipToTemplate` flag below also flips after 3 retries
  // exhaust naturally; this is the user-opted-out branch.
  const [skipForced, setSkipForced] = useState(false);
  // Locks the snapshot of `onComplete` so its identity churn can't
  // re-fire the completion effect after the parent navigates away.
  const onCompleteRef = useRef(onComplete);
  useEffect(() => {
    onCompleteRef.current = onComplete;
  }, [onComplete]);

  const examples = useMemo(() => EXAMPLE_ROLES, []);

  // Track previous values so cleanup-on-unmount doesn't act on a
  // stale closure (would cancel a brand-new mutation after a retry).
  const startRef = useRef(mutation.start);
  useEffect(() => {
    startRef.current = mutation.start;
  }, [mutation.start]);
  const resetRef = useRef(mutation.reset);
  useEffect(() => {
    resetRef.current = mutation.reset;
  }, [mutation.reset]);

  // Fire the mutation on mount + every retry. Stage clock + timers
  // are re-armed in the same effect so a retry is a clean reset.
  useEffect(() => {
    setCurrentStage(0);
    setEightSecondHint(false);
    setTimeoutReached(false);
    startRef.current({ problem, answers });

    const stageTimer = setInterval(() => {
      setCurrentStage((s) => (s < ALMOST_DONE_STAGE ? s + 1 : s));
    }, STAGE_INTERVAL_MS);
    const hintTimer = setTimeout(() => setEightSecondHint(true), STILL_WORKING_HINT_MS);
    const retryTimer = setTimeout(() => setTimeoutReached(true), RETRY_TIMEOUT_MS);

    return () => {
      clearInterval(stageTimer);
      clearTimeout(hintTimer);
      clearTimeout(retryTimer);
    };
    // `retryCount` change re-arms the timers; `problem`/`answers`
    // shouldn't realistically change while this view is mounted.
  }, [retryCount, problem, answers]);

  // On unmount, cancel the in-flight mutation so a slow server
  // response can't write to a draft the user has navigated away from.
  useEffect(() => {
    return () => {
      resetRef.current();
    };
  }, []);

  // Completion: both the cosmetic timer has shown "Almost done…" AND
  // the backend has returned. Latch via `firedRef` so the effect
  // can't double-fire if React replays it.
  const firedRef = useRef(false);
  useEffect(() => {
    if (firedRef.current) return;
    if (currentStage < ALMOST_DONE_STAGE) return;
    if (!mutation.isSuccess || !mutation.data) return;
    firedRef.current = true;
    onCompleteRef.current(mutation.data.draft);
  }, [currentStage, mutation.isSuccess, mutation.data]);

  const handleRetry = useCallback(() => {
    if (retryCount >= MAX_RETRIES) return;
    setRetryCount((c) => c + 1);
  }, [retryCount]);

  const showSkipToTemplate =
    skipForced || (retryCount >= MAX_RETRIES && timeoutReached);
  const showRetryPanel = !showSkipToTemplate && timeoutReached;

  return (
    <section
      data-testid="hire-step3-drafting"
      className="mx-auto max-w-2xl space-y-6"
    >
      <header className="space-y-2 text-center">
        <h1 className="text-3xl font-semibold tracking-tight text-[var(--foreground)]">
          Drafting your new teammate
        </h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          Just a moment — we&apos;re putting it all together.
        </p>
      </header>

      {showSkipToTemplate ? (
        <SkipToTemplatePanel examples={examples} onPick={onSkipToTemplate} />
      ) : showRetryPanel ? (
        <RetryPanel onRetry={handleRetry} onPickTemplate={() => setSkipForced(true)} />
      ) : (
        <div className="space-y-3">
          <DraftingProgressList currentStage={currentStage} />
          {currentStage >= ALMOST_DONE_STAGE && !mutation.isSuccess ? (
            <p
              data-testid="hire-step3-almost-done-spinner"
              className="flex items-center gap-2 text-xs text-[var(--muted-foreground)]"
            >
              <Loader2 className="size-3 animate-spin" aria-hidden />
              <span>Still tightening things up…</span>
            </p>
          ) : null}
          {eightSecondHint && !mutation.isSuccess ? (
            <p
              data-testid="hire-step3-still-working"
              role="status"
              className="text-xs text-[var(--muted-foreground)]"
            >
              Still working — usually only takes a few seconds.
            </p>
          ) : null}
        </div>
      )}
    </section>
  );
}

interface RetryPanelProps {
  onRetry: () => void;
  onPickTemplate: () => void;
}

function RetryPanel({ onRetry, onPickTemplate }: RetryPanelProps) {
  return (
    <Card data-testid="hire-step3-retry-panel">
      <CardHeader>
        <CardTitle>This is taking longer than expected.</CardTitle>
      </CardHeader>
      <CardBody>
        <div className="flex flex-wrap gap-2">
          <Button
            type="button"
            variant="primary"
            data-testid="hire-step3-retry"
            onClick={onRetry}
          >
            Try again
          </Button>
          <Button
            type="button"
            variant="secondary"
            data-testid="hire-step3-pick-template"
            onClick={onPickTemplate}
          >
            Pick a template instead
          </Button>
        </div>
      </CardBody>
    </Card>
  );
}

interface SkipToTemplatePanelProps {
  examples: readonly ExampleRole[];
  onPick: (example: ExampleRole) => void;
}

function SkipToTemplatePanel({ examples, onPick }: SkipToTemplatePanelProps) {
  return (
    <Card data-testid="hire-step3-skip-to-template">
      <CardHeader>
        <CardTitle>
          Couldn&apos;t draft this one automatically. Want to start from a
          template?
        </CardTitle>
      </CardHeader>
      <CardBody>
        <div className="grid gap-2 sm:grid-cols-2">
          {examples.map((example) => (
            <Button
              key={example.title}
              type="button"
              variant="secondary"
              className="h-auto justify-start whitespace-normal px-3 py-2 text-left"
              data-testid={`hire-step3-template-${slugify(example.title)}`}
              onClick={() => onPick(example)}
            >
              <span className="block w-full">
                <span className="block text-sm font-medium">{example.title}</span>
                <span className="mt-1 block text-xs text-[var(--muted-foreground)]">
                  {example.description}
                </span>
              </span>
            </Button>
          ))}
        </div>
      </CardBody>
    </Card>
  );
}

function slugify(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
}
