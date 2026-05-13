/**
 * Audit drill-in type lock for E5-c (CONTRACT).
 *
 * `AuditDetail` extends the locked `AuditRow` primitive (E5-a) with
 * the four detail-card payloads the wireframe renders below the
 * summary hero: inputs, decision chain, provenance, output. Anything
 * that re-imports this type — the eventual sibling backend brief
 * `mission-gateway-audit-detail-endpoint`, E5-d (replay), E5-e
 * (single-action export), E5-f (hash verifier) — depends on these
 * field names. Re-shaping is a CONTRACT break that requires
 * re-cutting E5.
 *
 * v1 vs target (see streams/E5-c-audit-action-drillin.md):
 *  - Source: v1 reads from `GET /api/v1/audit/<actionId>` with a
 *    dev-only mock fallback. Target: server endpoint mandatory in
 *    prod, RBAC-aware redaction, tenant-scoped.
 *  - Hashes: v1 renders precomputed hashes verbatim. Target: a
 *    client-side verifier walks the chain (E5-f).
 *  - Computer-use screenshots: v1 ships the renderer; target keeps
 *    per-step capture / lightbox.
 *  - Cross-system jump links: v1 uses static URL prefix detection
 *    (Stripe, Slack, GitHub, Linear); target moves the generators
 *    into per-connector manifests.
 */
import type { AuditRow } from "@/lib/types/audit";

/** One step in the locked 8-step decision-chain trace. `details`
 *  payloads are free-form per step — the renderer collapses them
 *  into an expandable `<details>` block. */
export interface DecisionStep {
  n: number;
  title: string;
  body: string;
  /** Optional expandable JSON payload (renders under "Show details"). */
  details?: Record<string, unknown>;
}

/** Provenance row identities and hashes. Keep the field names stable
 *  — the `Copy hash chain` button emits a canonical JSON payload
 *  built from this same shape (see `provenanceClipboardPayload`). */
export interface ProvenanceBlockData {
  model: { id: string; version: string };
  promptTemplate: { id: string; hash: string };
  skill: { id: string; version: string };
  memorySnapshot: { id: string };
  policy: { id: string; version: string; hash: string };
}

/** Output card payload — kind discriminates the renderer. */
export type AuditOutputKind = "text" | "json" | "computer-use";

export interface ScreenshotFrame {
  url: string;
  timestamp: string;
  description?: string;
}

export interface AuditOutput {
  kind: AuditOutputKind;
  /** Plain text for `text` / `computer-use`, structured object for `json`. */
  body: string | Record<string, unknown>;
  /** Computer-use only: ordered screenshot sequence. */
  screenshots?: ScreenshotFrame[];
  /** Downstream delivery / receipt confirmation, if available. */
  delivery?: {
    sentAt: string;
    receivedAt?: string;
    downstreamId?: string;
  };
  /** Linked follow-up action — buyer reply, customer response, etc. */
  followUp?: { id: string; verb: string };
}

/** Drill-in trigger kinds — what set this action off. */
export type AuditTriggerKind = "message" | "cron" | "dependent_action";

export interface AuditDetailInputs {
  triggerKind: AuditTriggerKind;
  triggerSummary: string;
  /** Predecessor action — drill-in renders a back-link to its page. */
  predecessor?: { id: string; verb: string };
}

/** Full drill-in payload. Locked by the brief; do not re-shape. */
export interface AuditDetail extends AuditRow {
  inputs: AuditDetailInputs;
  decisionChain: DecisionStep[];
  provenance: ProvenanceBlockData;
  output: AuditOutput;
}

/** Detail-source provenance — mirrors `AuditFeedSource` from E5-a. */
export type AuditDetailSource = "gateway" | "mock-offline" | "mock-bypass";

/** Canonical JSON shape for the `[ Copy hash chain ]` clipboard payload.
 *  Lives on the type module so E5-f (verifier) can re-import it as the
 *  CONTRACT input. Field order is preserved at serialization time via
 *  `provenanceClipboardPayload()` below. */
export interface ProvenanceClipboard {
  actionId: string;
  timestamp: string;
  model: { id: string; version: string };
  promptTemplate: { id: string; hash: string };
  skill: { id: string; version: string };
  memorySnapshot: { id: string };
  policy: { id: string; version: string; hash: string };
}

/** Build the canonical clipboard payload for the provenance card.
 *  Field order matches the brief's documented JSON template — the
 *  E5-f hash verifier and any operator-side audit-replay scripts
 *  rely on byte-identical reproduction. */
export function provenanceClipboardPayload(detail: AuditDetail): ProvenanceClipboard {
  return {
    actionId: detail.id,
    timestamp: detail.timestamp,
    model: detail.provenance.model,
    promptTemplate: detail.provenance.promptTemplate,
    skill: detail.provenance.skill,
    memorySnapshot: detail.provenance.memorySnapshot,
    policy: detail.provenance.policy,
  };
}

/** Cross-system jump-link descriptor (Stripe / Slack / GitHub /
 *  Linear). v1 detection is URL-prefix based; v2 moves to
 *  per-connector manifest generators. */
export interface CrossSystemLink {
  system: "stripe" | "slack" | "github" | "linear";
  label: string;
  href: string;
}

/** Detect any cross-system jump links in a freeform text payload. v1
 *  rules per the brief — extend via per-connector manifests in v2.
 *  Returns links in the order they appear; dedupe by href. */
export function detectCrossSystemLinks(text: string): CrossSystemLink[] {
  const rules: Array<{ system: CrossSystemLink["system"]; prefix: string; label: string }> = [
    { system: "stripe", prefix: "https://dashboard.stripe.com/", label: "View in Stripe" },
    { system: "slack", prefix: "https://slack.com/archives/", label: "View in Slack" },
    { system: "github", prefix: "https://github.com/", label: "View in GitHub" },
    { system: "linear", prefix: "https://app.linear.app/", label: "View in Linear" },
  ];
  const URL_RE = /https?:\/\/[^\s<>"']+/g;
  const out: CrossSystemLink[] = [];
  const seen = new Set<string>();
  for (const match of text.matchAll(URL_RE)) {
    const href = match[0];
    if (seen.has(href)) continue;
    const rule = rules.find((r) => href.startsWith(r.prefix));
    if (!rule) continue;
    if (rule.system === "github") {
      const tail = href.slice(rule.prefix.length);
      if (!/^[^/]+\/[^/]+\/(issues|pull)\/\d+/.test(tail)) continue;
    }
    seen.add(href);
    out.push({ system: rule.system, label: rule.label, href });
  }
  return out;
}
