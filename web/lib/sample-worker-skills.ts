/**
 * Mock per-worker Skills payloads. Three sections per worker —
 * Active / Library / Learned — keyed to the 6 personas in
 * `sample-workers-detail.ts`. The Skills tab uses these as fallback
 * data while the gateway endpoints land via the Track-F siblings
 * `mission-skill-store-available-for-worker` and
 * `mission-skill-publish-verified-ingest`.
 *
 * Enterprise-flavored framing per the brief — names read like real
 * marketplace skills (`senior-eng-screening-v3`,
 * `invoice-line-item-extractor-v5`), trust scores span the badge
 * tones (≥90 success / 70-90 warning / <70 destructive), and at least
 * one persona surfaces an empty Learned section so the empty-state
 * copy is exercised.
 */

import type {
  ActiveSkill,
  LearnedPattern,
  LibrarySkill,
  WorkerSkills,
} from "@/lib/api/worker-skills";

function active(
  id: string,
  name: string,
  version: string,
  trustScore: number,
  usageCount7d: number,
): ActiveSkill {
  return { id, name, version, trustScore, usageCount7d };
}

function library(
  id: string,
  name: string,
  version: string,
  trustScore: number,
  source: LibrarySkill["source"],
): LibrarySkill {
  return { id, name, version, trustScore, source };
}

function learned(
  id: string,
  label: string,
  successCount: number,
  failCount: number,
  sampleOutputs: string[],
): LearnedPattern {
  return { id, label, successCount, failCount, sampleOutputs };
}

const PRIYA: WorkerSkills = {
  active: [
    active("sk-priya-screen", "senior-eng-screening", "v3.1.0", 94, 128),
    active("sk-priya-slot", "interview-slot-proposer", "v2.0.4", 87, 42),
    active("sk-priya-reject", "warm-rejection-drafter", "v4.2.1", 91, 31),
  ],
  library: [
    library("sk-priya-comp", "compensation-research", "v2.0.0", 88, "workspace"),
    library("sk-priya-refs", "references-checker", "v1.0.3", 65, "marketplace"),
  ],
  learned: [
    learned(
      "lp-priya-decline",
      "auto-decline-incomplete-applications",
      8,
      0,
      [
        "Thanks for applying. We need a complete portfolio link before we can review your application — please reply with one and we'll pick this back up.",
        "Hi Maya — your CV came through but the role asks for at least 5 years of Rust experience. We can revisit if that changes.",
        "Hello! Your application is missing the work-authorization confirmation. Could you confirm your status? We'll hold the slot for 48h.",
      ],
    ),
  ],
};

const SAM: WorkerSkills = {
  active: [
    active("sk-sam-extract", "invoice-line-item-extractor", "v5.2.0", 96, 312),
    active("sk-sam-dupes", "duplicate-invoice-detector", "v3.0.1", 92, 47),
  ],
  library: [
    library("sk-sam-po", "purchase-order-matcher", "v2.1.0", 89, "workspace"),
    library("sk-sam-vendor", "vendor-info-validator", "v1.0.2", 81, "shared"),
  ],
  learned: [
    learned(
      "lp-sam-rush",
      "flag-Q4-rush-invoices",
      12,
      0,
      [
        "Flagged INV-2024-08741 — vendor 'Acme Tech' historically issues Q4 catch-up invoices in November. Holding for Controller review.",
        "Flagged INV-2024-08760 — same vendor + $25k+ over usual cadence. Q4 pattern. Routing to Controller.",
        "Flagged INV-2024-08772 — Q4 rush profile (vendor + amount + October submission). Awaiting approval.",
      ],
    ),
  ],
};

const MARIAH: WorkerSkills = {
  active: [
    active("sk-mariah-rfp", "rfp-response-drafter", "v4.0.0", 89, 18),
    active("sk-mariah-renew", "renewal-checklist-runner", "v3.1.2", 93, 6),
  ],
  library: [
    library("sk-mariah-redline", "contract-redline-suggester", "v1.0.0", 68, "marketplace"),
  ],
  learned: [],
};

const JAMAL: WorkerSkills = {
  active: [
    active("sk-jamal-resolve", "tier1-ticket-resolver", "v6.0.2", 95, 412),
    active("sk-jamal-route", "ticket-routing-classifier", "v2.3.0", 88, 188),
  ],
  library: [
    library("sk-jamal-refund", "refund-eligibility-checker", "v1.1.0", 84, "workspace"),
    library("sk-jamal-csat", "csat-followup-scheduler", "v1.0.0", 72, "shared"),
  ],
  learned: [
    learned(
      "lp-jamal-onhold",
      "auto-pause-on-hold-tickets",
      6,
      1,
      [
        "Paused ticket SUP-29382 — customer asked to come back next week. Set reminder for Mon 09:00.",
        "Paused ticket SUP-29401 — awaiting customer device logs. Auto-resume on inbound reply.",
        "Paused ticket SUP-29440 — customer travelling, asked to defer. Set reminder for Fri.",
      ],
    ),
  ],
};

const NORI: WorkerSkills = {
  active: [
    active("sk-nori-quote", "quote-pricing-validator", "v3.2.0", 92, 64),
    active("sk-nori-deal", "deal-desk-router", "v2.0.0", 86, 11),
  ],
  library: [
    library("sk-nori-comp", "competitor-pricing-lookup", "v1.0.1", 78, "marketplace"),
  ],
  learned: [],
};

const RIO: WorkerSkills = {
  active: [],
  library: [
    library("sk-rio-show", "showing-slot-proposer", "v1.0.0", 81, "workspace"),
  ],
  learned: [],
};

export const SAMPLE_WORKER_SKILLS: Record<string, WorkerSkills> = {
  wkr_01J0PRIYA: PRIYA,
  wkr_01J0SAM: SAM,
  wkr_01J0MARIAH: MARIAH,
  wkr_01J0JAMAL: JAMAL,
  wkr_01J0NORI: NORI,
  wkr_01J0RIO: RIO,
};

export function findSampleWorkerSkills(workerId: string): WorkerSkills | undefined {
  return SAMPLE_WORKER_SKILLS[workerId];
}
