"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useEffect, useMemo } from "react";
import { HireShell } from "@/components/skycot/HireShell";
import { Step1Land } from "@/components/skycot/hire/Step1Land";
import { findExampleRoleById } from "@/lib/hire/examples";
import { useHireDraft } from "@/lib/hire/use-hire-draft";
import { useStep1 } from "@/lib/hire/use-step1";

/**
 * Step 1 — Land. Real free-text input + examples popover (E2-b),
 * with optional `?example=<id>` pre-fill from the team empty state
 * (E3-e). When the param is present and the saved draft has no
 * problem text yet, the matching `EXAMPLE_ROLES` description seeds
 * the textarea so Continue activates immediately. Editing the seeded
 * text remains unrestricted.
 *
 * On mount we hydrate the draft store; if the user has progressed past
 * step 1 in a previous session and the draft is still within its 7-day
 * TTL, we route forward to wherever they left off so they don't
 * re-do work. Otherwise we render Step1Land seeded from any existing
 * `draft.state.problem` so the user's prior input survives a reload.
 *
 * Submit (Continue or Cmd/Ctrl+Enter) writes
 * `{ step: 2, problem: <value> }` to the draft store and routes to
 * `/hire/step/2`. No server call yet — Genesis-lite lives in E2-c.
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
    <HireShell
      step={1}
      unsavedHire={hasBody}
      continueDisabled={!step1.canSubmit}
      onContinue={step1.attemptSubmit}
      onDiscard={() => void draft.clear()}
    >
      <Step1Land step1={step1} />
    </HireShell>
  );
}
