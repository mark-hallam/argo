"use client";

import { useState } from "react";
import {
  AlertTriangle,
  ArrowRight,
  BookCheck,
  CheckCircle2,
  Loader2,
  ShieldCheck,
  ShieldAlert,
  Ban,
  Plug,
} from "lucide-react";
import type {
  GenesisVerticalReadyEvent,
  RoleContract,
  RoleContractAction,
} from "@/lib/api/genesis";

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
        <style jsx>{styles}</style>
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

  return (
    <div className="rc-wrap" data-testid="genesis-role-contract-review">
      <style jsx>{styles}</style>
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
              ) : (
                <p className="rc-escalation">
                  None bound — it can draft and propose, but physically cannot
                  transmit through any live integration until you grant one.
                </p>
              )}
            </div>
          </aside>
        </div>

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

// Locked-token styles. Every color is a `var(--token)` from globals.css
// (purple→PINK gradient, brand ink, Audiowide display, Inter body, semantic
// success/warning/destructive) — no literals, no new palette.
const styles = `
  .rc-wrap {
    max-width: 920px;
    margin: 36px auto 0;
    padding: 0 24px;
  }
  .rc-card {
    position: relative;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 20px 48px -24px color-mix(in srgb, var(--brand-ink) 22%, transparent);
    padding: 32px;
    overflow: hidden;
  }
  .rc-card-empty {
    display: flex;
    gap: 14px;
    align-items: flex-start;
    color: var(--destructive);
  }
  .rc-empty-sub {
    margin: 6px 0 0;
    color: var(--muted-foreground);
    font-size: 13.5px;
  }
  .rc-topline {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 4px;
    background: var(--brand-gradient);
  }
  .rc-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 20px;
    flex-wrap: wrap;
  }
  .rc-eyebrow {
    font-family: var(--font-mono);
    font-size: 10.5px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    font-weight: 600;
    color: var(--brand-primary);
    margin: 0 0 8px;
  }
  .rc-name {
    font-family: var(--font-audiowide);
    font-size: 28px;
    font-weight: 400;
    color: var(--brand-ink);
    margin: 0;
    line-height: 1.12;
  }
  .rc-does {
    margin: 8px 0 0;
    color: var(--muted-foreground);
    font-size: 14.5px;
    line-height: 1.55;
    max-width: 56ch;
  }
  .rc-badges {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
  }
  .rc-gate-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    padding: 5px 11px;
    border-radius: 999px;
    white-space: nowrap;
  }
  .rc-gate-pass {
    color: var(--success-foreground);
    background: var(--success);
  }
  .rc-gate-review {
    color: var(--warning-foreground);
    background: var(--warning);
  }
  .rc-gate-block {
    color: var(--destructive-foreground);
    background: var(--destructive);
  }
  .rc-grid {
    margin-top: 26px;
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
    gap: 26px;
  }
  @media (max-width: 760px) {
    .rc-grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .rc-h3 {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    font-weight: 600;
    color: var(--brand-primary);
    margin: 0 0 12px;
  }
  .rc-h4 {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--brand-ink);
    margin: 0 0 8px;
  }
  .rc-actions {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .rc-action {
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: color-mix(in srgb, var(--brand-primary) 4%, var(--card));
  }
  .rc-action-head {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .rc-action-name {
    font-family: var(--font-mono);
    font-size: 13px;
    font-weight: 600;
    color: var(--brand-ink);
  }
  .rc-action-desc {
    margin: 7px 0 0;
    font-size: 13px;
    color: var(--muted-foreground);
    line-height: 1.45;
  }
  .rc-chip {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--border);
    color: var(--muted-foreground);
  }
  .rc-chip-approval {
    color: var(--warning-foreground);
    background: var(--warning);
    border-color: transparent;
  }
  .rc-chip-auto {
    color: var(--success-foreground);
    background: var(--success);
    border-color: transparent;
  }
  .rc-chip-executor {
    color: var(--brand-primary);
    border-color: color-mix(in srgb, var(--brand-primary) 40%, transparent);
  }
  .rc-risk-high {
    color: var(--destructive-foreground);
    background: var(--destructive);
    border-color: transparent;
  }
  .rc-risk-medium {
    color: var(--warning-foreground);
    background: var(--warning);
    border-color: transparent;
  }
  .rc-risk-low {
    color: var(--muted-foreground);
  }
  .rc-aside {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .rc-panel {
    padding: 14px 16px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--card);
  }
  .rc-panel-willnot {
    border-color: color-mix(in srgb, var(--destructive) 35%, var(--border));
  }
  .rc-tag-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .rc-tag-list li {
    font-family: var(--font-mono);
    font-size: 11.5px;
    padding: 3px 9px;
    border-radius: 6px;
    border: 1px solid var(--border);
    color: var(--brand-ink);
    background: color-mix(in srgb, var(--brand-primary) 4%, var(--card));
  }
  .rc-escalation {
    margin: 0;
    font-size: 13px;
    color: var(--muted-foreground);
    line-height: 1.5;
  }
  .rc-willnot-list {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .rc-willnot-list li {
    font-size: 13px;
    color: var(--foreground);
    line-height: 1.45;
  }
  .rc-confirm {
    margin-top: 26px;
    padding-top: 22px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .rc-confirm-check {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    font-size: 14px;
    color: var(--foreground);
    line-height: 1.45;
    cursor: pointer;
  }
  .rc-confirm-check input {
    margin-top: 3px;
    width: 18px;
    height: 18px;
    accent-color: var(--brand-primary);
    cursor: pointer;
  }
  .rc-cta {
    align-self: flex-start;
    min-height: 48px;
    padding: 0 28px;
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-family: var(--font-sans);
    font-size: 15.5px;
    font-weight: 700;
    color: var(--brand-primary-foreground);
    background: var(--brand-gradient);
    border: none;
    border-radius: var(--radius-lg);
    cursor: pointer;
    box-shadow: 0 10px 28px -10px color-mix(in srgb, var(--brand-primary) 55%, transparent);
  }
  .rc-cta:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    box-shadow: none;
  }
  .rc-blocked-note {
    margin: 0;
    font-size: 13px;
    color: var(--muted-foreground);
  }
  .rc-spin {
    animation: rc-spin-kf 1s linear infinite;
  }
  @keyframes rc-spin-kf {
    to {
      transform: rotate(360deg);
    }
  }
`;
