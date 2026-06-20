"use client";

import { AlertTriangle, CheckCircle2, ExternalLink, Loader2 } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { ApiError } from "@/lib/api/client";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { useProviderKey } from "@/lib/onboarding/use-provider-key";
import { isDemoMode } from "@/lib/skycot/demo-mode";
import { WizardSkeleton } from "@/components/auth/WizardSkeleton";
import { scrubOnboardingErrorMessage } from "../_scrub-error";
import { OnboardingShell } from "../_OnboardingShell";
import { ONBOARDING_MAX_LENGTHS, parseField, providerKeySchema } from "../schemas";

const ANTHROPIC_CONSOLE_URL = "https://console.anthropic.com/settings/keys";

// sessionStorage (not localStorage) so the key clears the moment the
// tab closes. Letting the user refresh / back-navigate mid-edit without
// losing what they pasted is worth the per-tab persistence; baking the
// key into long-lived browser storage is not.
const API_KEY_SESSION_STORAGE_KEY = "skycot:onboarding:apiKey";

function readPersistedApiKey(): string {
  if (typeof window === "undefined") return "";
  try {
    return window.sessionStorage.getItem(API_KEY_SESSION_STORAGE_KEY) ?? "";
  } catch {
    return "";
  }
}

function persistApiKey(value: string): void {
  if (typeof window === "undefined") return;
  try {
    if (value.length === 0) {
      window.sessionStorage.removeItem(API_KEY_SESSION_STORAGE_KEY);
    } else {
      window.sessionStorage.setItem(API_KEY_SESSION_STORAGE_KEY, value);
    }
  } catch {
    // sessionStorage is best-effort — if the browser blocks it (private
    // mode, quota), the wizard still works; the user just loses the
    // step-back grace.
  }
}

function clearPersistedApiKey(): void {
  if (typeof window === "undefined") return;
  try {
    window.sessionStorage.removeItem(API_KEY_SESSION_STORAGE_KEY);
  } catch {
    // Same best-effort guarantee — clearing failure is non-fatal.
  }
}

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
  // No persisted localStorage input is read here directly (the API
  // key lives in `sessionStorage`), but the wizard chrome — pip
  // strip, step label, save-and-exit affordance — derives from the
  // shared draft store. Gating on `hasHydrated` keeps the wizard's
  // chrome from briefly flashing the default step before persist
  // restores. Consistency across the four steps also protects future
  // edits that pull a persisted field into this page.
  const hasHydrated = useOnboardingDraft((s) => s.hasHydrated);
  if (!hasHydrated) {
    return <WizardSkeleton step={3} />;
  }
  return <ConnectProviderFormBody />;
}

function ConnectProviderFormBody() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setProvider = useOnboardingDraft((s) => s.setProvider);
  const validate = useProviderKey();
  const [apiKey, setApiKey] = useState("");
  const [providerError, setProviderError] = useState<ProviderError | null>(null);
  const [success, setSuccess] = useState(false);
  const [successIsMock, setSuccessIsMock] = useState(false);
  const demoMode = isDemoMode();

  useEffect(() => {
    setStep(3);
  }, [setStep]);

  // Hydrate the input from sessionStorage on mount so a step-back or
  // mid-step refresh doesn't force the user to paste their key again.
  // sessionStorage is tab-scoped — the value vanishes when the tab
  // closes, which matches the user-facing promise below the input.
  useEffect(() => {
    const persisted = readPersistedApiKey();
    if (persisted.length > 0) {
      setApiKey(persisted);
    }
  }, []);

  function onApiKeyChange(value: string) {
    setApiKey(value);
    persistApiKey(value);
  }

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setProviderError(null);
    setSuccess(false);
    // Validate shape + length through the shared schema (keeps the
    // `sk-ant-` prefix check, adds an upper bound so a runaway paste can't
    // reach the gateway). The schema trims, so `key` is the cleaned value.
    const keyResult = parseField(providerKeySchema, apiKey);
    if (!keyResult.ok) {
      setProviderError({ message: keyResult.error, tone: "error" });
      return;
    }
    const key = keyResult.value;
    try {
      const result = await validate.mutateAsync({ apiKey: key });
      setProvider(true, false);
      setSuccess(true);
      setSuccessIsMock(result.mockProvider);
      setApiKey("");
      clearPersistedApiKey();
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
    clearPersistedApiKey();
    router.push("/onboarding/pick-vertical");
  }

  // Skycot-managed keys — the DEFAULT. The platform supplies the Anthropic
  // key (the runtime falls back to the platform key when a tenant has no
  // bring-your-own key), billed at API cost + the Pro overage markup. No key
  // is POSTed, so the tenant simply has no stored BYOK key and the platform
  // fallback drives execution with REAL responses — never the mock provider.
  function onUseManagedKeys() {
    setProvider(true, false);
    clearPersistedApiKey();
    router.push("/onboarding/pick-vertical");
  }

  const submitting = validate.isPending;
  const canSubmit = apiKey.trim().length > 0 && !submitting;

  return (
    <OnboardingShell
      step={3}
      stepLabel="PROVIDER"
      eyebrow="STEP 03 · CONNECT AI PROVIDER"
      heroSegments={[
        { text: "Connect " },
        { text: "Anthropic.", accent: true },
      ]}
      sub={
        demoMode ? (
          <>
            Workers run with a <strong>mock provider</strong> (simulated responses).
            Type any <code>sk-ant-</code> key to continue; no Anthropic call is made.
          </>
        ) : (
          <>
            Use Skycot&rsquo;s managed keys (recommended &mdash; no key needed, billed at
            API cost plus a small markup), or connect your own Anthropic key below. We
            verify the key shape (<code>sk-ant-...</code>).
          </>
        )
      }
    >
      <div className="ob-stack mx-auto max-w-xl">
        <form
          className="ob-form-card"
          onSubmit={onSubmit}
          data-testid="onboarding-connect-provider-form"
          noValidate
        >
          {!demoMode ? (
            <div className="ob-stack" data-testid="onboarding-provider-managed">
              <button
                type="button"
                className="ob-cta w-full justify-center"
                onClick={onUseManagedKeys}
                data-testid="onboarding-provider-use-managed"
              >
                Use Skycot managed keys
              </button>
              <p
                className="ob-field-hint"
                data-testid="onboarding-provider-managed-note"
              >
                Recommended — no API key needed. Your teammates run on Skycot&rsquo;s
                keys, billed at API cost plus a small markup. Real responses, never a
                simulation.
              </p>
              <p className="ob-field-hint text-center opacity-70">
                — or connect your own Anthropic key —
              </p>
            </div>
          ) : null}
          <label className="ob-field">
            <span className="ob-field-label">Anthropic API key</span>
            <input
              className="ob-field-input is-mono"
              data-testid="provider-key"
              type="password"
              value={apiKey}
              onChange={(e) => onApiKeyChange(e.target.value)}
              placeholder={demoMode ? "sk-ant-anything-works-here-in-demo" : "sk-ant-..."}
              autoComplete="off"
              aria-label="Anthropic API key"
              spellCheck={false}
              maxLength={ONBOARDING_MAX_LENGTHS.providerKey}
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
            <p
              className="ob-field-hint"
              data-testid="onboarding-provider-key-scope-note"
            >
              Your key stays in this browser tab. Closing the tab clears it.
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
              <span>
                {successIsMock
                  ? "Mock provider connected — workers run with simulated responses for the demo. Forwarding…"
                  : "Key validated. Forwarding…"}
              </span>
            </p>
          ) : null}
          {demoMode ? (
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
          ) : null}
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
              {demoMode ? (
                <button
                  type="button"
                  className="ob-cta-ghost"
                  onClick={onSkip}
                  data-testid="onboarding-provider-skip"
                >
                  Skip for now
                </button>
              ) : null}
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
