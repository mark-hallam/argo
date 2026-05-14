import { AlertTriangle, CheckCircle2, Info } from "lucide-react";
import { cn } from "@/lib/utils";
import type { EvalRun } from "@/lib/api/evals";

const TONE = {
  regressed: {
    icon: AlertTriangle,
    label: "Regression detected",
    className: "border-[var(--destructive)]/35 bg-[var(--destructive)]/8 text-[var(--foreground)]",
  },
  stable: {
    icon: CheckCircle2,
    label: "Stable",
    className: "border-[var(--success)]/35 bg-[var(--success)]/10 text-[var(--foreground)]",
  },
  no_baseline: {
    icon: Info,
    label: "No baseline",
    className: "border-[var(--warning)]/35 bg-[var(--warning)]/10 text-[var(--foreground)]",
  },
} as const;

export function RegressionBanner({ run }: { run: EvalRun }) {
  const tone = TONE[run.decision];
  const Icon = tone.icon;
  const regressions = run.fixtures.filter((f) => !f.passed || f.scoreDelta < 0).length;
  return (
    <section
      data-testid="regression-banner"
      data-decision={run.decision}
      className={cn("rounded-[var(--radius-md)] border px-4 py-3", tone.className)}
    >
      <div className="flex items-start gap-3">
        <Icon className="mt-0.5 size-5 shrink-0" aria-hidden />
        <div className="min-w-0">
          <h2 className="text-sm font-semibold">{tone.label}</h2>
          <p className="mt-1 text-sm text-[var(--muted-foreground)]">
            {run.version} passed {Math.round(run.passRate * 100)}% of {run.fixtureCount} fixtures.
            {run.decision === "regressed"
              ? ` ${regressions} fixture${regressions === 1 ? "" : "s"} need review before promotion.`
              : " No blocking eval drift is currently visible."}
          </p>
        </div>
      </div>
    </section>
  );
}
