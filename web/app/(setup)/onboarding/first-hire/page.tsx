"use client";

import { Loader2, Sparkles } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useMemo, useState } from "react";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import {
  FIRST_HIRE_VERIFY_FAILED_MESSAGE,
  useFirstHire,
} from "@/lib/onboarding/use-first-hire";
import { exampleForVertical } from "@/lib/onboarding/example-prompts";
import { isDemoMode } from "@/lib/skycot/demo-mode";
import { FirstHireErrorBanner } from "@/components/skycot/onboarding/FirstHireErrorBanner";
import { WizardSkeleton } from "@/components/auth/WizardSkeleton";
import { scrubOnboardingErrorMessage } from "../_scrub-error";
import { OnboardingShell } from "../_OnboardingShell";

const NAME_PLACEHOLDER_BY_VERTICAL: Record<string, string> = {
  "marketing-intern": "Maya",
  "gtm-engineer": "Marcus",
  "sales-sdr": "Priya",
  "customer-support": "Jamal",
  "finance-ar": "Nori",
};

export default function OnboardingFirstHirePage() {
  // Gate on persist rehydration — both name and prompt come from the
  // draft store via selectors, so without the gate the textarea would
  // mount with the empty `firstHirePrompt` default before persist
  // resolves and re-renders with the saved value (F8 flash).
  const hasHydrated = useOnboardingDraft((s) => s.hasHydrated);
  if (!hasHydrated) {
    return <WizardSkeleton step={5} />;
  }
  return <FirstHireFormBody />;
}

function FirstHireFormBody() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setFirstHireNameInStore = useOnboardingDraft((s) => s.setFirstHireName);
  const setFirstHirePromptInStore = useOnboardingDraft((s) => s.setFirstHirePrompt);
  const reset = useOnboardingDraft((s) => s.reset);
  // Read both inputs straight from the draft store so step-back
  // navigation and refresh both restore what the user has typed. The
  // store is the single source of truth — no local mirror to drift out
  // of sync.
  const name = useOnboardingDraft((s) => s.firstHireName);
  const prompt = useOnboardingDraft((s) => s.firstHirePrompt);
  // Step 4 stores the picked verticalId in the draft store; we resolve
  // it to a placeholder so the example matches what the operator just
  // chose. Falls back to the marketing-intern copy when the operator
  // landed here directly (deep link / wizard skip) without picking a
  // template. Keeps Genesis's archetype dispatch aligned with the
  // template the operator selected rather than the placeholder text.
  const pickedVerticalId = useOnboardingDraft((s) => s.verticalId);
  const examplePrompt = exampleForVertical(pickedVerticalId);
  const namePlaceholder = useMemo(
    () =>
      (pickedVerticalId && NAME_PLACEHOLDER_BY_VERTICAL[pickedVerticalId]) ||
      "Maya",
    [pickedVerticalId],
  );
  const firstHire = useFirstHire();
  const [error, setError] = useState("");
  const [showHelpLink, setShowHelpLink] = useState(false);

  useEffect(() => {
    setStep(5);
  }, [setStep]);

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setError("");
    setShowHelpLink(false);
    const trimmedName = name.trim();
    const description = prompt.trim();
    if (trimmedName.length === 0) {
      setError("Give them a name.");
      return;
    }
    if (description.length === 0) {
      setError("Tell us a bit about who you'd like to hire.");
      return;
    }
    const { tenantId, verticalId } = useOnboardingDraft.getState();
    if (!verticalId) {
      setError("Pick a template first.");
      return;
    }
    setFirstHireNameInStore(trimmedName);
    setFirstHirePromptInStore(description);
    try {
      const res = await firstHire.mutateAsync({
        name: trimmedName,
        description,
        tenantId,
        verticalId,
      });
      reset();
      // sessionStorage flag drives the one-shot "Set their schedule"
      // callout on the worker overview page; per-tab + cleared on first
      // read, so a reload doesn't re-surface it. Carries through both
      // the demo-mode roster redirect and the /onboarding/done
      // celebration's auto-advance.
      try {
        sessionStorage.setItem("skycot:just-hired", res.workerId);
      } catch {
        /* private mode / storage disabled — silently degrade */
      }
      // Hire-complete redirect:
      // - In demo mode: land on /workers?just-hired=1&new=<id> so the
      //   camera sees the roster grow (new card highlighted) — the
      //   recording's own emotional beat. The /onboarding/done
      //   celebration would land between the wizard and the roster and
      //   blunt that punchline. URL pin in
      //   `onboarding-demo-mode-first-hire-lands-on-worker.spec.ts`
      //   still matches. See brief
      //   `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker`.
      // - In non-demo mode: route through `/onboarding/done` so the
      //   F1 "meet your first teammate" celebration lands before the
      //   buyer reaches the worker profile. The done page reads
      //   `?worker=<id>&name=<workerName>` from the URL (the draft is
      //   reset above so the URL is the only carrier) and auto-advances
      //   to `/workers/<id>` after 6s.
      if (isDemoMode()) {
        router.push(`/workers?just-hired=1&new=${encodeURIComponent(res.workerId)}`);
      } else {
        const nameParam = encodeURIComponent(res.workerName);
        router.push(
          `/onboarding/done?worker=${encodeURIComponent(res.workerId)}&name=${nameParam}`,
        );
      }
    } catch (e) {
      // Wave-11 fix (2026-05-21): prior copy claimed "we'll create
      // your worker in the background — try again or skip to your
      // team" but there's no background job and no Skip button. Be
      // honest: the call failed; user retries. Real success path
      // ships via web-gateway-endpoints-onboarding-batch.
      if (e instanceof Error && e.message === FIRST_HIRE_VERIFY_FAILED_MESSAGE) {
        setError(e.message);
        setShowHelpLink(true);
      } else {
        // Scrub internal protocol detail before it reaches the form.
        const scrubbed = scrubOnboardingErrorMessage(e);
        const suffix = scrubbed.wasProtocolLeak
          ? ""
          : " Try again, or come back once the gateway is up.";
        setError(`${scrubbed.message}${suffix}`);
      }
    }
  }

  const submitting = firstHire.isPending;
  const canSubmit =
    name.trim().length > 0 && prompt.trim().length > 0 && !submitting;

  return (
    <OnboardingShell
      step={5}
      completedSteps={4}
      stepLabel="FIRST HIRE"
      eyebrow="STEP 05 · YOUR TEAMMATE"
      heroSegments={[
        { text: "Describe your " },
        { text: "first teammate.", accent: true },
      ]}
      sub={
        <>
          Their <strong>role</strong> and{" "}
          <strong>what they&apos;ll do</strong> — plain English. You can set
          their schedule after they&apos;re hired.
        </>
      }
    >
      <form
        className="ob-form-card"
        onSubmit={onSubmit}
        data-testid="onboarding-first-hire-form"
        noValidate
        style={{ maxWidth: 640 }}
      >
        <label className="ob-field">
          <span className="ob-field-label">Their name</span>
          <input
            className="ob-field-input"
            data-testid="first-hire-name"
            type="text"
            value={name}
            onChange={(e) => setFirstHireNameInStore(e.target.value)}
            placeholder={namePlaceholder}
            aria-label="First hire name"
            autoComplete="off"
            autoFocus
            required
          />
        </label>
        <label className="ob-field">
          <span className="ob-field-label">Tell us about them</span>
          <textarea
            className="ob-field-textarea"
            data-testid="first-hire-prompt"
            value={prompt}
            onChange={(e) => setFirstHirePromptInStore(e.target.value)}
            placeholder={examplePrompt}
            aria-label="First hire description"
            rows={5}
          />
          <p className="ob-field-hint">
            Example: <em>{examplePrompt}</em>
          </p>
        </label>
        {error ? (
          <FirstHireErrorBanner message={error} showHelpLink={showHelpLink} />
        ) : null}
        <div className="ob-cta-row is-split">
          <button
            type="button"
            className="ob-cta-back"
            onClick={() => router.push("/onboarding/pick-vertical")}
            data-testid="onboarding-first-hire-back"
          >
            ← Back
          </button>
          <button
            type="submit"
            className="ob-cta"
            disabled={!canSubmit}
            aria-busy={submitting || undefined}
            data-testid="onboarding-first-hire-submit"
          >
            {submitting ? (
              <Loader2 className="size-4 animate-spin" aria-hidden />
            ) : (
              <Sparkles className="size-4" aria-hidden />
            )}
            Hire them
            <span className="ob-cta-kbd" aria-hidden>
              ↵
            </span>
          </button>
        </div>
      </form>
    </OnboardingShell>
  );
}
