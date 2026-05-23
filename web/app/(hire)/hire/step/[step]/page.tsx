"use client";

import { notFound, useRouter } from "next/navigation";
import { use, useCallback, useEffect, useState } from "react";
import { WizardEditorialShell, type WizardStep } from "@/components/auth/WizardEditorialShell";
import { WizardSkeleton } from "@/components/auth/WizardSkeleton";
import { WizardErrorState } from "@/components/auth/WizardErrorState";
import { Step2Listen } from "@/components/skycot/hire/Step2Listen";
import { Step3Drafting } from "@/components/skycot/hire/Step3Drafting";
import { Step4Introduce } from "@/components/skycot/hire/Step4Introduce";
import { Step5Connect } from "@/components/skycot/hire/Step5Connect";
import type { WorkerDraft } from "@/lib/hire/draft-types";
import type { ExampleRole } from "@/lib/hire/examples";
import type { QuestionAnswer } from "@/lib/hire/genesis-types";
import { useHireDraft } from "@/lib/hire/use-hire-draft";

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

interface StepCopy {
  label: string;
  eyebrow: string;
  hero: { text: string; accent?: boolean }[];
  sub?: React.ReactNode;
}

const STEP_COPY: Record<Step, StepCopy> = {
  2: {
    label: "LISTEN",
    eyebrow: "TELL US MORE",
    hero: [{ text: "A few" }, { text: " " }, { text: "questions.", accent: true }],
    sub: (
      <>
        We&apos;ll use these to shape your teammate&apos;s charter — what they own and
        how they decide.
      </>
    ),
  },
  3: {
    label: "DRAFTING",
    eyebrow: "WORKING ON IT",
    hero: [{ text: "Crafting your" }, { text: " " }, { text: "teammate.", accent: true }],
    sub: (
      <>
        Drafting the charter, picking schedules, and writing a few example replies.{" "}
        <strong>This takes about ten seconds.</strong>
      </>
    ),
  },
  4: {
    label: "INTRODUCE",
    eyebrow: "MEET THE TEAM",
    hero: [
      { text: "Meet your" },
      { text: " " },
      { text: "first teammate.", accent: true },
    ],
    sub: (
      <>
        Read their charter, scan a few example replies, then{" "}
        <strong>hire them</strong> — or adjust anything that&apos;s not quite right.
      </>
    ),
  },
  5: {
    label: "CONNECT",
    eyebrow: "ONE LAST STEP",
    hero: [{ text: "Connect a" }, { text: " " }, { text: "channel.", accent: true }],
    sub: (
      <>
        Pick where you&apos;ll hand them work — email, Slack, or a shared inbox.{" "}
        <strong>You can always add more later.</strong>
      </>
    ),
  },
};

/**
 * Dynamic `/hire/step/[step]` page. Validates the step segment is
 * 2..5; invalid values trigger Next's 404 boundary. Step 1 lives at
 * `/hire`, Step 6 lives at `/hire/done/[workerId]`. Visuals are the
 * locked F1 editorial-center (2026-05-19) hosted in
 * `WizardEditorialShell`; per-step Audiowide hero, eyebrow, and sub
 * copy are driven by `STEP_COPY`. Onboarding logic (provider key,
 * worker spawn, hire commit) is unchanged.
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

  // Step 4 invariant: requires a `workerDraft` from Step 3. If
  // missing, bounce to Step 3 so drafting runs (which in turn
  // requires Step 2 answers — the redirect chain re-seeds the flow).
  useEffect(() => {
    if (draft.isLoading) return;
    if (step !== 4) return;
    if (!draft.state?.workerDraft) {
      router.replace("/hire/step/3");
    }
  }, [draft.isLoading, draft.state?.workerDraft, step, router]);

  // Step 5 invariant: requires a server-assigned `workerId` from
  // Step 4's hire commit. Without it, the connector / channel UI has
  // nothing to attach to, so bounce back to Step 4.
  useEffect(() => {
    if (draft.isLoading) return;
    if (step !== 5) return;
    if (!draft.state?.workerDraft?.workerId) {
      router.replace("/hire/step/4");
    }
  }, [draft.isLoading, draft.state?.workerDraft, step, router]);

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

  const handleStep4Change = useCallback(
    (workerDraft: WorkerDraft) => {
      void draft.update({ workerDraft });
    },
    [draft],
  );

  const handleStep4Commit = useCallback(
    (workerId: string) => {
      void (async () => {
        const current = draft.state?.workerDraft;
        if (!current) return;
        await draft.update({
          step: 5,
          workerDraft: { ...current, workerId },
        });
        router.push("/hire/step/5");
      })();
    },
    [draft, router],
  );

  const handleStep4PickTemplate = useCallback(
    (example: ExampleRole) => {
      void (async () => {
        await draft.update({
          step: 2,
          problem: example.description,
          answers: {},
          workerDraft: undefined,
        });
        router.push("/hire/step/2");
      })();
    },
    [draft, router],
  );

  const handleStep5Continue = useCallback(
    (workerId: string) => {
      router.push(`/hire/done/${encodeURIComponent(workerId)}`);
    },
    [router],
  );

  const handleStep5Skip = useCallback(
    (workerId: string) => {
      router.push(`/hire/done/${encodeURIComponent(workerId)}?inert=1`);
    },
    [router],
  );

  const handleStep5Back = useCallback(() => {
    router.push("/hire/step/4");
  }, [router]);

  if (step === null) {
    notFound();
  }

  const activeStep: Step = step;
  const copy = STEP_COPY[activeStep];

  if (draft.isLoading) {
    return <WizardSkeleton step={activeStep as WizardStep} />;
  }

  if (draft.error) {
    return (
      <WizardErrorState
        variant="draft-hydration"
        errorMessage={draft.error.message}
        onReset={() => {
          void draft.clear();
          router.replace("/hire");
        }}
      />
    );
  }

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
    router.push(`/hire/step/${activeStep + 1}`);
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

  // Steps 3-5 manage their own internal CTAs (drafting animation,
  // Hire/Adjust, Connect/Skip), so the shell's footer is hidden.
  const hideFooter = activeStep === 3 || activeStep === 4 || activeStep === 5;
  const continueDisabled = activeStep === 2 ? !step2Snapshot.canContinue : true;

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
  } else if (activeStep === 4 && draft.state?.workerDraft) {
    stepContent = (
      <Step4Introduce
        workerDraft={draft.state.workerDraft}
        problem={draft.state?.problem ?? ""}
        answers={readAnswersFromDraft(draft.state?.answers)}
        onChange={handleStep4Change}
        onCommit={handleStep4Commit}
        onPickTemplate={handleStep4PickTemplate}
      />
    );
  } else if (activeStep === 5 && draft.state?.workerDraft?.workerId) {
    stepContent = (
      <Step5Connect
        workerDraft={{
          ...draft.state.workerDraft,
          workerId: draft.state.workerDraft.workerId,
        }}
        onContinue={handleStep5Continue}
        onSkip={handleStep5Skip}
        onBack={handleStep5Back}
      />
    );
  } else {
    stepContent = null;
  }

  return (
    <WizardEditorialShell
      step={activeStep as WizardStep}
      stepLabel={copy.label}
      eyebrow={copy.eyebrow}
      heroSegments={copy.hero}
      sub={copy.sub}
      unsavedHire={hasBody}
      continueDisabled={continueDisabled}
      onBack={goBack}
      onContinue={hideFooter ? undefined : goNext}
      onDiscard={() => void draft.clear()}
      hideFooter={hideFooter}
    >
      {stepContent}
    </WizardEditorialShell>
  );
}
