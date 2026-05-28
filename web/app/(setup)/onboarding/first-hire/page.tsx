"use client";

import { Loader2, Sparkles } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useMemo, useState } from "react";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import {
  FIRST_HIRE_VERIFY_FAILED_MESSAGE,
  useFirstHire,
} from "@/lib/onboarding/use-first-hire";
import { FirstHireErrorBanner } from "@/components/skycot/onboarding/FirstHireErrorBanner";
import { scrubOnboardingErrorMessage } from "../_scrub-error";
import { OnboardingShell } from "../_OnboardingShell";
import {
  findSamplePersona,
  getSamplePersona,
  SAMPLE_PERSONAS,
  type SamplePersona,
} from "@/lib/sample-personas";

// Canonical sample-worker UUIDs (mirror `SAMPLE_PERSONAS` ids in
// `web/lib/sample-personas.ts`). These are the destinations the demo
// short-circuit in `use-first-hire.ts::buildDemoFirstHireResult` lands
// on per vertical_id — kept in sync here so the preview card matches
// the post-submit redirect target. Without this mirror, the operator
// could "Hire Jamal" but land on Mariah.
const SAMPLE_MARIAH = "018f67d0-0000-7000-8000-000000000001";
const SAMPLE_PRIYA = "018f67d0-0000-7000-8000-000000000002";
const SAMPLE_JAMAL = "018f67d0-0000-7000-8000-000000000003";
const SAMPLE_NORI = "018f67d0-0000-7000-8000-000000000004";

const VERTICAL_TO_SAMPLE_ID: Record<string, string> = {
  "marketplace-seller": SAMPLE_MARIAH,
  recruiter: SAMPLE_PRIYA,
  "customer-support": SAMPLE_JAMAL,
  "marketing-intern": SAMPLE_NORI,
  // Fallback mappings for the other templates surfaced in
  // pick-vertical/page.tsx. The picker shows 5 templates; only 4 have
  // canonical persona matches. Map the rest to the closest persona by
  // role-shape so the card preview is plausible.
  "gtm-engineer": SAMPLE_MARIAH,
  "sales-sdr": SAMPLE_PRIYA,
  "accounts-receivable": SAMPLE_PRIYA,
  "finance-ar": SAMPLE_PRIYA,
};

function resolveCanonicalPersona(verticalId: string | null | undefined): SamplePersona {
  if (verticalId && VERTICAL_TO_SAMPLE_ID[verticalId]) {
    const persona = findSamplePersona(VERTICAL_TO_SAMPLE_ID[verticalId]!);
    if (persona) return persona;
  }
  // Default to Mariah — Scene-3 runbook treats her as the canonical
  // "first hire" placeholder when no vertical match is available.
  return getSamplePersona(SAMPLE_MARIAH);
}

function templateLabel(verticalId: string | null | undefined): string {
  const labels: Record<string, string> = {
    "marketplace-seller": "Marketplace Seller",
    recruiter: "Recruiter",
    "customer-support": "Customer Support",
    "marketing-intern": "Marketing Intern",
    "gtm-engineer": "GTM Engineer",
    "sales-sdr": "Sales SDR",
    "accounts-receivable": "Accounts Receivable",
    "finance-ar": "Accounts Receivable",
  };
  return (verticalId && labels[verticalId]) || "your template";
}

export default function OnboardingFirstHirePage() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setFirstHirePromptInStore = useOnboardingDraft((s) => s.setFirstHirePrompt);
  const reset = useOnboardingDraft((s) => s.reset);
  const verticalId = useOnboardingDraft((s) => s.verticalId);
  const firstHire = useFirstHire();
  const [error, setError] = useState("");
  const [showHelpLink, setShowHelpLink] = useState(false);

  useEffect(() => {
    setStep(5);
  }, [setStep]);

  const persona = useMemo(() => resolveCanonicalPersona(verticalId), [verticalId]);
  const tplLabel = templateLabel(verticalId);

  // The gateway requires a non-empty description (Genesis is real on
  // the backend even when the demo-mode SPA short-circuits the call).
  // Synthesise an honest description from the canonical persona so the
  // worker that lands on /workers/<id> has a real tagline to anchor the
  // "first-hire-derived persona" memory_seed audit row downstream.
  const seedDescription = useMemo(
    () =>
      `A ${persona.role.title.toLowerCase()} named ${persona.name} on the ${persona.role.team} team — ${persona.role.tagline}.`,
    [persona],
  );

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setError("");
    setShowHelpLink(false);
    if (!verticalId) {
      setError("Pick a template first.");
      return;
    }
    const { tenantId } = useOnboardingDraft.getState();
    setFirstHirePromptInStore(seedDescription);
    try {
      const res = await firstHire.mutateAsync({
        description: seedDescription,
        tenantId,
        verticalId,
      });
      reset();
      try {
        sessionStorage.setItem("skycot:just-hired", res.workerId);
      } catch {
        /* private mode / storage disabled — silently degrade */
      }
      router.push(`/workers/${res.workerId}`);
    } catch (e) {
      if (e instanceof Error && e.message === FIRST_HIRE_VERIFY_FAILED_MESSAGE) {
        setError(e.message);
        setShowHelpLink(true);
      } else {
        const scrubbed = scrubOnboardingErrorMessage(e);
        const suffix = scrubbed.wasProtocolLeak
          ? ""
          : " Try again, or come back once the gateway is up.";
        setError(`${scrubbed.message}${suffix}`);
      }
    }
  }

  const submitting = firstHire.isPending;

  return (
    <OnboardingShell
      step={5}
      completedSteps={4}
      stepLabel="FIRST HIRE"
      eyebrow="STEP 05 · YOUR FIRST TEAMMATE"
      heroSegments={[
        { text: "Meet " },
        { text: `${persona.name}.`, accent: true },
      ]}
      sub={
        <>
          Based on your <strong>{tplLabel}</strong> template, we've paired you
          with {persona.name} — {persona.role.tagline}. Ready to start working
          alongside you.
        </>
      }
    >
      <form
        className="ob-form-card"
        onSubmit={onSubmit}
        data-testid="onboarding-first-hire-form"
        noValidate
        style={{ maxWidth: 680 }}
      >
        <MeetTeammateCard persona={persona} />
        {error ? (
          <FirstHireErrorBanner message={error} showHelpLink={showHelpLink} />
        ) : null}
        <div className="ob-cta-row is-split">
          <button
            type="button"
            className="ob-cta-back"
            onClick={() => router.push("/onboarding/pick-vertical")}
            data-testid="onboarding-first-hire-back"
          >
            ← Back
          </button>
          <button
            type="submit"
            className="ob-cta"
            disabled={submitting || !verticalId}
            aria-busy={submitting || undefined}
            data-testid="onboarding-first-hire-submit"
          >
            {submitting ? (
              <Loader2 className="size-4 animate-spin" aria-hidden />
            ) : (
              <Sparkles className="size-4" aria-hidden />
            )}
            Hire {persona.name}
            <span className="ob-cta-kbd" aria-hidden>
              ↵
            </span>
          </button>
        </div>
        <p className="ob-meet-change">
          Want a different role?{" "}
          <button
            type="button"
            className="ob-meet-change-link"
            onClick={() => router.push("/onboarding/pick-vertical")}
          >
            Go back to template picker
          </button>
        </p>
      </form>
    </OnboardingShell>
  );
}

interface MeetTeammateCardProps {
  persona: SamplePersona;
}

function MeetTeammateCard({ persona }: MeetTeammateCardProps) {
  const personality = persona.personality;
  return (
    <div className="ob-meet-card" data-testid="onboarding-meet-card">
      <div className="ob-meet-header">
        <div className="ob-meet-avatar" aria-hidden>
          {persona.initials.charAt(0)}
        </div>
        <div className="ob-meet-who">
          <h2 className="ob-meet-name">
            {persona.name}
            <span className="ob-meet-pill">READY TO START</span>
          </h2>
          <p className="ob-meet-role">
            {persona.role.title}
            <span className="ob-meet-sep">·</span>
            {persona.role.team}
            <span className="ob-meet-sep">·</span>
            {persona.role.tagline}
          </p>
        </div>
      </div>

      <div className="ob-meet-traits">
        {persona.traits.map((t) => (
          <span key={t} className="ob-meet-trait">
            {t}
          </span>
        ))}
      </div>

      {personality ? (
        <div className="ob-meet-twocol">
          <div className="ob-meet-col ob-meet-col-great">
            <h3 className="ob-meet-coltitle">
              <span aria-hidden>✓</span> Great at
            </h3>
            <ul>
              {personality.greatAt.map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
          </div>
          <div className="ob-meet-col ob-meet-col-escalate">
            <h3 className="ob-meet-coltitle">
              <span aria-hidden>⚠</span> Escalate when
            </h3>
            <ul>
              {personality.escalateWhen.map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
          </div>
        </div>
      ) : null}

      {personality?.signatureBehaviors?.length ? (
        <div className="ob-meet-signature">
          <span className="ob-meet-signature-label">Signature behaviors</span>
          <span className="ob-meet-signature-body">
            {personality.signatureBehaviors.join(" · ")}
          </span>
        </div>
      ) : null}
    </div>
  );
}

// Suppress unused-import warning while we keep the type-only ref live.
void SAMPLE_PERSONAS;
