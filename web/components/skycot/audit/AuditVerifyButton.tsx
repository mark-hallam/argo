"use client";

import { useState } from "react";
import { Loader2, ShieldAlert, ShieldCheck } from "lucide-react";
import { buttonVariants } from "@/components/ui/Button";
import { verifyAuditChain, type AuditVerifyResult } from "@/lib/api/audit-verify";
import { cn } from "@/lib/utils";

/**
 * `Verify integrity` — the in-browser chain-integrity action (#3014).
 *
 * Replaces the old static "CLI verify not run in browser" placeholder: a
 * customer can now self-serve PROVE their tamper-evident log is intact, then
 * hand the proof bundle to a regulator — without a terminal. Truth-bounded by
 * construction: it renders the REAL `intact` state the verifier returned (a
 * broken chain shows the first tampered row, never a green tick), and on any
 * error it says "couldn't verify" rather than implying success.
 */
function shortHash(h: string): string {
  if (!h || h.length <= 16) return h || "—";
  return `${h.slice(0, 10)}…${h.slice(-6)}`;
}

type VerifyState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "done"; result: AuditVerifyResult }
  | { kind: "error"; message: string };

export function AuditVerifyButton() {
  const [state, setState] = useState<VerifyState>({ kind: "idle" });

  async function run() {
    setState({ kind: "loading" });
    try {
      const result = await verifyAuditChain();
      setState({ kind: "done", result });
    } catch (err) {
      setState({
        kind: "error",
        message: err instanceof Error ? err.message : "Verification failed",
      });
    }
  }

  if (state.kind === "done") {
    const r = state.result;
    return (
      <div
        data-testid="audit-verify-result"
        data-intact={r.intact ? "true" : "false"}
        className={cn(
          "inline-flex flex-wrap items-center gap-x-2 gap-y-1 rounded-[var(--radius-md)] border px-3 py-2 text-xs font-medium",
          r.intact
            ? "border-[var(--success)]/40 bg-[var(--success)]/10 text-[var(--success)]"
            : "border-[var(--destructive)]/40 bg-[var(--destructive)]/10 text-[var(--destructive)]",
        )}
      >
        {r.intact ? (
          <ShieldCheck aria-hidden className="size-4" />
        ) : (
          <ShieldAlert aria-hidden className="size-4" />
        )}
        <span>
          {r.intact
            ? `Chain intact — ${r.rows_checked} ${r.rows_checked === 1 ? "row" : "rows"} verified`
            : `Chain BROKEN${
                r.first_broken_seq != null ? ` at row ${r.first_broken_seq}` : ""
              } — ${r.breaks} ${r.breaks === 1 ? "break" : "breaks"}`}
        </span>
        <span
          className="font-mono text-[10px] text-[var(--muted-foreground)]"
          title={r.head_hash}
        >
          head {shortHash(r.head_hash)}
        </span>
        <button
          type="button"
          onClick={run}
          className="underline decoration-dotted underline-offset-2 hover:opacity-80"
        >
          Re-verify
        </button>
      </div>
    );
  }

  if (state.kind === "error") {
    return (
      <div
        data-testid="audit-verify-error"
        className="inline-flex flex-wrap items-center gap-x-2 gap-y-1 rounded-[var(--radius-md)] border border-[var(--destructive)]/40 bg-[var(--destructive)]/10 px-3 py-2 text-xs font-medium text-[var(--destructive)]"
      >
        <ShieldAlert aria-hidden className="size-4" />
        <span>Couldn&apos;t verify — {state.message}</span>
        <button
          type="button"
          onClick={run}
          className="underline decoration-dotted underline-offset-2 hover:opacity-80"
        >
          Try again
        </button>
      </div>
    );
  }

  return (
    <button
      type="button"
      data-testid="audit-verify-button"
      onClick={run}
      disabled={state.kind === "loading"}
      className={cn(buttonVariants({ variant: "secondary", size: "sm" }))}
    >
      {state.kind === "loading" ? (
        <Loader2 aria-hidden className="size-4 animate-spin" />
      ) : (
        <ShieldCheck aria-hidden className="size-4" />
      )}
      {state.kind === "loading" ? "Verifying…" : "Verify integrity"}
    </button>
  );
}
