import { cn } from "@/lib/utils";

/**
 * Three-tier severity pill rendered on each approval card.
 * P0 → over-SLA / requires-immediate, P1 → elevated, P2 → normal.
 *
 * Tone tokens come from the global palette so dark/light themes and
 * white-label tenants pick up the swap automatically. We render the
 * label in JetBrains Mono via the .skycot-mono utility (defined in
 * globals.css) for the ops-dashboard signal.
 */

export type Severity = "p0" | "p1" | "p2";

const LABEL: Record<Severity, string> = {
  p0: "P0",
  p1: "P1",
  p2: "P2",
};

const TONE_CLS: Record<Severity, string> = {
  p0: "border-[var(--destructive)]/40 bg-[var(--destructive)]/12 text-[var(--destructive)]",
  p1: "border-[var(--warning)]/40 bg-[var(--warning)]/12 text-[var(--warning)]",
  p2: "border-[var(--border)] bg-[var(--muted)] text-[var(--muted-foreground)]",
};

export interface SeverityPillProps {
  severity: Severity;
  className?: string;
}

export function SeverityPill({ severity, className }: SeverityPillProps) {
  return (
    <span
      data-testid={`severity-pill-${severity}`}
      data-severity={severity}
      className={cn(
        "inline-flex items-center gap-1 rounded-[var(--radius-sm)] border px-2 py-0.5",
        "text-[10px] font-semibold leading-none tracking-[0.08em]",
        "font-mono",
        TONE_CLS[severity],
        className,
      )}
    >
      <span aria-hidden className="size-1.5 rounded-full bg-current" />
      {LABEL[severity]}
    </span>
  );
}
