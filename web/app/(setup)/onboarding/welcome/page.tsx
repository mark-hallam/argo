"use client";

import { ArrowRight } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { Button } from "@/components/ui/Button";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { OnboardingShell } from "../_OnboardingShell";

export default function OnboardingWelcomePage() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);

  useEffect(() => {
    setStep(1);
  }, [setStep]);

  return (
    <OnboardingShell
      step={1}
      stepLabel="WELCOME"
      eyebrow="WELCOME TO SKYCOT"
      heroSegments={[{ text: "Welcome to SKYCOT." }]}
      sub={
        <>
          Let&apos;s hire your first worker. <strong>(90 seconds.)</strong>
        </>
      }
    >
      <section
        className="flex flex-col items-center gap-8 text-center"
        data-testid="onboarding-welcome"
      >
        <ul className="grid w-full max-w-2xl grid-cols-1 gap-3 text-left sm:grid-cols-3">
          <li
            className="rounded-[12px] border border-[#ece9f5] bg-white/70 p-4 shadow-sm"
            data-testid="onboarding-pillar-audit-first"
          >
            <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.18em] text-[#52525b]">
              Audit-first
            </p>
            <p className="mt-1.5 text-sm text-[#1e1b4b]">
              Every action signed and replayable — SOC 2-ready from day one.
            </p>
          </li>
          <li
            className="rounded-[12px] border border-[#ece9f5] bg-white/70 p-4 shadow-sm"
            data-testid="onboarding-pillar-single-binary"
          >
            <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.18em] text-[#52525b]">
              Single-binary
            </p>
            <p className="mt-1.5 text-sm text-[#1e1b4b]">
              One Rust process. No Kubernetes, no sidecars, no surprises.
            </p>
          </li>
          <li
            className="rounded-[12px] border border-[#ece9f5] bg-white/70 p-4 shadow-sm"
            data-testid="onboarding-pillar-enterprise-grade"
          >
            <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.18em] text-[#52525b]">
              Enterprise-grade
            </p>
            <p className="mt-1.5 text-sm text-[#1e1b4b]">
              Multi-tenant, capability-based RBAC, GDPR Article 30 trails.
            </p>
          </li>
        </ul>
        <Button
          size="lg"
          data-testid="onboarding-welcome-continue"
          onClick={() => router.push("/onboarding/workspace")}
        >
          Continue
          <ArrowRight aria-hidden />
        </Button>
      </section>
    </OnboardingShell>
  );
}
