"use client";

import { useRouter } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import { apiPost, ApiError } from "@/lib/api/client";
import { useVerticals, type VerticalRow } from "@/lib/api/verticals";
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

/**
 * Client-side decoration keyed by vertical id: the gateway carries
 * id + display name + skill count, but `sampleDuties` (the 3-chip
 * "what does this teammate actually do?" line on each card) and a
 * compact one-sentence summary stay client-side because the manifest
 * `description` is too long for the card body and there's no
 * dedicated chips field. When the gateway returns a vertical we
 * don't have decoration for, we synthesise both from the gateway's
 * `headline` / `tags` so the card still renders.
 */
const VERTICAL_DECORATION: Record<
  string,
  { summary: string; sampleDuties: string[] }
> = {
  "gtm-engineer": {
    summary:
      "Go-to-market automation: lead enrichment, sequencing, signal triage.",
    sampleDuties: [
      "Enrich inbound leads",
      "Send Day-0 sequence",
      "Triage signal alerts",
    ],
  },
  "marketing-intern": {
    summary: "Content drafting, social posts, weekly competitive scans.",
    sampleDuties: [
      "Draft blog posts",
      "Schedule social posts",
      "Weekly competitive scan",
    ],
  },
  "marketplace-seller": {
    summary:
      "Inventory readiness, listing drafts, buyer replies, and price checks.",
    sampleDuties: [
      "Audit inventory",
      "Draft marketplace listings",
      "Negotiate offers",
    ],
  },
  recruiter: {
    summary:
      "Resume screening, candidate outreach, interview scheduling, and offer handoff.",
    sampleDuties: [
      "Score resumes",
      "Draft screening emails",
      "Schedule interviews",
    ],
  },
  "sales-sdr": {
    summary: "Outbound prospecting and meeting booking with approval gates.",
    sampleDuties: [
      "Research prospects",
      "Draft cold-outreach",
      "Book qualified meetings",
    ],
  },
  "customer-support": {
    summary: "Tier-1 ticket triage, FAQ replies, escalation routing.",
    sampleDuties: [
      "Triage new tickets",
      "Draft FAQ replies",
      "Escalate complex cases",
    ],
  },
  "finance-ar": {
    summary: "Invoicing, dunning emails, payment reconciliation.",
    sampleDuties: [
      "Create invoices",
      "Send dunning emails",
      "Reconcile payments",
    ],
  },
};

/**
 * Fallback card list used when the gateway catalogue is empty or
 * unreachable AND we're in demo / dev mode. Skill counts here match
 * what the on-disk packs install (parity with
 * `POST /api/v1/onboarding/install-vertical` `skills_added`).
 */
const FALLBACK_VERTICALS: ReadonlyArray<VerticalCard> = [
  {
    id: "gtm-engineer",
    displayName: "GTM Engineer",
    summary: VERTICAL_DECORATION["gtm-engineer"]!.summary,
    skillCount: 8,
    sampleDuties: VERTICAL_DECORATION["gtm-engineer"]!.sampleDuties,
  },
  {
    id: "marketing-intern",
    displayName: "Marketing Intern",
    summary: VERTICAL_DECORATION["marketing-intern"]!.summary,
    skillCount: 5,
    sampleDuties: VERTICAL_DECORATION["marketing-intern"]!.sampleDuties,
  },
  {
    id: "marketplace-seller",
    displayName: "Marketplace Seller",
    summary: VERTICAL_DECORATION["marketplace-seller"]!.summary,
    skillCount: 2,
    sampleDuties: VERTICAL_DECORATION["marketplace-seller"]!.sampleDuties,
  },
  {
    id: "recruiter",
    displayName: "Recruiter",
    summary: VERTICAL_DECORATION.recruiter!.summary,
    skillCount: 5,
    sampleDuties: VERTICAL_DECORATION.recruiter!.sampleDuties,
  },
  {
    id: "sales-sdr",
    displayName: "Sales SDR",
    summary: VERTICAL_DECORATION["sales-sdr"]!.summary,
    skillCount: 5,
    sampleDuties: VERTICAL_DECORATION["sales-sdr"]!.sampleDuties,
  },
  {
    id: "customer-support",
    displayName: "Customer Support",
    summary: VERTICAL_DECORATION["customer-support"]!.summary,
    skillCount: 5,
    sampleDuties: VERTICAL_DECORATION["customer-support"]!.sampleDuties,
  },
  {
    id: "finance-ar",
    displayName: "Accounts Receivable",
    summary: VERTICAL_DECORATION["finance-ar"]!.summary,
    skillCount: 5,
    sampleDuties: VERTICAL_DECORATION["finance-ar"]!.sampleDuties,
  },
];

/**
 * Project a gateway catalogue row into the wizard's card shape.
 * Client-side decoration (`summary` + `sampleDuties`) is merged in by
 * id; when no entry exists in `VERTICAL_DECORATION`, we synthesise
 * from the gateway `headline` + first few tags so unknown packs
 * still render rather than crashing the wizard.
 */
function cardFromRow(row: VerticalRow): VerticalCard {
  const decoration = VERTICAL_DECORATION[row.id];
  const summary = decoration?.summary ?? row.headline ?? row.description ?? "";
  const sampleDuties =
    decoration?.sampleDuties ??
    (row.tags ?? []).slice(0, 3).map((t) =>
      t.replace(/[-_]/g, " ").replace(/\b\w/g, (c) => c.toUpperCase()),
    );
  return {
    id: row.id,
    displayName: row.name || row.id,
    summary,
    skillCount: row.skill_count ?? 0,
    sampleDuties,
  };
}

export default function OnboardingPickVerticalPage() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setVertical = useOnboardingDraft((s) => s.setVertical);
  const initialVertical = useOnboardingDraft((s) => s.verticalId);
  const [selected, setSelected] = useState<string | null>(initialVertical);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const {
    verticals: catalogRows,
    source: catalogSource,
    isLoading: catalogLoading,
  } = useVerticals();

  useEffect(() => {
    setStep(4);
  }, [setStep]);

  // Cards rendered to the operator: gateway truth when the catalogue
  // returns any rows; otherwise the bundled fallback list. The
  // fallback's skill counts match `skills_added` from
  // `POST /api/v1/onboarding/install-vertical` so the wizard never
  // shows a number the gateway won't honour.
  const cards = useMemo<ReadonlyArray<VerticalCard>>(() => {
    if (catalogRows.length > 0) {
      return catalogRows.map(cardFromRow);
    }
    return FALLBACK_VERTICALS;
  }, [catalogRows]);

  async function onChoose(verticalId: string) {
    setSelected(verticalId);
    setError("");
    setBusy(true);
    const chosen = cards.find((c) => c.id === verticalId);
    setVertical(
      verticalId,
      chosen?.displayName ?? null,
      chosen?.skillCount ?? null,
    );
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
          {cards.map((v) => {
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
        <p
          className="ob-field-hint"
          data-testid="onboarding-vertical-source"
          aria-live="polite"
        >
          {catalogLoading
            ? "Loading catalogue…"
            : catalogSource === "gateway"
              ? `v1 catalogue · ${cards.length} verticals`
              : `v1 catalogue (offline · ${cards.length} verticals)`}
        </p>
        {error ? (
          <p className="ob-form-note" data-testid="onboarding-vertical-error">
            {error}
          </p>
        ) : null}
      </div>
    </OnboardingShell>
  );
}
