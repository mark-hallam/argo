"use client";

import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { useProvision } from "@/lib/onboarding/use-provision";

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
      router.push("/onboarding/connect-provider");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Provisioning failed. Please try again.");
    }
  }

  const submitting = provision.isPending;
  const canSubmit = workspaceName.trim().length > 0 && companyName.trim().length > 0 && !submitting;

  return (
    <section className="mx-auto flex max-w-xl flex-col gap-6">
      <header className="space-y-2 text-center">
        <p className="text-xs font-medium uppercase tracking-[0.12em] text-[var(--muted-foreground)]">
          Step 2 of 5
        </p>
        <h1 className="text-2xl font-semibold tracking-tight">Name your workspace</h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          Workers live in workspaces. You can rename them later.
        </p>
      </header>
      <form
        className="flex flex-col gap-4 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-6 shadow-sm"
        onSubmit={onSubmit}
        data-testid="onboarding-workspace-form"
      >
        <label className="block space-y-1.5">
          <span className="text-sm font-medium">Workspace name</span>
          <input
            className="h-10 w-full rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)] px-3 text-sm outline-none focus:ring-2 focus:ring-[var(--ring)]"
            data-testid="workspace-name"
            type="text"
            value={workspaceName}
            onChange={(e) => setWorkspaceName(e.target.value)}
            placeholder="Acme Growth"
            autoComplete="off"
            aria-label="Workspace name"
          />
        </label>
        <label className="block space-y-1.5">
          <span className="text-sm font-medium">Company name</span>
          <input
            className="h-10 w-full rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)] px-3 text-sm outline-none focus:ring-2 focus:ring-[var(--ring)]"
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
          <p className="text-sm text-[var(--destructive)]" data-testid="onboarding-workspace-error">
            {error}
          </p>
        ) : null}
        <div className="flex items-center justify-between pt-2">
          <Button
            type="button"
            variant="ghost"
            onClick={() => router.push("/onboarding/welcome")}
            data-testid="onboarding-workspace-back"
          >
            Back
          </Button>
          <Button
            type="submit"
            disabled={!canSubmit}
            loading={submitting}
            data-testid="onboarding-workspace-submit"
          >
            Continue
          </Button>
        </div>
      </form>
    </section>
  );
}
