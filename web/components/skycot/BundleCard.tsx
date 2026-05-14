"use client";

import { Check, Layers, X } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { StatusPill } from "@/components/ui/StatusPill";
import { cn } from "@/lib/utils";

/**
 * `BundleCard` — the load-bearing visual for the server-defined
 * approval-bundle UX (mission-approval-bundle-ui).
 *
 * Operators MUST see at a glance that a bundle is N actions, not 1.
 * The card uses two trailing offset/shadow layers behind the
 * foreground card so the metaphor reads as a stack of cards.
 * Single-decision UI: Approve / Refuse acts on every still-pending
 * member atomically server-side.
 *
 * Structural contract (DO NOT REORDER):
 *   1. Layered shadows that say "this is a stack of N"
 *   2. Stack icon + bold count badge
 *   3. The worker's label (what the batch is about)
 *   4. Two actions: Approve · Refuse
 */

export type BundleDecision = "approve" | "refuse";

export interface BundleCardProps {
  /** Server-assigned bundle id, opaque to the UI. */
  bundleId: string;
  /** Human label the worker supplied — "8 social-media replies", etc. */
  label: string;
  /** Number of member approvals; rendered as the badge count. */
  count: number;
  /** Per-member approval-ids in worker order — surfaced in the affordance "what's inside". */
  members?: string[];
  /** Called with the operator's collective decision. */
  onDecide: (decision: BundleDecision) => void | Promise<void>;
  /** Host-controlled decision state. Once non-null the card is in post-decision mode. */
  decision?: BundleDecision | null;
  className?: string;
  /** Test-only override for the "..." inspector affordance — defaults to off. */
  showMembersOnMount?: boolean;
}

const STACK_DEPTH_PX = [-6, -3] as const;

export function BundleCard({
  bundleId,
  label,
  count,
  members,
  onDecide,
  decision: controlledDecision,
  className,
  showMembersOnMount = false,
}: BundleCardProps) {
  const [busy, setBusy] = useState(false);
  const [pending, setPending] = useState<BundleDecision | null>(null);
  const [showMembers, setShowMembers] = useState(showMembersOnMount);
  const decision = controlledDecision ?? pending;

  async function handle(d: BundleDecision) {
    if (busy || decision != null) return;
    setBusy(true);
    try {
      setPending(d);
      await onDecide(d);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      className={cn("relative w-full", className)}
      role="region"
      aria-label={`Approval bundle: ${label}`}
      data-testid={`bundle-card-${bundleId}`}
    >
      {/* Stack-illusion shadow layers. Pure decoration; pointer-events
          off so they never intercept clicks on the foreground card. */}
      {STACK_DEPTH_PX.map((offset, i) => (
        <div
          key={offset}
          aria-hidden
          className="absolute inset-x-0 top-0 h-full rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] shadow-sm pointer-events-none"
          style={{
            transform: `translate(${offset}px, ${-offset}px)`,
            opacity: 0.6 - i * 0.15,
            zIndex: -1 - i,
          }}
          data-testid={`bundle-stack-layer-${i}`}
        />
      ))}

      <Card className="relative w-full overflow-hidden shadow-md ring-1 ring-[var(--primary)]/15">
        <div className="flex items-start gap-3 px-5 pt-5">
          <div className="grid size-10 place-items-center rounded-[var(--radius-md)] bg-[var(--primary)]/10 text-[var(--primary)]">
            <Layers className="size-5" />
          </div>
          <div className="flex-1 min-w-0 pt-0.5">
            <div className="flex items-center gap-2 flex-wrap">
              <h3 className="text-[15px] font-semibold leading-snug text-[var(--foreground)]">
                {label}
              </h3>
              <StatusPill tone="info" size="sm" data-testid="bundle-count-pill">
                {count} actions
              </StatusPill>
            </div>
            <p className="mt-1 text-[12px] text-[var(--muted-foreground)]">
              One decision applies to every action in this bundle.
            </p>
          </div>
        </div>

        <div className="px-5 py-4 text-[13px] leading-relaxed text-[var(--muted-foreground)]">
          {decision == null ? (
            <span>
              Approving accepts all <b>{count}</b> at once. Refusing rejects the
              whole group.
            </span>
          ) : decision === "approve" ? (
            <span>Approved · {count} actions sent to your teammates.</span>
          ) : (
            <span>Refused · {count} actions cancelled.</span>
          )}
        </div>

        {decision == null ? (
          <div className="flex items-center gap-2 px-5 pb-5">
            <Button
              variant="primary"
              size="md"
              className="flex-1 shadow-sm"
              onClick={() => handle("approve")}
              loading={busy && pending === "approve"}
              disabled={busy}
              data-testid="bundle-approve"
            >
              <Check />
              Approve bundle
            </Button>
            <Button
              variant="secondary"
              size="md"
              className="flex-1"
              onClick={() => handle("refuse")}
              loading={busy && pending === "refuse"}
              disabled={busy}
              data-testid="bundle-refuse"
            >
              <X />
              Refuse bundle
            </Button>
            {members && members.length > 0 ? (
              <Button
                variant="ghost"
                size="md"
                onClick={() => setShowMembers((v) => !v)}
                aria-expanded={showMembers}
                aria-controls={`bundle-members-${bundleId}`}
                data-testid="bundle-toggle-members"
              >
                {showMembers ? "Hide" : "What's inside?"}
              </Button>
            ) : null}
          </div>
        ) : (
          <div className="px-5 pb-5 text-xs text-[var(--muted-foreground)]">
            Bundle decision recorded. Individual members will follow.
          </div>
        )}

        {showMembers && members && members.length > 0 ? (
          <div
            id={`bundle-members-${bundleId}`}
            className="border-t border-[var(--border)] px-5 py-3 text-xs leading-relaxed bg-[var(--muted)]/30"
          >
            <p className="font-semibold mb-1 text-[var(--foreground)]">
              Members ({members.length})
            </p>
            <ul className="space-y-0.5 text-[var(--muted-foreground)] font-mono">
              {members.map((id) => (
                <li key={id} data-testid="bundle-member-id">
                  {id}
                </li>
              ))}
            </ul>
          </div>
        ) : null}
      </Card>
    </div>
  );
}
