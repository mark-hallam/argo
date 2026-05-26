"use client";

import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { toast } from "sonner";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { useProvision } from "@/lib/onboarding/use-provision";
import { Toaster } from "@/components/ui/sonner";
import { scrubOnboardingErrorMessage } from "../_scrub-error";
import { OnboardingShell } from "../_OnboardingShell";

// On fresh provisions we hold the wizard on this step briefly so the
// celebration toast is actually perceived before the route changes.
// Idempotent re-calls navigate immediately — no celebration owed.
const CELEBRATION_HOLD_MS = 1200;

export default function OnboardingWorkspacePage() {
  const router = useRouter();
  const initialWorkspace = useOnboardingDraft((s) => s.workspaceName);
  const initialCompany = useOnboardingDraft((s) => s.companyName);
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setWorkspace = useOnboardingDraft((s) => s.setWorkspace);
  const setProvision = useOnboardingDraft((s) => s.setProvision);
  const provision = useProvision();
  const [workspaceName, setWorkspaceName] = useState(initialWorkspace);
  const [companyName, setCompanyName] = useState(initialCompany);
  const [error, setError] = useState("");

  useEffect(() => {
    setStep(2);
  }, [setStep]);

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
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
        setTimeout(() => router.push("/onboarding/connect-provider"), CELEBRATION_HOLD_MS);
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
  const canSubmit = workspaceName.trim().length > 0 && companyName.trim().length > 0 && !submitting;

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
      </form>
    </OnboardingShell>
  );
}
