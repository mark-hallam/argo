"use client";

import { AlertTriangle, CheckCircle2, ExternalLink } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { useProviderKey } from "@/lib/onboarding/use-provider-key";
import { OnboardingShell } from "../_OnboardingShell";

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
    <OnboardingShell
      step={3}
      stepLabel="PROVIDER"
      eyebrow="STEP THREE"
      heroSegments={[
        { text: "Connect " },
        { text: "Anthropic.", accent: true },
      ]}
      sub={
        <>
          A <strong>$0.001 test call</strong> confirms the key works — or skip and use a mock
          provider for now.
        </>
      }
    >
      <section className="mx-auto flex max-w-xl flex-col gap-6">
        <form
          className="flex flex-col gap-4 rounded-[12px] border border-[#ece9f5] bg-white/80 p-6 shadow-sm"
          onSubmit={onSubmit}
          data-testid="onboarding-connect-provider-form"
        >
          <label className="block space-y-1.5">
            <span className="text-sm font-medium text-[#1e1b4b]">Anthropic API key</span>
            <input
              className="h-10 w-full rounded-[8px] border border-[#d1d1d1] bg-white px-3 font-mono text-sm text-[#1e1b4b] outline-none focus:border-[#8b5cf6] focus:ring-2 focus:ring-[#8b5cf6]/30"
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
              className="inline-flex items-center gap-1 text-xs text-[#6d28d9] underline-offset-2 hover:underline"
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
              className="flex items-start gap-2 text-sm text-[#dc2626]"
              data-testid="onboarding-provider-error"
            >
              <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden />
              <span>{error}</span>
            </p>
          ) : null}
          {success ? (
            <p
              className="flex items-start gap-2 text-sm text-[#0f766e]"
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
          className="rounded-[8px] border border-[#fcd34d]/40 bg-[#fcd34d]/10 px-3 py-2 text-xs text-[#854d0e]"
          data-testid="onboarding-provider-skip-warning"
        >
          Skipping uses a mock provider. Workers will run, but responses are simulated until you add a
          real key in Settings.
        </p>
      </section>
    </OnboardingShell>
  );
}
