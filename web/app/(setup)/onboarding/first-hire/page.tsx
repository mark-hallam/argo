"use client";

import { Sparkles } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
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
      eyebrow="STEP FIVE"
      heroSegments={[
        { text: "Describe your " },
        { text: "first worker.", accent: true },
      ]}
      sub={
        <>
          Plain English is fine. Give them a <strong>name, a role, and what they should do this week</strong>.
        </>
      }
    >
      <section className="mx-auto flex max-w-2xl flex-col gap-6">
        <form
          className="flex flex-col gap-4 rounded-[12px] border border-[#ece9f5] bg-white/80 p-6 shadow-sm"
          onSubmit={onSubmit}
          data-testid="onboarding-first-hire-form"
        >
          <label className="block space-y-1.5">
            <span className="text-sm font-medium text-[#1e1b4b]">Tell us about them</span>
            <textarea
              className="min-h-32 w-full rounded-[8px] border border-[#d1d1d1] bg-white p-3 text-sm text-[#1e1b4b] outline-none focus:border-[#8b5cf6] focus:ring-2 focus:ring-[#8b5cf6]/30"
              data-testid="first-hire-prompt"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder={EXAMPLE_PROMPT}
              aria-label="First hire description"
            />
            <p className="text-xs text-[#52525b]">
              Example: <em>{EXAMPLE_PROMPT}</em>
            </p>
          </label>
          {error ? (
            <p
              className="text-sm text-[#dc2626]"
              data-testid="onboarding-first-hire-error"
            >
              {error}
            </p>
          ) : null}
          <div className="flex items-center justify-between pt-2">
            <Button
              type="button"
              variant="ghost"
              onClick={() => router.push("/onboarding/pick-vertical")}
              data-testid="onboarding-first-hire-back"
            >
              Back
            </Button>
            <Button
              type="submit"
              disabled={!canSubmit}
              loading={submitting}
              data-testid="onboarding-first-hire-submit"
            >
              <Sparkles aria-hidden />
              Hire them
            </Button>
          </div>
        </form>
      </section>
    </OnboardingShell>
  );
}
