"use client";

import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { toast } from "sonner";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { useProvision } from "@/lib/onboarding/use-provision";
import { Toaster } from "@/components/ui/sonner";
import { scrubOnboardingErrorMessage } from "../_scrub-error";
import { OnboardingShell } from "../_OnboardingShell";

// Returning users (workspace already provisioned in draft) auto-advance.
// Short enough to feel snappy, just long enough to perceive the banner.
const RETURNING_USER_HOLD_MS = 200;

export default function OnboardingWorkspacePage() {
  const router = useRouter();
  const initialWorkspace = useOnboardingDraft((s) => s.workspaceName);
  const initialCompany = useOnboardingDraft((s) => s.companyName);
  const existingWorkspaceId = useOnboardingDraft((s) => s.workspaceId);
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setWorkspace = useOnboardingDraft((s) => s.setWorkspace);
  const setProvision = useOnboardingDraft((s) => s.setProvision);
  const provision = useProvision();
  const [workspaceName, setWorkspaceName] = useState(initialWorkspace);
  const [companyName, setCompanyName] = useState(initialCompany);
  const [error, setError] = useState("");
  const [celebrating, setCelebrating] = useState(false);
  const continueRef = useRef<HTMLButtonElement>(null);
  const isReturning = existingWorkspaceId !== null && existingWorkspaceId.length > 0;

  useEffect(() => {
    setStep(2);
  }, [setStep]);

  useEffect(() => {
    if (!isReturning) return;
    const t = setTimeout(
      () => router.push("/onboarding/connect-provider"),
      RETURNING_USER_HOLD_MS,
    );
    return () => clearTimeout(t);
  }, [isReturning, router]);

  // After a successful fresh provision the toast fires; pull focus to the
  // Continue button so the user can advance with Enter (or a click) without
  // a timer racing their hover.
  useEffect(() => {
    if (celebrating) continueRef.current?.focus();
  }, [celebrating]);

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (celebrating) {
      router.push("/onboarding/connect-provider");
      return;
    }
    setError("");
    const wsName = workspaceName.trim();
    const coName = companyName.trim();
    if (wsName.length === 0 || coName.length === 0) {
      setError("Workspace name and company name are required.");
      return;
    }
    setWorkspace(wsName, coName);
    try {
      const res = await provision.mutateAsync({ workspaceName: wsName, companyName: coName });
      setProvision(res.tenantId, res.workspaceId);
      if (res.created) {
        toast.success("Workspace created — welcome to Skycot.");
        setCelebrating(true);
      } else {
        router.push("/onboarding/connect-provider");
      }
    } catch (e) {
      // Scrub internal protocol detail (X-Argo-Principal etc.) before
      // it reaches the form card — see F5/F5b 2026-05-26-2007.
      setError(scrubOnboardingErrorMessage(e).message);
    }
  }

  const submitting = provision.isPending;
  const canSubmit =
    (celebrating || (workspaceName.trim().length > 0 && companyName.trim().length > 0)) &&
    !submitting;

  return (
    <OnboardingShell
      step={2}
      stepLabel="WORKSPACE"
      eyebrow="STEP TWO · WORKSPACE"
      heroSegments={[
        { text: "Name your " },
        { text: "workspace.", accent: true },
      ]}
      sub={
        <>
          Workers live in workspaces. <strong>You can rename them later</strong> &mdash;
          this is just so we can stop calling it &quot;your workspace.&quot;
        </>
      }
    >
      <Toaster />
      {isReturning ? (
        <p
          className="ob-form-note"
          data-testid="onboarding-workspace-returning-banner"
          role="status"
        >
          Welcome back &mdash; let&apos;s pick up where we left off.
        </p>
      ) : null}
      <form
        className="ob-form-card"
        onSubmit={onSubmit}
        data-testid="onboarding-workspace-form"
        noValidate
      >
        <label className="ob-field">
          <span className="ob-field-label">Workspace name</span>
          <input
            className="ob-field-input"
            data-testid="workspace-name"
            type="text"
            value={workspaceName}
            onChange={(e) => setWorkspaceName(e.target.value)}
            placeholder="Acme Growth"
            autoComplete="off"
            aria-label="Workspace name"
            autoFocus
          />
          <p className="ob-field-hint">
            Tip: if you have multiple workspaces for different teams or business units,
            add a suffix. Try &quot;Acme Co · sales&quot; or &quot;Acme Co · ops&quot;.
          </p>
        </label>
        <label className="ob-field">
          <span className="ob-field-label">Company name</span>
          <input
            className="ob-field-input"
            data-testid="company-name"
            type="text"
            value={companyName}
            onChange={(e) => setCompanyName(e.target.value)}
            placeholder="Acme Inc."
            autoComplete="organization"
            aria-label="Company name"
          />
        </label>
        {error ? (
          <p className="ob-form-error" data-testid="onboarding-workspace-error">
            {error}
          </p>
        ) : null}
        <div className="ob-cta-row is-split">
          <button
            type="button"
            className="ob-cta-back"
            onClick={() => router.push("/onboarding/welcome")}
            data-testid="onboarding-workspace-back"
          >
            ← Back
          </button>
          <button
            ref={continueRef}
            type="submit"
            className="ob-cta"
            disabled={!canSubmit}
            aria-busy={submitting || undefined}
            data-testid="onboarding-workspace-submit"
          >
            {submitting ? <Loader2 className="size-4 animate-spin" aria-hidden /> : null}
            Continue
            <span className="ob-cta-kbd" aria-hidden>
              ↵
            </span>
          </button>
        </div>
        {celebrating ? (
          <p
            className="ob-field-hint"
            data-testid="onboarding-workspace-press-enter-hint"
          >
            <em>Press Enter to continue.</em>
          </p>
        ) : null}
      </form>
    </OnboardingShell>
  );
}
