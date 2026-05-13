"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Toast, ToastProvider, ToastViewport } from "@/components/ui/Toast";
import { AdjustDrawer } from "@/components/skycot/hire/AdjustDrawer";
import { ExampleReplyCard } from "@/components/skycot/hire/ExampleReplyCard";
import { WorkerDraftCard } from "@/components/skycot/hire/WorkerDraftCard";
import { Button } from "@/components/ui/Button";
import type { WorkerDraft } from "@/lib/hire/draft-types";
import type { ExampleRole } from "@/lib/hire/examples";
import type { QuestionAnswer } from "@/lib/hire/genesis-types";
import { useGenesisWorkerDraft } from "@/lib/hire/use-genesis-worker-draft";
import { useHireWorker } from "@/lib/hooks/use-hire-worker";

/**
 * Hire flow — Step 4 (Introduce).
 *
 * The load-bearing emotional moment of the flow: the user meets the
 * proposed teammate, inspects example replies, and either commits or
 * adjusts. Behavior:
 *
 *  - Renders `WorkerDraftCard` (avatar + inline-editable name /
 *    tagline / charter + toggleable duties) and 3 `ExampleReplyCard`
 *    rows below it.
 *  - "Adjust" opens the `AdjustDrawer` — Save fires a `draftWorker`
 *    re-mutation with `{ problem, answers, adjustment }`; on success
 *    the draft state updates and the drawer closes.
 *  - 3+ Adjusts surface the template fallback inside the drawer.
 *  - "Hire Mariah" primary button POSTs the hire commit, optimistically
 *    advances to Step 5, and rolls back to Step 4 (with toast) on
 *    error.
 *
 *  The parent (page.tsx) owns the persisted draft + routing. This
 *  component receives the current `workerDraft` + `problem` / `answers`
 *  and emits `onChange` / `onCommit` / `onTemplate` callbacks.
 */
export interface Step4IntroduceProps {
  workerDraft: WorkerDraft;
  problem: string;
  answers: Record<string, QuestionAnswer>;
  onChange: (next: WorkerDraft) => void;
  /** Fires once the hire commit succeeds; parent persists workerId + routes. */
  onCommit: (workerId: string) => void;
  /** Fires when the user picks a template tile from the Adjust drawer. */
  onPickTemplate: (example: ExampleRole) => void;
}

export function Step4Introduce({
  workerDraft,
  problem,
  answers,
  onChange,
  onCommit,
  onPickTemplate,
}: Step4IntroduceProps) {
  const [adjustOpen, setAdjustOpen] = useState(false);
  const [adjustCount, setAdjustCount] = useState(0);
  const [errorToast, setErrorToast] = useState<{
    id: number;
    name: string;
    message: string;
  } | null>(null);

  const redraft = useGenesisWorkerDraft();
  const hire = useHireWorker();

  // Latch refs for callbacks fired by mutations after unmount or
  // identity churn.
  const onChangeRef = useRef(onChange);
  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);
  const onCommitRef = useRef(onCommit);
  useEffect(() => {
    onCommitRef.current = onCommit;
  }, [onCommit]);

  // When the redraft mutation succeeds, write the new draft + close
  // the drawer + bump the adjust counter. We watch the mutation's
  // success state rather than a callback so the resolution path is
  // consistent regardless of re-render timing.
  const lastRedraftDataRef = useRef<unknown>(null);
  useEffect(() => {
    if (!redraft.isSuccess || !redraft.data) return;
    if (lastRedraftDataRef.current === redraft.data) return;
    lastRedraftDataRef.current = redraft.data;
    onChangeRef.current(redraft.data.draft);
    setAdjustOpen(false);
    setAdjustCount((c) => c + 1);
  }, [redraft.isSuccess, redraft.data]);

  // Hire commit success → bubble workerId to parent.
  const lastHireDataRef = useRef<unknown>(null);
  useEffect(() => {
    if (!hire.isSuccess || !hire.data) return;
    if (lastHireDataRef.current === hire.data) return;
    lastHireDataRef.current = hire.data;
    onCommitRef.current(hire.data.workerId);
  }, [hire.isSuccess, hire.data]);

  // Hire commit error → surface toast, stay on Step 4.
  useEffect(() => {
    if (!hire.isError || !hire.error) return;
    setErrorToast({
      id: Date.now(),
      name: workerDraft.suggestedName,
      message: hire.error.message,
    });
  }, [hire.isError, hire.error, workerDraft.suggestedName]);

  const handleSaveAdjust = useCallback(
    (adjustment: string) => {
      redraft.start({ problem, answers, adjustment });
    },
    [redraft, problem, answers],
  );

  const handleHire = useCallback(() => {
    hire.commit({ workerDraft, problem, answers });
  }, [hire, workerDraft, problem, answers]);

  const handleRetry = useCallback(() => {
    setErrorToast(null);
    hire.reset();
    hire.commit({ workerDraft, problem, answers });
  }, [hire, workerDraft, problem, answers]);

  const hireDisabled = adjustOpen || redraft.isPending || hire.isPending;

  return (
    <ToastProvider swipeDirection="right" duration={6_000}>
      <section
        data-testid="hire-step4-introduce"
        className="mx-auto max-w-2xl space-y-6"
      >
        <header className="space-y-2">
          <h1 className="text-2xl font-semibold tracking-tight text-[var(--foreground)]">
            Meet your new teammate.
          </h1>
          <p className="text-sm text-[var(--muted-foreground)]">
            Take a moment — adjust anything that doesn&apos;t feel right, then
            hire them when you&apos;re ready.
          </p>
        </header>

        <WorkerDraftCard
          draft={workerDraft}
          onChange={onChange}
          onAdjust={() => setAdjustOpen(true)}
          adjustDisabled={redraft.isPending || hire.isPending}
        />

        {workerDraft.exampleReplies.length > 0 ? (
          <section
            data-testid="hire-step4-examples"
            className="space-y-3"
            aria-label="Example replies"
          >
            <h2 className="text-xs font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">
              Example replies
            </h2>
            <div className="space-y-3">
              {workerDraft.exampleReplies.slice(0, 3).map((example, idx) => (
                <ExampleReplyCard
                  key={`${example.inboundContext}-${idx}`}
                  example={example}
                  workerName={workerDraft.suggestedName}
                  index={idx}
                />
              ))}
            </div>
          </section>
        ) : null}

        <div className="flex justify-end pt-2">
          <Button
            type="button"
            variant="primary"
            size="lg"
            data-testid="hire-step4-hire"
            onClick={handleHire}
            loading={hire.isPending}
            disabled={hireDisabled}
          >
            Hire {workerDraft.suggestedName}
          </Button>
        </div>
      </section>

      <AdjustDrawer
        open={adjustOpen}
        onOpenChange={setAdjustOpen}
        adjustCount={adjustCount}
        saving={redraft.isPending}
        onSave={handleSaveAdjust}
        onPickTemplate={onPickTemplate}
      />

      {errorToast ? (
        <Toast
          key={errorToast.id}
          tone="error"
          data-testid="hire-step4-error-toast"
          title={`Couldn't hire ${errorToast.name}`}
          description={errorToast.message}
          action={
            // Radix ToastAction is itself a <button>, so we pass a
            // styled <span> here and let the outer action element take
            // the click. Avoids the button-in-button hydration warning.
            <span
              role="button"
              tabIndex={0}
              data-testid="hire-step4-error-retry"
              onClick={handleRetry}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  handleRetry();
                }
              }}
              className="inline-flex h-8 cursor-pointer items-center rounded-[var(--radius-sm)] bg-[var(--secondary)] px-3 text-sm font-medium text-[var(--secondary-foreground)] hover:bg-[var(--accent)]"
            >
              Try again
            </span>
          }
          onOpenChange={(open) => {
            if (!open) setErrorToast(null);
          }}
        />
      ) : null}
      <ToastViewport />
    </ToastProvider>
  );
}
