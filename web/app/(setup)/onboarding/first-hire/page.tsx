"use client";

import { Loader2, Sparkles } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import {
  FIRST_HIRE_VERIFY_FAILED_MESSAGE,
  useFirstHire,
} from "@/lib/onboarding/use-first-hire";
import { FirstHireErrorBanner } from "@/components/skycot/onboarding/FirstHireErrorBanner";
import { scrubOnboardingErrorMessage } from "../_scrub-error";
import { OnboardingShell } from "../_OnboardingShell";

// Placeholder intentionally avoids cadence words ("weekly", "daily") —
// the wizard only POSTs {description, vertical_id} today; cadence is set
// later on the worker detail page's Schedules card. Phase 2 (Genesis
// parsing) will earn back the right to promise schedule extraction here;
// until then the copy must not write a cheque the wiring can't cash.
// See brief fix-first-hire-cta-copy-promises-weekly-schedule-not-parsed.
const EXAMPLE_PROMPT =
  "A marketing intern named Maya who drafts blog posts and asks before publishing.";

export default function OnboardingFirstHirePage() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setFirstHirePromptInStore = useOnboardingDraft((s) => s.setFirstHirePrompt);
  const reset = useOnboardingDraft((s) => s.reset);
  // Read the prompt straight from the draft store so step-back navigation
  // and refresh both restore what the user has typed. The store is the
  // single source of truth — no local mirror to drift out of sync.
  const prompt = useOnboardingDraft((s) => s.firstHirePrompt);
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
    const description = prompt.trim();
    if (description.length === 0) {
      setError("Tell us a bit about who you'd like to hire.");
      return;
    }
    const { tenantId, verticalId } = useOnboardingDraft.getState();
    if (!verticalId) {
      setError("Pick a template first.");
      return;
    }
    setFirstHirePromptInStore(description);
    try {
      const res = await firstHire.mutateAsync({
        description,
        tenantId,
        verticalId,
      });
      reset();
      // sessionStorage flag drives the one-shot "Set their schedule"
      // callout on the worker overview page; per-tab + cleared on first
      // read, so a reload doesn't re-surface it. URL stays clean so the
      // demo-mode redirect regex pin in `onboarding-demo-mode-first-hire
      // -lands-on-worker.spec.ts` still matches. See brief
      // fix-first-hire-cta-copy-promises-weekly-schedule-not-parsed.
      try {
        sessionStorage.setItem("skycot:just-hired", res.workerId);
      } catch {
        /* private mode / storage disabled — silently degrade */
      }
      // Hire-complete redirect: /workers/<id> is the canonical worker
      // profile route (per (app)/workers/[workerId]/...). Previous code
      // pushed /team/<id> which doesn't exist and 404'd new users at
      // the end of onboarding.
      router.push(`/workers/${res.workerId}`);
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
  const canSubmit = prompt.trim().length > 0 && !submitting;

  return (
    <OnboardingShell
      step={5}
      completedSteps={4}
      stepLabel="FIRST HIRE"
      eyebrow="STEP FIVE · FINAL"
      heroSegments={[
        { text: "Describe your " },
        { text: "first worker.", accent: true },
      ]}
      sub={
        <>
          Plain English is fine. Give them a <strong>name and a role</strong>.
          You can set their schedule after they're hired.
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
          <span className="ob-field-label">Tell us about them</span>
          <textarea
            className="ob-field-textarea"
            data-testid="first-hire-prompt"
            value={prompt}
            onChange={(e) => setFirstHirePromptInStore(e.target.value)}
            placeholder={EXAMPLE_PROMPT}
            aria-label="First hire description"
            rows={5}
            autoFocus
          />
          <p className="ob-field-hint">
            Example: <em>{EXAMPLE_PROMPT}</em>
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
