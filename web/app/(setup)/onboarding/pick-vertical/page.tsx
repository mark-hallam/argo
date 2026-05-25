"use client";

import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { apiPost, ApiError } from "@/lib/api/client";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { OnboardingShell } from "../_OnboardingShell";

interface InstallVerticalResponse {
  installed: boolean;
  vertical_id: string;
  workflows_added: number;
  skills_added: number;
}

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
    id: "marketplace-seller",
    displayName: "Marketplace Seller",
    summary: "Inventory readiness, listing drafts, buyer replies, and price checks.",
    skillCount: 6,
    sampleDuties: ["Audit inventory", "Draft marketplace listings", "Negotiate offers"],
  },
  {
    id: "recruiter",
    displayName: "Recruiter",
    summary: "Resume screening, candidate outreach, interview scheduling, and offer handoff.",
    skillCount: 6,
    sampleDuties: ["Score resumes", "Draft screening emails", "Schedule interviews"],
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
      // web-gateway-endpoints-onboarding-batch: the gateway endpoint now
      // exists, so use the canonical `apiPost` client (bearer auth +
      // typed error envelope). Errors block the redirect so the operator
      // can retry instead of hard-sticking on /first-hire's 404.
      await apiPost<InstallVerticalResponse, { vertical_id: string }>(
        "/api/v1/onboarding/install-vertical",
        { vertical_id: verticalId },
      );
      router.push("/onboarding/first-hire");
    } catch (e) {
      const message =
        e instanceof ApiError
          ? `${e.message} (HTTP ${e.status}). Choice saved locally — retry the step.`
          : e instanceof Error
            ? `${e.message}. Choice saved locally — retry the step.`
            : "Install failed; choice saved locally.";
      setError(message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <OnboardingShell
      step={4}
      stepLabel="VERTICAL"
      eyebrow="STEP FOUR · PICK A ROLE"
      heroSegments={[
        { text: "Pick a " },
        { text: "starting template.", accent: true },
      ]}
      sub={
        <>
          We install the <strong>skills, memory schema, and approval gates</strong>. You can swap
          or extend later.
        </>
      }
    >
      <div className="ob-stack mx-auto max-w-4xl">
        <div className="ob-cta-row is-split">
          <button
            type="button"
            className="ob-cta-back"
            onClick={() => router.push("/onboarding/connect-provider")}
            data-testid="onboarding-vertical-back"
          >
            ← Back
          </button>
          <span className="ob-field-hint">
            Pick a template to continue &mdash; we install the skills and route you to step 5.
          </span>
        </div>
        <ul className="ob-roles" data-testid="onboarding-vertical-grid">
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
                  className={isSelected ? "ob-role-card is-selected" : "ob-role-card"}
                >
                  <div className="ob-role-head">
                    <span className="ob-role-name">{v.displayName}</span>
                    <span className="ob-role-check" aria-hidden>
                      ✓
                    </span>
                  </div>
                  <p className="ob-role-desc">{v.summary}</p>
                  <div className="ob-skill-chips">
                    {v.sampleDuties.map((d) => (
                      <span key={d} className="ob-skill-chip">
                        {d}
                      </span>
                    ))}
                  </div>
                  <p className="ob-role-meta">
                    <strong>{v.skillCount}</strong> skills · ready to install
                  </p>
                </button>
              </li>
            );
          })}
        </ul>
        {error ? (
          <p className="ob-form-note" data-testid="onboarding-vertical-error">
            {error}
          </p>
        ) : null}
      </div>
    </OnboardingShell>
  );
}
