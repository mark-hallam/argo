"use client";

import { notFound, useRouter } from "next/navigation";
import { use, useEffect } from "react";
import { HireShell } from "@/components/skycot/HireShell";
import { useHireDraft } from "@/lib/hire/use-hire-draft";

const STEP_PLACEHOLDERS: Record<number, string> = {
  2: "Step 2 — placeholder for E2-c (Listen)",
  3: "Step 3 — placeholder for E2-d (Drafting transition)",
  4: "Step 4 — placeholder for E2-e (Introduce)",
  5: "Step 5 — placeholder for E2-g (Connect)",
};

const VALID_STEPS = new Set(Object.keys(STEP_PLACEHOLDERS).map(Number));

type Step = 2 | 3 | 4 | 5;

function parseStep(raw: string): Step | null {
  const n = Number.parseInt(raw, 10);
  if (!Number.isFinite(n)) return null;
  if (!VALID_STEPS.has(n)) return null;
  return n as Step;
}

/**
 * Dynamic `/hire/step/[step]` page. Validates the step segment is
 * 2..5; invalid values trigger Next's 404 boundary. Step 1 lives at
 * `/hire`, Step 6 lives at `/hire/done/[workerId]`.
 */
export default function HireStepPage({ params }: { params: Promise<{ step: string }> }) {
  const { step: rawStep } = use(params);
  const step = parseStep(rawStep);
  const router = useRouter();
  const draft = useHireDraft();

  useEffect(() => {
    if (step !== null) {
      void draft.update({ step });
    }
  }, [step, draft]);

  if (step === null) {
    notFound();
  }

  const activeStep: Step = step;
  const hasBody = Boolean(
    draft.state?.problem ||
      (draft.state?.answers && Object.keys(draft.state.answers).length > 0),
  );

  function goNext() {
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

  return (
    <HireShell
      step={activeStep}
      unsavedHire={hasBody}
      continueDisabled
      onBack={goBack}
      onContinue={goNext}
      onDiscard={() => void draft.clear()}
    >
      <div className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-20 px-6 text-center space-y-3">
        <h1 className="text-xl font-semibold text-[var(--foreground)]">Hire a teammate</h1>
        <p className="text-sm text-[var(--muted-foreground)]">{STEP_PLACEHOLDERS[step]}</p>
      </div>
    </HireShell>
  );
}
