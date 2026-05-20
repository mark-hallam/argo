"use client";

import { Check } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { OnboardingShell } from "../_OnboardingShell";

interface VerticalCard {
  id: string;
  displayName: string;
  summary: string;
  skillCount: number;
  sampleDuties: string[];
}

const VERTICALS: ReadonlyArray<VerticalCard> = [
  {
    id: "gtm-engineer",
    displayName: "GTM Engineer",
    summary: "Go-to-market automation: lead enrichment, sequencing, signal triage.",
    skillCount: 5,
    sampleDuties: ["Enrich inbound leads", "Send Day-0 sequence", "Triage signal alerts"],
  },
  {
    id: "marketing-intern",
    displayName: "Marketing Intern",
    summary: "Content drafting, social posts, weekly competitive scans.",
    skillCount: 5,
    sampleDuties: ["Draft blog posts", "Schedule social posts", "Weekly competitive scan"],
  },
  {
    id: "sales-sdr",
    displayName: "Sales SDR",
    summary: "Outbound prospecting and meeting booking with approval gates.",
    skillCount: 5,
    sampleDuties: ["Research prospects", "Draft cold-outreach", "Book qualified meetings"],
  },
  {
    id: "customer-support",
    displayName: "Customer Support",
    summary: "Tier-1 ticket triage, FAQ replies, escalation routing.",
    skillCount: 5,
    sampleDuties: ["Triage new tickets", "Draft FAQ replies", "Escalate complex cases"],
  },
  {
    id: "finance-ar",
    displayName: "Accounts Receivable",
    summary: "Invoicing, dunning emails, payment reconciliation.",
    skillCount: 4,
    sampleDuties: ["Create invoices", "Send dunning emails", "Reconcile payments"],
  },
];

export default function OnboardingPickVerticalPage() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setVertical = useOnboardingDraft((s) => s.setVertical);
  const initialVertical = useOnboardingDraft((s) => s.verticalId);
  const [selected, setSelected] = useState<string | null>(initialVertical);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    setStep(4);
  }, [setStep]);

  async function onChoose(verticalId: string) {
    setSelected(verticalId);
    setError("");
    setBusy(true);
    setVertical(verticalId);
    try {
      const res = await fetch(
        `${process.env.NEXT_PUBLIC_GATEWAY_URL ?? ""}/api/v1/onboarding/install-vertical`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json", Accept: "application/json" },
          body: JSON.stringify({ vertical_id: verticalId }),
        },
      );
      if (!res.ok && res.status !== 404) {
        throw new Error(`gateway returned HTTP ${res.status}`);
      }
      router.push("/onboarding/first-hire");
    } catch (e) {
      setError(
        e instanceof Error
          ? `${e.message}. We saved your choice — you can retry on the next step.`
          : "Install failed; choice saved locally.",
      );
      router.push("/onboarding/first-hire");
    } finally {
      setBusy(false);
    }
  }

  return (
    <OnboardingShell
      step={4}
      stepLabel="VERTICAL"
      eyebrow="STEP FOUR"
      heroSegments={[
        { text: "Pick a " },
        { text: "starting template.", accent: true },
      ]}
      sub={
        <>
          We install the <strong>skills, memory schema, and approval gates</strong>. You can swap or
          extend later.
        </>
      }
    >
      <section className="mx-auto flex max-w-3xl flex-col gap-6">
        <ul
          className="grid grid-cols-1 gap-4 sm:grid-cols-2"
          data-testid="onboarding-vertical-grid"
        >
          {VERTICALS.map((v) => {
            const isSelected = selected === v.id;
            return (
              <li key={v.id}>
                <button
                  type="button"
                  onClick={() => onChoose(v.id)}
                  disabled={busy}
                  aria-pressed={isSelected}
                  data-testid={`vertical-${v.id}`}
                  className={`flex h-full w-full flex-col gap-2 rounded-[12px] border p-4 text-left transition-colors hover:border-[#8b5cf6]/40 hover:bg-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#8b5cf6]/40 disabled:cursor-wait ${
                    isSelected
                      ? "border-[#8b5cf6] bg-white shadow-[0_8px_24px_-12px_rgba(139,92,246,0.35)] ring-1 ring-[#8b5cf6]"
                      : "border-[#ece9f5] bg-white/70"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="text-base font-semibold text-[#1e1b4b]">{v.displayName}</span>
                    {isSelected ? (
                      <Check className="size-4 text-[#8b5cf6]" aria-hidden />
                    ) : null}
                  </div>
                  <p className="text-sm text-[#3f3f46]">{v.summary}</p>
                  <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.12em] text-[#71717a]">
                    {v.skillCount} skills
                  </p>
                  <ul className="mt-1 space-y-0.5 text-xs text-[#3f3f46]">
                    {v.sampleDuties.map((d) => (
                      <li key={d}>• {d}</li>
                    ))}
                  </ul>
                </button>
              </li>
            );
          })}
        </ul>
        {error ? (
          <p
            className="rounded-[8px] border border-[#fcd34d]/40 bg-[#fcd34d]/10 px-3 py-2 text-xs text-[#854d0e]"
            data-testid="onboarding-vertical-error"
          >
            {error}
          </p>
        ) : null}
        <div className="flex items-center justify-between">
          <Button
            type="button"
            variant="ghost"
            onClick={() => router.push("/onboarding/connect-provider")}
            data-testid="onboarding-vertical-back"
          >
            Back
          </Button>
        </div>
      </section>
    </OnboardingShell>
  );
}
