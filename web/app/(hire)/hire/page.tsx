"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useEffect, useMemo } from "react";
import { WizardEditorialShell } from "@/components/auth/WizardEditorialShell";
import { Step1Land } from "@/components/skycot/hire/Step1Land";
import { findExampleRoleById } from "@/lib/hire/examples";
import { useHireDraft } from "@/lib/hire/use-hire-draft";
import { useStep1 } from "@/lib/hire/use-step1";

/**
 * Step 1 — Land. The wizard entry page. Real free-text input + examples
 * popover (E2-b), with optional `?example=<id>` pre-fill from the team
 * empty state (E3-e). On mount we hydrate the draft store; if the user
 * has progressed past step 1 within the 7-day TTL we route forward to
 * wherever they left off.
 *
 * Visual treatment is locked F1 editorial-center (2026-05-19): hosted
 * inside `WizardEditorialShell` with the Audiowide "Build your AI
 * workforce." hero. Step 1's existing form chrome (textarea, examples
 * popover, validation hint) lives inside the shell's body slot.
 */
export default function HireLandingPage() {
  return (
    <Suspense fallback={null}>
      <HireLandingContent />
    </Suspense>
  );
}

function HireLandingContent() {
  const router = useRouter();
  const draft = useHireDraft();
  const searchParams = useSearchParams();
  const exampleId = searchParams?.get("example") ?? null;

  const examplePrefill = useMemo(
    () => findExampleRoleById(exampleId)?.description ?? "",
    [exampleId],
  );

  useEffect(() => {
    if (draft.isLoading) return;
    const saved = draft.state;
    if (!saved) return;
    if (saved.step >= 2 && saved.step <= 5) {
      router.replace(`/hire/step/${saved.step}`);
    }
  }, [draft.isLoading, draft.state, router]);

  const initialValue = draft.state?.problem
    ? draft.state.problem
    : examplePrefill;

  const step1 = useStep1({
    initialValue,
    onSubmit: (value) => {
      void (async () => {
        await draft.update({ step: 2, problem: value });
        router.push("/hire/step/2");
      })();
    },
  });

  const hasBody = Boolean(
    step1.value.trim() ||
      draft.state?.problem ||
      (draft.state?.answers && Object.keys(draft.state.answers).length > 0),
  );

  return (
    <WizardEditorialShell
      step={1}
      stepLabel="WELCOME"
      eyebrow="WELCOME TO SKYCOT"
      heroSegments={[
        { text: "Build your" },
        { text: " " },
        { text: "AI workforce.", accent: true },
      ]}
      sub={
        <>
          In <strong>four minutes</strong>, you&apos;ll have hired your first AI teammate
          — and they&apos;ll start work as soon as you finish.
        </>
      }
      unsavedHire={hasBody}
      continueDisabled={!step1.canSubmit}
      onContinue={step1.attemptSubmit}
      onDiscard={() => void draft.clear()}
      continueLabel="Continue"
    >
      <Step1Land step1={step1} />
    </WizardEditorialShell>
  );
}
