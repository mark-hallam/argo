"use client";

import { ArrowRight, UserCircle2 } from "lucide-react";

/**
 * When-to-escalate matrix for the worker's escalation tab.
 *
 * Each row maps an `action` (something the worker would do) ×
 * `condition` (a signal that fires) to a `routesTo` human owner.
 * Pure presentation; the rules are seeded server-side in the page.
 *
 * Design language matches the C2 worker-detail register: white
 * card, mono section labels, condition pills, warm purple bullets.
 */

export interface EscalationRule {
  /** The action the worker would otherwise auto-take. */
  action: string;
  /** Plain-English condition that should pause the worker. */
  condition: string;
  /** Human owner the escalation routes to. */
  routesTo: string;
  /** Optional Slack/email channel hint. */
  channel?: string;
}

export interface EscalationMatrixProps {
  rules: EscalationRule[];
  workerName?: string;
  className?: string;
}

export function EscalationMatrix({
  rules,
  workerName,
  className,
}: EscalationMatrixProps) {
  const heading = workerName
    ? `When ${workerName} escalates`
    : "When this worker escalates";

  return (
    <div className={className} data-testid="escalation-matrix">
      <style jsx>{`
        .em-card {
          background: white;
          border: 1px solid var(--border, #ece9f5);
          border-radius: 14px;
          padding: 22px;
        }
        .em-label {
          font-family: ui-monospace, "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 14px;
        }
        .em-title {
          margin: 0 0 14px;
          font-size: 15px;
          font-weight: 600;
          color: var(--foreground, #1e1b4b);
        }
        .em-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 13.5px;
        }
        .em-table thead th {
          text-align: left;
          font-size: 10.5px;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          color: var(--muted-foreground, #71717a);
          font-weight: 700;
          padding: 0 8px 8px;
          border-bottom: 1px solid var(--border, #ece9f5);
        }
        .em-table tbody td {
          padding: 12px 8px;
          vertical-align: top;
          color: var(--foreground, #1e1b4b);
          border-bottom: 1px solid var(--border, #ece9f5);
        }
        .em-table tbody tr:last-child td {
          border-bottom: none;
        }
        .em-action {
          font-weight: 600;
          color: var(--foreground, #1e1b4b);
        }
        .em-condition {
          color: var(--muted-foreground, #3f3f46);
        }
        .em-routes {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 4px 10px;
          border-radius: 999px;
          background: rgba(139, 92, 246, 0.08);
          color: #6d28d9;
          font-weight: 600;
          font-size: 12.5px;
          white-space: nowrap;
        }
        .em-channel {
          margin-top: 4px;
          font-size: 11.5px;
          color: var(--muted-foreground, #71717a);
        }
        .em-arrow {
          color: #c4b5fd;
        }
      `}</style>

      <div className="em-card">
        <div className="em-label">When-to-escalate matrix</div>
        <h3 className="em-title">{heading}</h3>
        <table className="em-table">
          <thead>
            <tr>
              <th scope="col">Action</th>
              <th scope="col">Condition</th>
              <th scope="col" aria-label="Routes to" />
              <th scope="col">Routes to</th>
            </tr>
          </thead>
          <tbody>
            {rules.map((rule, i) => (
              <tr key={`${rule.action}-${i}`} data-testid="escalation-matrix-row">
                <td className="em-action">{rule.action}</td>
                <td className="em-condition">{rule.condition}</td>
                <td aria-hidden="true">
                  <ArrowRight className="em-arrow" size={14} />
                </td>
                <td>
                  <span
                    className="em-routes"
                    data-testid="escalation-matrix-routes-to"
                  >
                    <UserCircle2 size={12} aria-hidden="true" />
                    {rule.routesTo}
                  </span>
                  {rule.channel ? (
                    <div className="em-channel">via {rule.channel}</div>
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
