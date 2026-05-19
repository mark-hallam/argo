"use client";

import { Sparkles, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { apiGet, apiPatch } from "@/lib/api/client";
import { cn } from "@/lib/utils";

/**
 * Trust-ratchet promotion card (Stream `mission-pm-trust-ratchet-ui`).
 * After N consecutive approvals on the same (worker, tool, target_glob),
 * the inbox offers a one-click promotion to auto-approve. The decision
 * stays with the operator — nothing is promoted without a click.
 * Dismissal is local: a 7-day expiry written to localStorage.
 */

export interface TrustStreak {
  worker_id: string;
  tool: string;
  target_glob?: string | null;
  streak_count: number;
  last_decided_at_micros: number;
}

export interface TrustStreakBundle {
  worker_id: string;
  min_threshold: number;
  streaks: TrustStreak[];
}

export interface TrustRatchetCardProps {
  worker: { id: string; name: string; avatarUrl?: string };
  tool: string;
  targetGlob?: string;
  streakCount: number;
  recentApprovalsIso?: string[];
  onPromote: () => void | Promise<void>;
  dismissKeyPrefix?: string;
  className?: string;
}

const DEFAULT_DISMISS_PREFIX = "skycot.trust-ratchet.dismissed";
const DISMISS_TTL_MS = 7 * 24 * 60 * 60 * 1000;

export function dismissKeyFor(
  prefix: string, worker: { id: string }, tool: string, targetGlob: string | undefined,
): string {
  return `${prefix}::${worker.id}::${tool}::${targetGlob ?? "*"}`;
}

export function isDismissed(
  prefix: string, worker: { id: string }, tool: string,
  targetGlob: string | undefined, now: number = Date.now(),
): boolean {
  if (typeof window === "undefined") return false;
  try {
    const raw = window.localStorage.getItem(dismissKeyFor(prefix, worker, tool, targetGlob));
    if (raw == null) return false;
    const expiresAt = Number.parseInt(raw, 10);
    return Number.isFinite(expiresAt) && now < expiresAt;
  } catch { return false; }
}

export function TrustRatchetCard({
  worker, tool, targetGlob, streakCount, recentApprovalsIso, onPromote,
  dismissKeyPrefix = DEFAULT_DISMISS_PREFIX, className,
}: TrustRatchetCardProps) {
  const [busy, setBusy] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  useEffect(() => {
    setDismissed(isDismissed(dismissKeyPrefix, worker, tool, targetGlob));
  }, [dismissKeyPrefix, worker, tool, targetGlob]);
  if (dismissed) return null;

  async function handlePromote() {
    if (busy) return;
    setBusy(true);
    try { await onPromote(); } finally { setBusy(false); }
  }
  function handleDismiss() {
    try {
      window.localStorage.setItem(
        dismissKeyFor(dismissKeyPrefix, worker, tool, targetGlob),
        String(Date.now() + DISMISS_TTL_MS),
      );
    } catch { /* localStorage may be disabled */ }
    setDismissed(true);
  }

  const subject = `${worker.name}'s ${tool}`;
  const scope = targetGlob ? ` for ${targetGlob}` : "";
  return (
    <Card
      className={cn("w-full overflow-hidden border-[var(--primary)]/40 bg-[var(--primary)]/5", className)}
      role="region"
      aria-label={`Auto-approve offer for ${subject}`}
      data-testid="trust-ratchet-card"
    >
      <div className="flex items-start gap-3 px-5 pt-5">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-[var(--primary)]/15 text-[var(--primary)]" aria-hidden>
          <Sparkles className="size-4" />
        </div>
        <div className="flex-1 min-w-0 pt-0.5">
          <div className="text-[15px] leading-snug">
            <span className="font-semibold text-[var(--foreground)]">You&apos;ve approved {subject}</span>{" "}
            <span className="text-[var(--muted-foreground)]">{streakCount} times in a row{scope}.</span>
          </div>
          <div className="mt-1 text-[12px] text-[var(--muted-foreground)]">
            Auto-approve this pattern going forward?
          </div>
        </div>
        <button type="button" onClick={handleDismiss}
          className="rounded-[var(--radius-sm)] p-1 text-[var(--muted-foreground)] hover:bg-[var(--muted)]/40"
          aria-label="Dismiss for 7 days" data-testid="trust-ratchet-dismiss">
          <X className="size-4" />
        </button>
      </div>
      {recentApprovalsIso && recentApprovalsIso.length > 0 ? (
        <ul className="mx-5 mt-3 list-disc space-y-0.5 pl-5 text-[12px] text-[var(--muted-foreground)]" data-testid="trust-ratchet-recent">
          {recentApprovalsIso.slice(0, 3).map((iso) => (
            <li key={iso}><time dateTime={iso} title={iso}>{iso}</time></li>
          ))}
        </ul>
      ) : null}
      <div className="flex items-center gap-2 px-5 py-4">
        <Button variant="primary" size="sm" onClick={handlePromote} disabled={busy} data-testid="trust-ratchet-promote">
          Auto-approve going forward
        </Button>
        <Button variant="ghost" size="sm" onClick={handleDismiss} data-testid="trust-ratchet-dismiss-text">
          Not now
        </Button>
      </div>
    </Card>
  );
}

/* --- inbox-section wrapper. Fan-out one fetch per worker_id;
 * promotion writes through the existing per-worker policy PATCH. */

export interface TrustRatchetSectionProps {
  workerIds: string[];
  workerLookup?: Record<string, { name: string; avatarUrl?: string }>;
}

async function fetchStreaks(workerId: string): Promise<TrustStreakBundle> {
  return apiGet<TrustStreakBundle>(`/api/v1/workers/${workerId}/trust-streaks`);
}

async function promotePolicy(workerId: string, tool: string, streakCount: number): Promise<void> {
  await apiPatch<unknown, { policy: string; operator_note: string }>(
    `/api/v1/workers/${workerId}/policies/${encodeURIComponent(tool)}`,
    { policy: "auto-approve", operator_note: `Promoted from trust ratchet (${streakCount} consecutive approvals)` },
  );
}

export function TrustRatchetSection({ workerIds, workerLookup }: TrustRatchetSectionProps) {
  const ids = Array.from(new Set(workerIds));
  if (ids.length === 0) return null;
  return (
    <section aria-label="Trust-ratchet offers" className="space-y-3" data-testid="trust-ratchet-section">
      {ids.map((id) => (
        <TrustRatchetForWorker key={id} workerId={id} display={workerLookup?.[id]} />
      ))}
    </section>
  );
}

function TrustRatchetForWorker({ workerId, display }: { workerId: string; display?: { name: string; avatarUrl?: string } }) {
  const query = useQuery({ queryKey: ["trust-streaks", workerId], queryFn: () => fetchStreaks(workerId) });
  const streaks = query.data?.streaks ?? [];
  if (streaks.length === 0) return null;
  const name = display?.name ?? workerId;
  return (
    <>
      {streaks.map((s) => (
        <TrustRatchetCard
          key={`${s.tool}::${s.target_glob ?? "*"}`}
          worker={{ id: workerId, name, avatarUrl: display?.avatarUrl }}
          tool={s.tool}
          targetGlob={s.target_glob ?? undefined}
          streakCount={s.streak_count}
          onPromote={() => promotePolicy(workerId, s.tool, s.streak_count)}
        />
      ))}
    </>
  );
}
