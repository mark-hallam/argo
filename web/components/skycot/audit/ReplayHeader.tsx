import { ShieldCheck, AlertOctagon } from "lucide-react";
import { VerdictPill } from "@/components/skycot/audit/VerdictPill";
import type { Verdict } from "@/lib/api/replay";

/**
 * Replay header — the distinguishing tint band that frames the
 * sandbox surface, the "Sandbox — no side effects" reassurance
 * banner, and (when present) the verdict pill on the right.
 *
 * Misaligned verdicts get an additional red banner above the header
 * explaining that the operator was notified — this satisfies the
 * brief's "Top of page: red banner explaining …" acceptance criterion.
 */

export interface ReplayHeaderProps {
  actionId: string;
  verdict?: Verdict;
}

export function ReplayHeader({ actionId, verdict }: ReplayHeaderProps) {
  return (
    <div className="space-y-3">
      {verdict === "drifted-misaligned" ? (
        <div
          role="alert"
          data-testid="replay-misaligned-banner"
          className="rounded-[var(--radius-md)] border border-[var(--destructive)]/40 bg-[var(--destructive)]/10 px-4 py-3 text-sm text-[var(--destructive)]"
        >
          <div className="flex items-start gap-2">
            <AlertOctagon aria-hidden className="size-4 mt-0.5" />
            <p>
              <strong className="font-semibold">Policy regression detected.</strong>{" "}
              Replay output would have failed the current policy.
              Operator notified.
            </p>
          </div>
        </div>
      ) : null}

      <section
        data-testid="replay-header"
        data-action-id={actionId}
        className="rounded-[var(--radius-lg)] border border-[var(--info)]/30 bg-[var(--info)]/10 p-4"
      >
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-start gap-2">
            <ShieldCheck aria-hidden className="size-4 mt-0.5 text-[var(--info)]" />
            <div>
              <p
                data-testid="replay-sandbox-banner"
                className="text-sm font-medium text-[var(--foreground)]"
              >
                Sandbox — no side effects
              </p>
              <p className="text-xs text-[var(--muted-foreground)]">
                MCP calls hit mocks, computer-use targets a virtual display,
                outbound channels only produce would-have-sent traces.
              </p>
            </div>
          </div>
          {verdict ? <VerdictPill verdict={verdict} /> : null}
        </div>
      </section>
    </div>
  );
}
