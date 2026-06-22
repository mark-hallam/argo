"use client";

import { useState } from "react";
import {
  AlertTriangle,
  ArrowRight,
  BookCheck,
  CheckCircle2,
  Clock,
  Loader2,
  ShieldCheck,
  ShieldAlert,
  Ban,
  Plug,
  Split,
  Target,
  FolderOpen,
} from "lucide-react";
import type {
  GenesisVerticalReadyEvent,
  RoleContract,
  RoleContractAction,
  SuccessCriterion,
} from "@/lib/api/genesis";
import { parseCadenceToCron } from "@/lib/api/genesis-cadence";
import { extractFsRootFromAccessNeeds } from "@/lib/api/genesis-hire-intake";

/**
 * Genesis P5 (c) — the operator "what can this teammate do" review screen
 * (DEC-025 #1, human-review-before-activation ON by default).
 *
 * Renders the TRUTHFUL `role_contract` the backend derived from the
 * teammate's ACTUAL bound capability profile (the same source the P2
 * capability fence reads) — never a benign restatement — and requires an
 * explicit operator confirmation before the hire fires. Confirming echoes the
 * `contract_digest` back at the hire call; the gateway re-derives the digest
 * from the bound manifest and activates ONLY on a match (a stale/forged
 * digest is hard-blocked), so the teammate cannot activate without the
 * operator seeing exactly what it can — and cannot — do.
 *
 * Brand: consumes the LOCKED design tokens (`--brand-gradient` purple→PINK,
 * `--brand-ink`, Audiowide display, Inter body) from `web/app/globals.css`.
 * No color literals, no new design — it extends the locked `/genesis`
 * onboarding-studio surface.
 */

export interface RoleContractReviewProps {
  ready: GenesisVerticalReadyEvent;
  /** Fires with the confirmed contract digest when the operator approves. */
  onConfirmHire(): void;
  isHiring: boolean;
}

function riskClass(risk: string): string {
  switch (risk.toLowerCase()) {
    case "high":
      return "rc-risk-high";
    case "medium":
      return "rc-risk-medium";
    default:
      return "rc-risk-low";
  }
}

/** Render a typed success criterion as one human-readable line. */
function formatCriterion(k: SuccessCriterion): string {
  const op = k.comparator === "gte" ? "≥" : k.comparator === "lte" ? "≤" : "=";
  return `${k.name}: ${op} ${k.target} (${k.kind})`;
}

function GateBadge({ ready }: { ready: GenesisVerticalReadyEvent }) {
  const verdict = ready.judge_verdict;
  const passed = verdict === "pass";
  const blocked = verdict === "blocked";
  const Icon = passed ? ShieldCheck : blocked ? Ban : ShieldAlert;
  const label = passed
    ? "Cleared safety gate"
    : blocked
      ? "Blocked by safety gate"
      : "Awaiting your review";
  const cls = passed ? "rc-gate-pass" : blocked ? "rc-gate-block" : "rc-gate-review";
  return (
    <span className={`rc-gate-badge ${cls}`} data-testid="genesis-gate-badge">
      <Icon size={13} aria-hidden /> {label}
    </span>
  );
}

/**
 * The moat moment (Genesis P6): when the operator attached an SOP, show that
 * the teammate was quizzed on it and how it scored. Only renders when a SOP was
 * actually uploaded + graded (`sop_uploaded`), so the text-only path is
 * unaffected and we never claim a comprehension check that didn't run.
 */
function SopBadge({ ready }: { ready: GenesisVerticalReadyEvent }) {
  if (!ready.sop_uploaded) return null;
  const gate = ready.gate;
  const total = gate?.sop_total ?? 0;
  const passed = gate?.sop_passed ?? 0;
  const allPassed = total > 0 && passed === total;
  const label =
    total > 0
      ? `Read your SOP · ${passed}/${total} comprehension questions`
      : "Read your SOP";
  const cls = allPassed ? "rc-gate-pass" : "rc-gate-review";
  return (
    <span
      className={`rc-gate-badge ${cls}`}
      data-testid="genesis-sop-badge"
      title={
        ready.sop_source_label
          ? `Ingested document: ${ready.sop_source_label}`
          : undefined
      }
    >
      <BookCheck size={13} aria-hidden /> {label}
    </span>
  );
}

function ActionRow({ action }: { action: RoleContractAction }) {
  return (
    <li className="rc-action" data-testid="genesis-contract-action">
      <div className="rc-action-head">
        <code className="rc-action-name">{action.action}</code>
        <span className={`rc-chip ${riskClass(action.risk)}`}>{action.risk} risk</span>
        {action.requires_approval ? (
          <span className="rc-chip rc-chip-approval">needs approval</span>
        ) : (
          <span className="rc-chip rc-chip-auto">autonomous</span>
        )}
        <span className="rc-chip rc-chip-executor">{action.executor}</span>
      </div>
      {action.description ? (
        <p className="rc-action-desc">{action.description}</p>
      ) : null}
    </li>
  );
}

/**
 * Genesis ⑤ "completeness preview" — the one panel that turns invisible
 * mid-task stalls into a hire-time checklist. Computed PURELY from the contract
 * the operator is already looking at (no new backend, no overclaim): the things
 * to CONNECT once at hire (bound-but-draft integrations ∪ access named at
 * intake) + an unparseable schedule, vs. the genuine run-time approvals that are
 * the GOOD interruptions ("only approvals remain" — the product vision). When
 * there's nothing to connect, it says so plainly: it can run end-to-end now.
 */
function ReadinessPreview({
  contract,
  scheduleNeedsConfirm,
}: {
  contract: RoleContract;
  scheduleNeedsConfirm: boolean;
}) {
  // CONNECT-AT-HIRE set: draft-only integrations (bound but can't transmit) plus
  // access named at intake but not yet bound. Dedup case-insensitively so the
  // same system captured both ways ("Gmail" intake / "gmail" draft) shows once.
  const seen = new Set<string>();
  const connectNeeded: string[] = [];
  for (const raw of [...contract.draft_integrations, ...(contract.access_needs ?? [])]) {
    const name = raw.trim();
    const key = name.toLowerCase();
    if (!name || seen.has(key)) continue;
    seen.add(key);
    connectNeeded.push(name);
  }
  const approvals = contract.approval_required ?? [];
  const nothingToClear = connectNeeded.length === 0 && !scheduleNeedsConfirm;

  return (
    <div className="rc-panel rc-panel-readiness" data-testid="genesis-readiness">
      <h4 className="rc-h4">
        <CheckCircle2 size={13} aria-hidden /> Readiness — clear once at hire, then only approvals
      </h4>
      {nothingToClear ? (
        <p className="rc-escalation" data-testid="genesis-readiness-allclear">
          Nothing to connect — with what you&apos;ve given it, {contract.role_name} can run
          end-to-end the moment you hire.{" "}
          {approvals.length > 0
            ? "The only interruptions you'll see are the genuine approvals below."
            : "It runs on its own within the boundaries above."}
        </p>
      ) : (
        <>
          <p className="rc-escalation">
            {contract.role_name} can start drafting and proposing immediately. To run all the way
            to delivery, clear these once — here, not mid-task:
          </p>
          <ul className="rc-tag-list" data-testid="genesis-readiness-connect">
            {connectNeeded.map((c) => (
              <li key={c}>
                Connect <strong>{c}</strong> — it can draft, but physically can&apos;t send or
                transmit through it until you do.
              </li>
            ))}
            {scheduleNeedsConfirm ? (
              <li data-testid="genesis-readiness-schedule">
                <strong>Confirm the schedule</strong> after hiring — its words didn&apos;t map to a
                fixed time, so it won&apos;t fire on a cadence until you set one.
              </li>
            ) : null}
          </ul>
        </>
      )}
      {approvals.length > 0 ? (
        <p className="rc-escalation" data-testid="genesis-readiness-approvals">
          After that, the only run-time interruptions are genuine approvals:{" "}
          {approvals.join(", ")} — everything else runs free.
        </p>
      ) : null}
    </div>
  );
}

export function RoleContractReview({
  ready,
  onConfirmHire,
  isHiring,
}: RoleContractReviewProps) {
  const [confirmed, setConfirmed] = useState(false);
  const contract: RoleContract | null | undefined = ready.role_contract;
  // The OPERATOR'S confirmation is the activation gate, not the judge verdict.
  // A fresh Genesis teammate is ALWAYS `needs_review` by design (human-review-
  // before-activation, DEC-025 #1) — never `"pass"` at birth — so gating the
  // Hire button on `=== "pass"` left it permanently disabled even after the
  // operator ticked confirm, while the API hire succeeded via the confirmed
  // contract-digest echo. So: a `needs_review` teammate IS hireable once the
  // operator confirms (the gateway re-derives + checks the digest server-side);
  // a hard-`blocked` teammate is NOT (the safety gate refused it outright).
  const hireable = ready.judge_verdict !== "blocked";

  // No contract derived (manifest unavailable) — surface honestly rather than
  // letting the operator confirm a black box.
  if (!contract) {
    return (
      <div className="rc-wrap">
        <div className="rc-card rc-card-empty" role="alert" data-testid="genesis-contract-empty">
          <AlertTriangle size={20} aria-hidden />
          <div>
            <strong>No reviewable role contract.</strong>
            <p className="rc-empty-sub">
              Genesis couldn&apos;t derive this teammate&apos;s capability
              contract, so it can&apos;t be confirmed for hire. Try regenerating.
            </p>
          </div>
        </div>
      </div>
    );
  }

  // Intake-context slots the operator captured (DEC-048 rule 2 — every captured
  // slot must be visible before hire, so the operator can catch any loss). The
  // cadence is checked against the SAME deterministic parser the hire path uses,
  // so the "needs confirmation" note shows exactly when the minted schedule
  // would fall back to a default rather than the operator's words.
  const cadence = contract.cadence?.trim() ?? "";
  const cadenceNeedsConfirm =
    cadence.length > 0 && !parseCadenceToCron(cadence).parsed;
  const exceptions = contract.exceptions ?? [];
  const successCriteria = contract.success_criteria ?? [];
  // Access the operator NAMED at intake (e.g. "Xero") — captured into the
  // contract but draft-only by design (digest-excluded). These are NOT bound
  // integrations; surfacing them keeps the review card from silently dropping
  // a slot the operator stated out loud, and stops the Integrations card from
  // reading as a contradiction of what they just asked for (DEC-048).
  const accessNeeds = contract.access_needs ?? [];
  // uf-bind-intake-access-folder-to-fs-root: when the captured Access needs
  // name a local folder, surface it so the operator can confirm the teammate's
  // working folder before hiring. Truthful + no overclaim: the gateway binds
  // this only if it resolves under the user's home folder (otherwise it falls
  // back to a generated default), which the caption states honestly.
  const workingFolder = extractFsRootFromAccessNeeds(contract.access_needs);

  return (
    <div className="rc-wrap" data-testid="genesis-role-contract-review">
      <div className="rc-card">
        <div className="rc-topline" aria-hidden />
        <div className="rc-header">
          <div>
            <p className="rc-eyebrow">Review before you hire</p>
            <h2 className="rc-name">{contract.role_name}</h2>
            {contract.does ? <p className="rc-does">{contract.does}</p> : null}
          </div>
          <div className="rc-badges">
            <GateBadge ready={ready} />
            <SopBadge ready={ready} />
          </div>
        </div>

        <div className="rc-grid">
          <section className="rc-col">
            <h3 className="rc-h3">What it can do</h3>
            <ul className="rc-actions">
              {contract.actions.map((a) => (
                <ActionRow key={a.action} action={a} />
              ))}
            </ul>
          </section>

          <aside className="rc-col rc-aside">
            {contract.approval_required.length > 0 ? (
              <div className="rc-panel">
                <h4 className="rc-h4">
                  <ShieldAlert size={13} aria-hidden /> Always needs your approval
                </h4>
                <ul className="rc-tag-list" data-testid="genesis-approval-required">
                  {contract.approval_required.map((a) => (
                    <li key={a}>{a}</li>
                  ))}
                </ul>
              </div>
            ) : null}

            {contract.autonomous.length > 0 ? (
              <div className="rc-panel">
                <h4 className="rc-h4">
                  <CheckCircle2 size={13} aria-hidden /> Runs on its own
                </h4>
                <ul className="rc-tag-list" data-testid="genesis-autonomous">
                  {contract.autonomous.map((a) => (
                    <li key={a}>{a}</li>
                  ))}
                </ul>
              </div>
            ) : null}

            <div className="rc-panel">
              <h4 className="rc-h4">
                <ShieldCheck size={13} aria-hidden /> Escalation
              </h4>
              <p className="rc-escalation">{contract.escalation}</p>
            </div>

            {cadence ? (
              <div className="rc-panel" data-testid="genesis-cadence">
                <h4 className="rc-h4">
                  <Clock size={13} aria-hidden /> When it runs
                </h4>
                <p className="rc-escalation">{cadence}</p>
                {cadenceNeedsConfirm ? (
                  <p
                    className="rc-escalation"
                    data-testid="genesis-cadence-needs-confirm"
                  >
                    Schedule needs confirmation — set it in Schedules after
                    hiring.
                  </p>
                ) : null}
              </div>
            ) : null}

            {exceptions.length > 0 ? (
              <div className="rc-panel" data-testid="genesis-exceptions">
                <h4 className="rc-h4">
                  <Split size={13} aria-hidden /> Exceptions / hand-offs
                </h4>
                <ul className="rc-tag-list">
                  {exceptions.map((e) => (
                    <li key={e}>{e}</li>
                  ))}
                </ul>
              </div>
            ) : null}

            {successCriteria.length > 0 ? (
              <div className="rc-panel" data-testid="genesis-success-criteria">
                <h4 className="rc-h4">
                  <Target size={13} aria-hidden /> How success is measured
                </h4>
                <ul className="rc-tag-list">
                  {successCriteria.map((k) => (
                    <li key={k.name}>{formatCriterion(k)}</li>
                  ))}
                </ul>
              </div>
            ) : null}

            <div className="rc-panel rc-panel-willnot">
              <h4 className="rc-h4">
                <Ban size={13} aria-hidden /> What it will NOT do
              </h4>
              <ul className="rc-willnot-list" data-testid="genesis-will-not">
                {contract.will_not.map((w) => (
                  <li key={w}>{w}</li>
                ))}
              </ul>
            </div>

            {workingFolder ? (
              <div className="rc-panel" data-testid="genesis-working-folder">
                <h4 className="rc-h4">
                  <FolderOpen size={13} aria-hidden /> Working folder
                </h4>
                <p className="rc-escalation">
                  <code>{workingFolder}</code>
                </p>
                <p className="rc-escalation">
                  Approved file work happens inside this folder. It binds only
                  if it&apos;s under your home folder; otherwise a default
                  teammate folder is used.
                </p>
              </div>
            ) : null}

            {accessNeeds.length > 0 ? (
              <div className="rc-panel" data-testid="genesis-access-needs">
                <h4 className="rc-h4">
                  <Plug size={13} aria-hidden /> Access requested at intake (not
                  yet connected)
                </h4>
                <ul className="rc-tag-list">
                  {accessNeeds.map((a) => (
                    <li key={a}>{a}</li>
                  ))}
                </ul>
              </div>
            ) : null}

            <div className="rc-panel">
              <h4 className="rc-h4">
                <Plug size={13} aria-hidden /> Integrations (draft-only at birth)
              </h4>
              {contract.draft_integrations.length > 0 ? (
                <ul className="rc-tag-list" data-testid="genesis-draft-integrations">
                  {contract.draft_integrations.map((d) => (
                    <li key={d}>{d}</li>
                  ))}
                </ul>
              ) : accessNeeds.length > 0 ? (
                <p className="rc-escalation">
                  None bound yet — {accessNeeds.join(", ")}{" "}
                  {accessNeeds.length === 1 ? "was" : "were"} requested at intake
                  and stay draft-only until you connect{" "}
                  {accessNeeds.length === 1 ? "it" : "them"}.
                </p>
              ) : (
                <p className="rc-escalation">
                  None bound — it can draft and propose, but physically cannot
                  transmit through any live integration until you grant one.
                </p>
              )}
            </div>
          </aside>
        </div>

        <ReadinessPreview contract={contract} scheduleNeedsConfirm={cadenceNeedsConfirm} />

        <div className="rc-confirm">
          <label className="rc-confirm-check">
            <input
              type="checkbox"
              checked={confirmed}
              onChange={(e) => setConfirmed(e.target.checked)}
              disabled={isHiring}
              data-testid="genesis-confirm-checkbox"
            />
            <span>
              I&apos;ve reviewed what {contract.role_name} can and can&apos;t do,
              and I&apos;m hiring them with this scope.
            </span>
          </label>
          <button
            type="button"
            className="rc-cta"
            onClick={onConfirmHire}
            disabled={!confirmed || !hireable || isHiring}
            data-testid="genesis-confirm-hire-button"
          >
            {isHiring ? (
              <>
                <Loader2 size={18} className="rc-spin" aria-hidden /> Hiring…
              </>
            ) : (
              <>
                Confirm &amp; hire {contract.role_name}{" "}
                <ArrowRight size={16} aria-hidden />
              </>
            )}
          </button>
          {!hireable ? (
            <p className="rc-blocked-note" data-testid="genesis-hire-held">
              The safety gate <strong>blocked</strong> this teammate, so it
              can&apos;t be hired — confirming won&apos;t override the block.
              Regenerate the role with a narrower scope.
            </p>
          ) : null}
        </div>
      </div>
    </div>
  );
}
