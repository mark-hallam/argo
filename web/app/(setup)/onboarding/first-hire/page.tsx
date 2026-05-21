"use client";

import { Loader2, Sparkles } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { useFirstHire } from "@/lib/onboarding/use-first-hire";
import { OnboardingShell } from "../_OnboardingShell";

const EXAMPLE_PROMPT =
  "A marketing intern named Maya who drafts blog posts and runs weekly competitive scans.";

export default function OnboardingFirstHirePage() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setFirstHirePromptInStore = useOnboardingDraft((s) => s.setFirstHirePrompt);
  const reset = useOnboardingDraft((s) => s.reset);
  const initialPrompt = useOnboardingDraft((s) => s.firstHirePrompt);
  const firstHire = useFirstHire();
  const [prompt, setPrompt] = useState(initialPrompt);
  const [error, setError] = useState("");

  useEffect(() => {
    setStep(5);
  }, [setStep]);

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setError("");
    const description = prompt.trim();
    if (description.length === 0) {
      setError("Tell us a bit about who you'd like to hire.");
      return;
    }
    const verticalId = useOnboardingDraft.getState().verticalId;
    if (!verticalId) {
      setError("Pick a template first.");
      return;
    }
    setFirstHirePromptInStore(description);
    try {
      const res = await firstHire.mutateAsync({
        description,
        verticalId,
      });
      reset();
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
      setError(
        e instanceof Error
          ? `${e.message}. Try again, or come back once the gateway is up.`
          : "Couldn't reach the gateway. Try again in a moment.",
      );
    }
  }

  const submitting = firstHire.isPending;
  const canSubmit = prompt.trim().length > 0 && !submitting;

  return (
    <OnboardingShell
      step={5}
      stepLabel="FIRST HIRE"
      eyebrow="STEP FIVE · FINAL"
      heroSegments={[
        { text: "Describe your " },
        { text: "first worker.", accent: true },
      ]}
      sub={
        <>
          Plain English is fine. Give them a{" "}
          <strong>name, a role, and what they should do this week</strong>.
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
            onChange={(e) => setPrompt(e.target.value)}
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
          <p className="ob-form-error" data-testid="onboarding-first-hire-error">
            {error}
          </p>
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
