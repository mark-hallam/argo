"use client";

import { AlertTriangle, CheckCircle2, ExternalLink, Loader2 } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { ApiError } from "@/lib/api/client";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { useProviderKey } from "@/lib/onboarding/use-provider-key";
import { scrubOnboardingErrorMessage } from "../_scrub-error";
import { OnboardingShell } from "../_OnboardingShell";

const ANTHROPIC_CONSOLE_URL = "https://console.anthropic.com/settings/keys";

type ProviderError = {
  message: string;
  tone: "error" | "note";
  preferSkip?: boolean;
};

function errorCodeFromEnvelope(value: unknown): string | undefined {
  if (!value || typeof value !== "object" || !("error" in value)) {
    return undefined;
  }
  const envelope = value as { error?: { code?: unknown } };
  return typeof envelope.error?.code === "string" ? envelope.error.code : undefined;
}

function errorCodeFromBody(body: unknown): string | undefined {
  if (typeof body === "string") {
    try {
      return errorCodeFromEnvelope(JSON.parse(body) as unknown);
    } catch {
      return undefined;
    }
  }
  return errorCodeFromEnvelope(body);
}

function providerErrorFor(err: unknown): ProviderError {
  const code = err instanceof ApiError ? err.code ?? errorCodeFromBody(err.body) : undefined;
  if (code === "provider_validation_unconfigured") {
    return {
      message:
        "We couldn't validate your key right now. You can continue and use the mock provider until validation is available.",
      tone: "note",
      preferSkip: true,
    };
  }
  if (
    code === "anthropic_rejected_key" ||
    (err instanceof ApiError && err.message === "Anthropic rejected the key")
  ) {
    return {
      message: "Anthropic rejected that key. Double-check that you copied the whole value.",
      tone: "error",
    };
  }
  if (
    code === "bad_request" ||
    code === "invalid_key_shape" ||
    (err instanceof ApiError && err.status === 400)
  ) {
    return {
      message: "Your key doesn't look right. Anthropic keys start with sk-ant-.",
      tone: "error",
    };
  }
  return {
    message: "Something went wrong. You can skip this step and add a key later.",
    tone: "error",
    preferSkip: true,
  };
}

function logProviderValidationError(err: unknown) {
  if (process.env.NODE_ENV !== "production") {
    console.warn("Provider key validation failed", err);
  }
}

export default function OnboardingConnectProviderPage() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setProvider = useOnboardingDraft((s) => s.setProvider);
  const validate = useProviderKey();
  const [apiKey, setApiKey] = useState("");
  const [providerError, setProviderError] = useState<ProviderError | null>(null);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    setStep(3);
  }, [setStep]);

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setProviderError(null);
    setSuccess(false);
    const key = apiKey.trim();
    if (!key.startsWith("sk-ant-")) {
      setProviderError({
        message: "Anthropic API keys start with sk-ant-.",
        tone: "error",
      });
      return;
    }
    try {
      await validate.mutateAsync({ apiKey: key });
      setProvider(true, false);
      setSuccess(true);
      setApiKey("");
      router.push("/onboarding/pick-vertical");
    } catch (err) {
      logProviderValidationError(err);
      // HEAD's structured providerErrorFor() returns ProviderError objects
      // that don't expose raw gateway headers; scrubber not needed here.
      setProviderError(providerErrorFor(err));
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
          {providerError ? (
            <p
              className={providerError.tone === "note" ? "ob-form-note" : "ob-form-error"}
              data-testid="onboarding-provider-error"
            >
              <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden />
              <span>{providerError.message}</span>
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
                className={providerError?.preferSkip ? "ob-cta" : "ob-cta-ghost"}
                onClick={onSkip}
                data-testid="onboarding-provider-skip"
              >
                Skip for now
              </button>
              <button
                type="submit"
                className={providerError?.preferSkip ? "ob-cta-ghost" : "ob-cta"}
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
