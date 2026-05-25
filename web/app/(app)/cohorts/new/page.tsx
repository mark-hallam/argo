import Link from "next/link";
import { CohortCreateForm } from "@/components/skycot/cohorts/CohortCreateForm";

export default function NewCohortPage() {
  return (
    <div
      className="mx-auto max-w-5xl px-4 py-8 sm:px-6 lg:px-8"
      data-testid="cohort-create-page"
    >
      <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
        <div className="space-y-2">
          <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.16em] text-[var(--muted-foreground)]">
            Cohorts
          </p>
          <h1 className="text-3xl font-semibold tracking-tight text-[var(--foreground)]">
            Create cohort
          </h1>
          <p className="max-w-2xl text-sm leading-6 text-[var(--muted-foreground)]">
            Cohorts are generated from real workers, their department, and trust
            band. Draft the cohort here, then assign workers from the roster.
          </p>
        </div>
        <Link
          href="/cohorts"
          className="inline-flex h-8 items-center justify-center rounded-[var(--radius-sm)] px-3 text-sm font-medium text-[var(--foreground)] transition-colors hover:bg-[var(--accent)] hover:text-[var(--accent-foreground)]"
        >
          Back to cohorts
        </Link>
      </div>

      <CohortCreateForm />
    </div>
  );
}
