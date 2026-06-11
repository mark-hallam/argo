"use client";

import { useCallback, useState } from "react";
import { Play, ShieldAlert } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";

/**
 * "Needs your approval" card — now ONE atomic action (keystone fix).
 *
 * Rendered on the worker overview when a Run-now response comes back
 * with `held_tools` — the run happened, but the teammate's tools are
 * held by the 24-hour probation window pending a one-time human OK.
 *
 * The old two-step flow (approve the FINISHED run's session, then "Run
 * again") never converged: grants are session-scoped, every run mints
 * a fresh session, so the next run held the SAME tools again. The
 * primary button is now **Approve & run**: one call to run-now with
 * `approve_tools = heldTools`, so the gateway mints ONE-SHOT grants on
 * the NEW run's session before the turn — the approved tools actually
 * fire and the teammate does real work.
 *
 * The parent owns the run call (`onApproveAndRun`) and clears/refreshes
 * this card from the new run's response: converged → card unmounts and
 * the success toast points at the audit trail; new tools held → the
 * card re-renders with the new set (progressive approval).
 *
 * Direct session-grant approval (`lib/api/session-approvals.ts`) still
 * exists for other surfaces; this card no longer routes through it.
 *
 * Brand register: brand-restrained worker-overview card — existing
 * Card/Button idioms + CSS variables only (DESIGN_SYSTEM.md tokens).
 */

export interface ProbationApprovalCardProps {
  /** Display name for copy ("Ava ran, but…"). */
  workerName: string;
  /** Tools held by the probation window — non-empty. */
  heldTools: string[];
  /**
   * Run again WITH `approve_tools = heldTools` — one atomic operator
   * action. The promise resolves when the run round-trips; the parent
   * updates/clears the card from the response.
   */
  onApproveAndRun: (tools: string[]) => Promise<void>;
  className?: string;
}

type Phase = "idle" | "running";

export function ProbationApprovalCard({
  workerName,
  heldTools,
  onApproveAndRun,
  className,
}: ProbationApprovalCardProps) {
  const [phase, setPhase] = useState<Phase>("idle");

  const onApprove = useCallback(async () => {
    if (phase === "running") return;
    setPhase("running");
    try {
      await onApproveAndRun(heldTools);
    } finally {
      setPhase("idle");
    }
  }, [phase, heldTools, onApproveAndRun]);

  return (
    <Card
      data-testid="probation-approval-card"
      data-phase={phase}
      className={className}
    >
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <span
            aria-hidden="true"
            className="grid size-6 place-items-center rounded-full bg-[var(--primary)]/10 text-[var(--primary)]"
          >
            <ShieldAlert className="size-3.5" />
          </span>
          Needs your approval
        </CardTitle>
      </CardHeader>
      <CardBody className="pt-1 space-y-3">
        <p className="text-sm text-[var(--muted-foreground)]">
          {workerName} ran, but these tools are held. New teammates work
          under a 24-hour probation window — every tool needs your one-time
          OK. Approving runs them again with your OK applied to that run.
        </p>
        <ul
          role="list"
          data-testid="probation-held-tools"
          className="flex flex-wrap gap-1.5"
        >
          {heldTools.map((tool) => (
            <li
              key={tool}
              data-testid={`held-tool-${tool}`}
              className="rounded-full border border-[var(--border)] bg-[var(--muted)]/40 px-2.5 py-1 font-mono text-[11px] text-[var(--foreground)]"
            >
              {tool}
            </li>
          ))}
        </ul>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="primary"
            size="md"
            onClick={onApprove}
            loading={phase === "running"}
            data-testid="probation-approve-and-run"
          >
            <Play aria-hidden="true" />
            Approve &amp; run
          </Button>
        </div>
      </CardBody>
    </Card>
  );
}
