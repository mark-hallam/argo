"use client";

import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { useProvision } from "@/lib/onboarding/use-provision";
import { OnboardingShell } from "../_OnboardingShell";

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
    <OnboardingShell
      step={2}
      stepLabel="WORKSPACE"
      eyebrow="STEP TWO"
      heroSegments={[
        { text: "Name your " },
        { text: "workspace.", accent: true },
      ]}
      sub={
        <>
          Workers live in workspaces. <strong>You can rename them later.</strong>
        </>
      }
    >
      <section className="mx-auto flex max-w-xl flex-col gap-6">
        <form
          className="flex flex-col gap-4 rounded-[12px] border border-[#ece9f5] bg-white/80 p-6 shadow-sm"
          onSubmit={onSubmit}
          data-testid="onboarding-workspace-form"
        >
          <label className="block space-y-1.5">
            <span className="text-sm font-medium text-[#1e1b4b]">Workspace name</span>
            <input
              className="h-10 w-full rounded-[8px] border border-[#d1d1d1] bg-white px-3 text-sm text-[#1e1b4b] outline-none focus:border-[#8b5cf6] focus:ring-2 focus:ring-[#8b5cf6]/30"
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
            <span className="text-sm font-medium text-[#1e1b4b]">Company name</span>
            <input
              className="h-10 w-full rounded-[8px] border border-[#d1d1d1] bg-white px-3 text-sm text-[#1e1b4b] outline-none focus:border-[#8b5cf6] focus:ring-2 focus:ring-[#8b5cf6]/30"
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
            <p className="text-sm text-[#dc2626]" data-testid="onboarding-workspace-error">
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
    </OnboardingShell>
  );
}
