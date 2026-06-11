"use client";

import { useCallback, useState } from "react";
import { Check, Play, ShieldAlert } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { approveSessionTools } from "@/lib/api/session-approvals";
import { useToast } from "@/lib/ui/use-toast";

/**
 * "Needs your approval" card (day-1 unblock FIX 1).
 *
 * Rendered on the worker overview when a Run-now response comes back
 * with `held_tools` — the run happened, but the teammate's tools are
 * held by the 24-hour probation window pending a one-time human OK.
 * Without this card the held state was invisible: the SPA showed a
 * green "running now" toast and the only remediation was a CLI
 * command a web user can't run.
 *
 * Flow: list the held tools → [Approve tools] grants each one via
 * `POST /api/sessions/<id>/approve` → on success flip to a [Run
 * again] affordance that re-invokes Run now. Approve failure surfaces
 * the gateway error envelope message in a destructive toast and stays
 * retryable.
 *
 * Brand register: brand-restrained worker-overview card — existing
 * Card/Button idioms + CSS variables only (DESIGN_SYSTEM.md tokens).
 */

export interface ProbationApprovalCardProps {
  /** Display name for copy ("Ava ran, but…"). */
  workerName: string;
  /** Session the held run executed under. */
  sessionId: string;
  /** Tools held by the probation window — non-empty. */
  heldTools: string[];
  /** Re-invoke Run now after the tools are granted. */
  onRunAgain: () => void;
  className?: string;
}

type Phase = "idle" | "approving" | "approved";

export function ProbationApprovalCard({
  workerName,
  sessionId,
  heldTools,
  onRunAgain,
  className,
}: ProbationApprovalCardProps) {
  const toast = useToast();
  const [phase, setPhase] = useState<Phase>("idle");

  const onApprove = useCallback(async () => {
    if (phase === "approving") return;
    setPhase("approving");
    try {
      await approveSessionTools(sessionId, heldTools, {
        note: "approved from worker overview (probation window)",
      });
      setPhase("approved");
      toast.success("Tools approved", {
        description: `${workerName} can use ${heldTools.length === 1 ? "this tool" : "these tools"} now — run them again to do the work.`,
      });
    } catch (err) {
      setPhase("idle");
      toast.error("Couldn't approve the tools", {
        description: err instanceof Error ? err.message : undefined,
      });
    }
  }, [phase, sessionId, heldTools, workerName, toast]);

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
            {phase === "approved" ? (
              <Check className="size-3.5" />
            ) : (
              <ShieldAlert className="size-3.5" />
            )}
          </span>
          {phase === "approved" ? "Tools approved" : "Needs your approval"}
        </CardTitle>
      </CardHeader>
      <CardBody className="pt-1 space-y-3">
        <p className="text-sm text-[var(--muted-foreground)]">
          {phase === "approved" ? (
            <>
              {workerName} can use{" "}
              {heldTools.length === 1 ? "this tool" : "these tools"} now. Run
              them again to do the work.
            </>
          ) : (
            <>
              {workerName} ran, but these tools are held. New teammates work
              under a 24-hour probation window — every tool needs your
              one-time OK.
            </>
          )}
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
          {phase === "approved" ? (
            <Button
              type="button"
              variant="primary"
              size="md"
              onClick={onRunAgain}
              data-testid="probation-run-again"
            >
              <Play aria-hidden="true" />
              Run again
            </Button>
          ) : (
            <Button
              type="button"
              variant="primary"
              size="md"
              onClick={onApprove}
              loading={phase === "approving"}
              data-testid="probation-approve-tools"
            >
              Approve {heldTools.length === 1 ? "tool" : "tools"}
            </Button>
          )}
        </div>
      </CardBody>
    </Card>
  );
}
