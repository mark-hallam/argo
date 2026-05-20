"use client";

import { Sparkles } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { useFirstHire } from "@/lib/onboarding/use-first-hire";

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
      setError(
        e instanceof Error
          ? `${e.message}. We'll create your worker in the background — try again or skip to your team.`
          : "Genesis call failed. Try again.",
      );
    }
  }

  const submitting = firstHire.isPending;
  const canSubmit = prompt.trim().length > 0 && !submitting;

  return (
    <section className="mx-auto flex max-w-2xl flex-col gap-6">
      <header className="space-y-2 text-center">
        <p className="text-xs font-medium uppercase tracking-[0.12em] text-[var(--muted-foreground)]">
          Step 5 of 5
        </p>
        <h1 className="text-2xl font-semibold tracking-tight">Describe your first worker</h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          Plain English is fine. Give them a name, a role, and what they should do this week.
        </p>
      </header>
      <form
        className="flex flex-col gap-4 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-6 shadow-sm"
        onSubmit={onSubmit}
        data-testid="onboarding-first-hire-form"
      >
        <label className="block space-y-1.5">
          <span className="text-sm font-medium">Tell us about them</span>
          <textarea
            className="min-h-32 w-full rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)] p-3 text-sm outline-none focus:ring-2 focus:ring-[var(--ring)]"
            data-testid="first-hire-prompt"
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder={EXAMPLE_PROMPT}
            aria-label="First hire description"
          />
          <p className="text-xs text-[var(--muted-foreground)]">
            Example: <em>{EXAMPLE_PROMPT}</em>
          </p>
        </label>
        {error ? (
          <p
            className="text-sm text-[var(--destructive)]"
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
  );
}
