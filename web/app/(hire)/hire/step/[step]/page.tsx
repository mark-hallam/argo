"use client";

import { notFound, useRouter } from "next/navigation";
import { use, useCallback, useEffect, useState } from "react";
import { HireShell } from "@/components/skycot/HireShell";
import { Step2Listen } from "@/components/skycot/hire/Step2Listen";
import { Step3Drafting } from "@/components/skycot/hire/Step3Drafting";
import type { WorkerDraft } from "@/lib/hire/draft-types";
import type { ExampleRole } from "@/lib/hire/examples";
import type { QuestionAnswer } from "@/lib/hire/genesis-types";
import { useHireDraft } from "@/lib/hire/use-hire-draft";

const STEP_PLACEHOLDERS: Record<number, string> = {
  4: "Step 4 — placeholder for E2-e (Introduce)",
  5: "Step 5 — placeholder for E2-g (Connect)",
};

const VALID_STEPS = new Set<number>([2, 3, 4, 5]);

type Step = 2 | 3 | 4 | 5;

function parseStep(raw: string): Step | null {
  const n = Number.parseInt(raw, 10);
  if (!Number.isFinite(n)) return null;
  if (!VALID_STEPS.has(n)) return null;
  return n as Step;
}

function readAnswersFromDraft(
  raw: Record<string, unknown> | undefined,
): Record<string, QuestionAnswer> {
  if (!raw) return {};
  const out: Record<string, QuestionAnswer> = {};
  for (const [id, value] of Object.entries(raw)) {
    if (value && typeof value === "object" && "type" in value) {
      out[id] = value as QuestionAnswer;
    }
  }
  return out;
}

/**
 * Dynamic `/hire/step/[step]` page. Validates the step segment is
 * 2..5; invalid values trigger Next's 404 boundary. Step 1 lives at
 * `/hire`, Step 6 lives at `/hire/done/[workerId]`. Step 2 (Listen)
 * renders the real Genesis-lite flow (E2-c); Step 3 (Drafting)
 * renders the worker-draft transition (E2-d); 4-5 stay placeholders
 * until their respective briefs land.
 */
export default function HireStepPage({ params }: { params: Promise<{ step: string }> }) {
  const { step: rawStep } = use(params);
  const step = parseStep(rawStep);
  const router = useRouter();
  const draft = useHireDraft();
  const [step2Snapshot, setStep2Snapshot] = useState<{
    answers: Record<string, QuestionAnswer>;
    canContinue: boolean;
  }>({ answers: {}, canContinue: false });

  useEffect(() => {
    if (step !== null) {
      void draft.update({ step });
    }
  }, [step, draft]);

  // Step 2 invariant: requires a `problem` from Step 1. If the user
  // landed here directly (no problem in draft), bounce to Step 1.
  useEffect(() => {
    if (draft.isLoading) return;
    if (step !== 2) return;
    if (!draft.state?.problem) {
      router.replace("/hire");
    }
  }, [draft.isLoading, draft.state?.problem, step, router]);

  // Step 3 invariant: requires `answers` from Step 2. If empty,
  // bounce to Step 2 so the user re-answers (which also recreates
  // the `problem` precondition by extension).
  useEffect(() => {
    if (draft.isLoading) return;
    if (step !== 3) return;
    const answerCount = draft.state?.answers ? Object.keys(draft.state.answers).length : 0;
    if (answerCount === 0) {
      router.replace("/hire/step/2");
    }
  }, [draft.isLoading, draft.state?.answers, step, router]);

  const handleStep2Change = useCallback(
    (snap: { answers: Record<string, QuestionAnswer>; canContinue: boolean }) => {
      setStep2Snapshot({ answers: snap.answers, canContinue: snap.canContinue });
    },
    [],
  );

  const handleStep2SkipToTemplate = useCallback(
    (example: ExampleRole) => {
      void (async () => {
        await draft.update({ step: 2, problem: example.description, answers: {} });
        setStep2Snapshot({ answers: {}, canContinue: false });
      })();
    },
    [draft],
  );

  const handleStep3Complete = useCallback(
    (workerDraft: WorkerDraft) => {
      void (async () => {
        await draft.update({ step: 4, workerDraft });
        router.push("/hire/step/4");
      })();
    },
    [draft, router],
  );

  const handleStep3SkipToTemplate = useCallback(
    (example: ExampleRole) => {
      void (async () => {
        await draft.update({ step: 2, problem: example.description, answers: {} });
        router.push("/hire/step/2");
      })();
    },
    [draft, router],
  );

  if (step === null) {
    notFound();
  }

  const activeStep: Step = step;

  function goNext() {
    if (activeStep === 2) {
      void (async () => {
        const merged = {
          ...(draft.state?.answers ?? {}),
          ...step2Snapshot.answers,
        };
        await draft.update({ step: 3, answers: merged });
        router.push("/hire/step/3");
      })();
      return;
    }
    if (activeStep === 5) {
      router.push("/hire/done/stub");
    } else {
      router.push(`/hire/step/${activeStep + 1}`);
    }
  }

  function goBack() {
    if (activeStep === 2) {
      router.push("/hire");
    } else {
      router.push(`/hire/step/${activeStep - 1}`);
    }
  }

  const hasBody = Boolean(
    draft.state?.problem ||
      (draft.state?.answers && Object.keys(draft.state.answers).length > 0),
  );

  const continueDisabled = activeStep === 2 ? !step2Snapshot.canContinue : true;
  const hideFooter = activeStep === 3;

  let stepContent: React.ReactNode;
  if (activeStep === 2) {
    stepContent = (
      <Step2Listen
        problem={draft.state?.problem ?? ""}
        initialAnswers={readAnswersFromDraft(draft.state?.answers)}
        onChange={handleStep2Change}
        onSkipToTemplate={handleStep2SkipToTemplate}
      />
    );
  } else if (activeStep === 3) {
    stepContent = (
      <Step3Drafting
        problem={draft.state?.problem ?? ""}
        answers={readAnswersFromDraft(draft.state?.answers)}
        onComplete={handleStep3Complete}
        onSkipToTemplate={handleStep3SkipToTemplate}
      />
    );
  } else {
    stepContent = (
      <div className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-20 px-6 text-center space-y-3">
        <h1 className="text-xl font-semibold text-[var(--foreground)]">Hire a teammate</h1>
        <p className="text-sm text-[var(--muted-foreground)]">{STEP_PLACEHOLDERS[step]}</p>
      </div>
    );
  }

  return (
    <HireShell
      step={activeStep}
      unsavedHire={hasBody}
      continueDisabled={continueDisabled}
      onBack={goBack}
      onContinue={goNext}
      onDiscard={() => void draft.clear()}
      hideFooter={hideFooter}
    >
      {stepContent}
    </HireShell>
  );
}
