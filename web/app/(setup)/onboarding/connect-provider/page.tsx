"use client";

import { AlertTriangle, CheckCircle2, ExternalLink, Loader2 } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
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
      eyebrow="STEP THREE · CONNECT AI PROVIDER"
      heroSegments={[
        { text: "Connect " },
        { text: "Anthropic.", accent: true },
      ]}
      sub={
        <>
          A <strong>$0.001 test call</strong> confirms the key works &mdash; or skip and use a
          mock provider for now.
        </>
      }
    >
      <div className="ob-stack mx-auto max-w-xl">
        <form
          className="ob-form-card"
          onSubmit={onSubmit}
          data-testid="onboarding-connect-provider-form"
          noValidate
        >
          <label className="ob-field">
            <span className="ob-field-label">Anthropic API key</span>
            <input
              className="ob-field-input is-mono"
              data-testid="provider-key"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="sk-ant-..."
              autoComplete="off"
              aria-label="Anthropic API key"
              spellCheck={false}
              autoFocus
            />
            <p className="ob-field-hint">
              <a
                href={ANTHROPIC_CONSOLE_URL}
                target="_blank"
                rel="noopener noreferrer"
                data-testid="anthropic-console-link"
              >
                Get a key from the Anthropic console
                <ExternalLink className="ml-1 inline size-3" aria-hidden />
              </a>
            </p>
          </label>
          {error ? (
            <p className="ob-form-error" data-testid="onboarding-provider-error">
              <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden />
              <span>{error}</span>
            </p>
          ) : null}
          {success ? (
            <p className="ob-form-success" data-testid="onboarding-provider-success">
              <CheckCircle2 className="mt-0.5 size-4 shrink-0" aria-hidden />
              <span>Key validated. Forwarding…</span>
            </p>
          ) : null}
          <div
            className="ob-form-note flex items-start gap-2"
            data-testid="onboarding-provider-skip-warning"
            role="note"
          >
            <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden />
            <span>
              <strong>Mock provider</strong> — workers run but responses are simulated.
              Skipping uses a mock provider until you add a real key in Settings · Providers.
            </span>
          </div>
          <div className="ob-cta-row is-split">
            <button
              type="button"
              className="ob-cta-back"
              onClick={() => router.push("/onboarding/workspace")}
              data-testid="onboarding-provider-back"
            >
              ← Back
            </button>
            <div className="flex flex-wrap items-center gap-2">
              <button
                type="button"
                className="ob-cta-ghost"
                onClick={onSkip}
                data-testid="onboarding-provider-skip"
              >
                Skip for now
              </button>
              <button
                type="submit"
                className="ob-cta"
                disabled={!canSubmit}
                aria-busy={submitting || undefined}
                data-testid="onboarding-provider-submit"
              >
                {submitting ? <Loader2 className="size-4 animate-spin" aria-hidden /> : null}
                Validate &amp; continue
                <span className="ob-cta-kbd" aria-hidden>
                  ↵
                </span>
              </button>
            </div>
          </div>
        </form>
      </div>
    </OnboardingShell>
  );
}
