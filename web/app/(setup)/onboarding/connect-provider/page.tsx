"use client";

import { AlertTriangle, CheckCircle2, ExternalLink } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { useProviderKey } from "@/lib/onboarding/use-provider-key";

const ANTHROPIC_CONSOLE_URL = "https://console.anthropic.com/settings/keys";

export default function OnboardingConnectProviderPage() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setProvider = useOnboardingDraft((s) => s.setProvider);
  const validate = useProviderKey();
  const [apiKey, setApiKey] = useState("");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    setStep(3);
  }, [setStep]);

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setError("");
    setSuccess(false);
    const key = apiKey.trim();
    if (!key.startsWith("sk-ant-")) {
      setError("Anthropic API keys start with sk-ant-.");
      return;
    }
    try {
      await validate.mutateAsync({ apiKey: key });
      setProvider(true, false);
      setSuccess(true);
      setApiKey("");
      router.push("/onboarding/pick-vertical");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Validation failed. Please try again.");
    }
  }

  function onSkip() {
    setProvider(false, true);
    router.push("/onboarding/pick-vertical");
  }

  const submitting = validate.isPending;
  const canSubmit = apiKey.trim().length > 0 && !submitting;

  return (
    <section className="mx-auto flex max-w-xl flex-col gap-6">
      <header className="space-y-2 text-center">
        <p className="text-xs font-medium uppercase tracking-[0.12em] text-[var(--muted-foreground)]">
          Step 3 of 5
        </p>
        <h1 className="text-2xl font-semibold tracking-tight">Connect Anthropic</h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          We&apos;ll send a $0.001 test call to confirm the key works. You can skip and use a mock
          provider for now.
        </p>
      </header>
      <form
        className="flex flex-col gap-4 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-6 shadow-sm"
        onSubmit={onSubmit}
        data-testid="onboarding-connect-provider-form"
      >
        <label className="block space-y-1.5">
          <span className="text-sm font-medium">Anthropic API key</span>
          <input
            className="h-10 w-full rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)] px-3 font-mono text-sm outline-none focus:ring-2 focus:ring-[var(--ring)]"
            data-testid="provider-key"
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="sk-ant-..."
            autoComplete="off"
            aria-label="Anthropic API key"
            spellCheck={false}
          />
          <a
            className="inline-flex items-center gap-1 text-xs text-[var(--primary)] underline-offset-2 hover:underline"
            href={ANTHROPIC_CONSOLE_URL}
            target="_blank"
            rel="noopener noreferrer"
            data-testid="anthropic-console-link"
          >
            Get a key from the Anthropic console
            <ExternalLink className="size-3" aria-hidden />
          </a>
        </label>
        {error ? (
          <p
            className="flex items-start gap-2 text-sm text-[var(--destructive)]"
            data-testid="onboarding-provider-error"
          >
            <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden />
            <span>{error}</span>
          </p>
        ) : null}
        {success ? (
          <p
            className="flex items-start gap-2 text-sm text-[var(--success-foreground)]"
            data-testid="onboarding-provider-success"
          >
            <CheckCircle2 className="mt-0.5 size-4 shrink-0" aria-hidden />
            <span>Key validated. Forwarding…</span>
          </p>
        ) : null}
        <div className="flex items-center justify-between pt-2">
          <Button
            type="button"
            variant="ghost"
            onClick={() => router.push("/onboarding/workspace")}
            data-testid="onboarding-provider-back"
          >
            Back
          </Button>
          <div className="flex items-center gap-2">
            <Button
              type="button"
              variant="ghost"
              onClick={onSkip}
              data-testid="onboarding-provider-skip"
            >
              Skip for now
            </Button>
            <Button
              type="submit"
              disabled={!canSubmit}
              loading={submitting}
              data-testid="onboarding-provider-submit"
            >
              Validate &amp; continue
            </Button>
          </div>
        </div>
      </form>
      <p
        className="rounded-[var(--radius-sm)] border border-[var(--warning)]/30 bg-[var(--warning)]/10 px-3 py-2 text-xs text-[var(--warning-foreground)]"
        data-testid="onboarding-provider-skip-warning"
      >
        Skipping uses a mock provider. Workers will run, but responses are simulated until you add a
        real key in Settings.
      </p>
    </section>
  );
}
