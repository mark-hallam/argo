"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { AlertTriangle, ShieldAlert } from "lucide-react";
import { EscalationMatrix, type EscalationRule } from "@/components/skycot/workers/EscalationMatrix";
import { TrustOverTimeChart } from "@/components/skycot/workers/TrustOverTimeChart";
import { useWorker } from "@/lib/hooks/use-worker";
import { useWorkerEscalations } from "@/lib/hooks/use-worker-escalations";
import { findSampleTrustHistory, REFERENCE_NOW } from "@/lib/sample-worker-trust-history";

/**
 * Worker escalation tab — `/workers/<id>/escalation`.
 *
 * The escalation RULES are now read live from
 * `GET /api/v1/workers/<id>/escalations` (`useWorkerEscalations` →
 * `fetchWorkerEscalations`): a real or demo-live operator sees the
 * teammate's REAL persisted pause-and-escalate rules, never fabricated
 * ones — closing the NO-THEATER leak on this SAFETY surface (DEC-007 /
 * DEC-029). The bundled per-worker `SEEDS` survive ONLY as the
 * anonymous-dev/offline fallback.
 *
 * The live GET exposes the rule SET, not a per-event escalation HISTORY
 * feed or a trust-over-time series, so the history table + trust chart
 * render their honest EMPTY states for the real/demo-live path (the
 * sample history + `findSampleTrustHistory` are kept only for the
 * anonymous-dev fallback). Inventing past escalation events for a real
 * operator would be the exact theater this surface must not show.
 */

interface EscalationHistoryEntry {
  /** Unix ms of when the escalation fired. */
  ts: number;
  /** Audit-row identifier so we can link to /audit/action/<id>. */
  auditId: string;
  /** One-line description of what happened. */
  summary: string;
  /** Who the escalation routed to. */
  routedTo: string;
  /** Was the escalation resolved (approved / rejected / actioned)? */
  outcome: "approved" | "rejected" | "pending";
}

interface WorkerEscalationSeed {
  rules: EscalationRule[];
  history: EscalationHistoryEntry[];
}

const DEFAULT_SEED: WorkerEscalationSeed = {
  rules: [
    {
      action: "Reply to a customer message",
      condition: "Confidence in the next action is below 80%",
      routesTo: "On-call operator",
      channel: "Slack #skycot-escalations",
    },
    {
      action: "Trigger a public or irreversible side-effect",
      condition: "Action would post publicly or move money",
      routesTo: "Worker owner",
      channel: "Email",
    },
    {
      action: "Continue when ambiguous",
      condition: "Multiple plausible interpretations exist",
      routesTo: "On-call operator",
    },
  ],
  history: [],
};

const SEEDS: Record<string, WorkerEscalationSeed> = {
  "019e6785-928a-7270-a766-f945b6cd7161": {
    rules: [
      {
        action: "Re-price a listing",
        condition: "Price drops below the seller's floor by more than 5%",
        routesTo: "Seller",
        channel: "Email",
      },
      {
        action: "Approve photos for a listing",
        condition: "Edited-photo detector flags the upload",
        routesTo: "Owner",
        channel: "Slack #marketplace-trust",
      },
      {
        action: "Respond to a buyer offer",
        condition: "Two or more competing offers within 30 minutes",
        routesTo: "Seller",
        channel: "SMS",
      },
      {
        action: "Change a listing's category",
        condition: "Category change would re-list under a fee tier",
        routesTo: "Owner",
      },
    ],
    history: [
      {
        ts: REFERENCE_NOW - 2 * 24 * 60 * 60 * 1000,
        auditId: "act_01J0ESC_MAR_001",
        summary: "Edited-photo signal on listing #482 — held for owner review",
        routedTo: "Owner",
        outcome: "approved",
      },
      {
        ts: REFERENCE_NOW - 9 * 24 * 60 * 60 * 1000,
        auditId: "act_01J0ESC_MAR_002",
        summary: "Competing offers on listing #461 — handed to seller",
        routedTo: "Seller",
        outcome: "approved",
      },
      {
        ts: REFERENCE_NOW - 23 * 24 * 60 * 60 * 1000,
        auditId: "act_01J0ESC_MAR_003",
        summary: "Price drop on listing #418 below the floor — rejected the re-price",
        routedTo: "Seller",
        outcome: "rejected",
      },
    ],
  },
  "019e6785-928d-78b1-a6b2-136d9ed263ce": {
    rules: [
      {
        action: "Send a rejection email",
        condition: "Score lands in the 50-65 band (ambiguous fit)",
        routesTo: "Hiring manager",
        channel: "Slack DM",
      },
      {
        action: "Schedule an interview",
        condition: "Candidate worked at a direct competitor in the last 12 months",
        routesTo: "Hiring manager",
        channel: "Email",
      },
      {
        action: "Score a candidate against a new JD",
        condition: "JD has been edited in the last 24 hours",
        routesTo: "Recruiter",
      },
    ],
    history: [
      {
        ts: REFERENCE_NOW - 1 * 24 * 60 * 60 * 1000,
        auditId: "act_01J0ESC_PRI_001",
        summary: "Candidate scored 58 against JD#92 — held for hiring-manager call",
        routedTo: "Hiring manager",
        outcome: "pending",
      },
      {
        ts: REFERENCE_NOW - 6 * 24 * 60 * 60 * 1000,
        auditId: "act_01J0ESC_PRI_002",
        summary: "Competitor-tenure flag on candidate #284 — manager approved continue",
        routedTo: "Hiring manager",
        outcome: "approved",
      },
    ],
  },
  "019e6785-928e-7f20-803b-55266e526011": {
    rules: [
      {
        action: "Close a customer ticket",
        condition: "Customer mentions churn or 'cancel' in any reply",
        routesTo: "Account manager",
        channel: "Slack #cs-escalations",
      },
      {
        action: "Issue a refund",
        condition: "Refund amount exceeds $200",
        routesTo: "Finance",
        channel: "Email",
      },
      {
        action: "Reply to a VIP customer",
        condition: "Customer is on a VIP or enterprise tier",
        routesTo: "Owner",
      },
    ],
    history: [],
  },
};

function getSeed(workerId: string): WorkerEscalationSeed {
  return SEEDS[workerId] ?? DEFAULT_SEED;
}

function fmtDateTime(ts: number): string {
  return new Date(ts).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

const OUTCOME_LABEL: Record<EscalationHistoryEntry["outcome"], string> = {
  approved: "Approved",
  rejected: "Rejected",
  pending: "Pending",
};

const OUTCOME_TONE: Record<EscalationHistoryEntry["outcome"], string> = {
  approved: "var(--success, #14b8a6)",
  rejected: "var(--destructive, #ef4444)",
  pending: "var(--warning, #f59e0b)",
};

export default function WorkerEscalationPage() {
  const routeParams = useParams<{ workerId: string }>();
  const workerId = routeParams?.workerId ?? "";
  const workerQuery = useWorker(workerId);
  const workerName = workerQuery.data?.name;

  const seed = getSeed(workerId);
  // Live escalation rules. The page's per-worker SEEDS are passed as the
  // ANONYMOUS-dev fallback only; a real/demo-live operator gets the live
  // rule set (or an honest error), never the sample.
  const escQuery = useWorkerEscalations(workerId, () => seed.rules);
  const result = escQuery.data;

  // Until the live read settles we render no rules (no fabrication on a
  // safety surface); anonymous dev resolves to the sample near-instantly
  // via the api module's bypass short-circuit.
  const rules = result?.policy.rules ?? [];

  // History + trust series have NO live feed. Show the sample ONLY when
  // the rules came from a mock source (anonymous dev/demo); a real or
  // demo-live operator (`source === "gateway"`) gets the honest empty
  // state, never fabricated past escalations (NO THEATER).
  const fromMock =
    result?.source === "mock-offline" || result?.source === "mock-bypass";
  const history = fromMock ? seed.history : [];
  const trustPoints = fromMock ? findSampleTrustHistory(workerId) : [];

  return (
    <div
      role="tabpanel"
      aria-label="Escalation tab"
      data-testid="worker-tab-panel-escalation"
      className="space-y-4"
    >
      <CurrentRulesCard workerName={workerName} rules={rules} />
      <EscalationMatrix workerName={workerName} rules={rules} />
      <TrustOverTimeChart points={trustPoints} workerName={workerName} />
      <HistoryCard entries={history} />
    </div>
  );
}

function CurrentRulesCard({
  workerName,
  rules,
}: {
  workerName: string | undefined;
  rules: EscalationRule[];
}) {
  const heading = workerName
    ? `${workerName} escalates on:`
    : "This worker escalates on:";
  return (
    <div data-testid="escalation-current-rules">
      <style jsx>{`
        .erc-card {
          background: var(--card);
          border: 1px solid var(--border, #ece9f5);
          border-radius: 14px;
          padding: 22px;
          position: relative;
          overflow: hidden;
        }
        .erc-card::before {
          content: "";
          position: absolute;
          top: 0;
          left: 0;
          width: 100%;
          height: 4px;
          background: linear-gradient(90deg, #f59e0b, #ec4899, #8b5cf6);
        }
        .erc-label {
          font-family: ui-monospace, "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: var(--muted-foreground, #52525b);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 10px;
        }
        .erc-title {
          margin: 0 0 14px;
          font-size: 15px;
          font-weight: 600;
          color: var(--foreground, #1e1b4b);
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .erc-list {
          list-style: none;
          padding: 0;
          margin: 0;
          font-size: 13.5px;
          color: var(--foreground, #1e1b4b);
          line-height: 1.6;
        }
        .erc-list li {
          display: flex;
          gap: 10px;
          padding: 8px 0;
          border-top: 1px solid var(--border, #ece9f5);
        }
        .erc-list li:first-child {
          border-top: none;
        }
        .erc-list li::before {
          content: "";
          flex-shrink: 0;
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: #f59e0b;
          margin-top: 9px;
        }
      `}</style>
      <div className="erc-card">
        <div className="erc-label">Current escalation rules</div>
        <h2 className="erc-title">
          <AlertTriangle size={16} style={{ color: "#f59e0b" }} aria-hidden />
          {heading}
        </h2>
        <ul className="erc-list">
          {rules.map((r, i) => (
            <li key={i} data-testid="escalation-rule">
              {r.condition}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

function HistoryCard({ entries }: { entries: EscalationHistoryEntry[] }) {
  return (
    <div data-testid="escalation-history">
      <style jsx>{`
        .eh-card {
          background: var(--card);
          border: 1px solid var(--border, #ece9f5);
          border-radius: 14px;
          padding: 22px;
        }
        .eh-label {
          font-family: ui-monospace, "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: var(--muted-foreground, #52525b);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 14px;
        }
        .eh-title {
          margin: 0 0 14px;
          font-size: 15px;
          font-weight: 600;
          color: var(--foreground, #1e1b4b);
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .eh-empty {
          font-size: 13px;
          color: var(--muted-foreground, #52525b);
        }
        .eh-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 13px;
        }
        .eh-table thead th {
          text-align: left;
          font-size: 10.5px;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          color: var(--muted-foreground, #52525b);
          font-weight: 700;
          padding: 0 8px 8px;
          border-bottom: 1px solid var(--border, #ece9f5);
        }
        .eh-table tbody td {
          padding: 12px 8px;
          vertical-align: top;
          color: var(--foreground, #1e1b4b);
          border-bottom: 1px solid var(--border, #ece9f5);
        }
        .eh-table tbody tr:last-child td {
          border-bottom: none;
        }
        .eh-outcome {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-weight: 600;
          font-size: 11.5px;
          letter-spacing: 0.05em;
          text-transform: uppercase;
        }
        .eh-link {
          color: #6d28d9;
          text-decoration: none;
          font-weight: 600;
        }
        .eh-link:hover {
          text-decoration: underline;
        }
      `}</style>
      <div className="eh-card">
        <div className="eh-label">Recent escalation history</div>
        <h2 className="eh-title">
          <ShieldAlert size={16} style={{ color: "#8b5cf6" }} aria-hidden />
          What this worker recently escalated
        </h2>
        {entries.length === 0 ? (
          <p className="eh-empty" data-testid="escalation-history-empty">
            No escalations recorded in the last 90 days.
          </p>
        ) : (
          <table className="eh-table">
            <thead>
              <tr>
                <th scope="col">When</th>
                <th scope="col">What</th>
                <th scope="col">Routed to</th>
                <th scope="col">Outcome</th>
                <th scope="col">Audit</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((entry) => (
                <tr key={entry.auditId} data-testid="escalation-history-row">
                  <td>{fmtDateTime(entry.ts)}</td>
                  <td>{entry.summary}</td>
                  <td>{entry.routedTo}</td>
                  <td>
                    <span
                      className="eh-outcome"
                      data-outcome={entry.outcome}
                      style={{ color: OUTCOME_TONE[entry.outcome] }}
                    >
                      {OUTCOME_LABEL[entry.outcome]}
                    </span>
                  </td>
                  <td>
                    <Link
                      className="eh-link"
                      href={`/audit/action/${encodeURIComponent(entry.auditId)}`}
                      data-testid="escalation-history-audit-link"
                    >
                      {entry.auditId}
                    </Link>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
