"use client";

import { CheckCircle2, Plus } from "lucide-react";
import { type FormEvent, type ReactNode, useMemo, useState } from "react";
import { Button } from "@/components/ui/Button";

const DEPARTMENTS = [
  "GTM",
  "Marketing",
  "Sales",
  "Support",
  "Operations",
  "Project",
] as const;

interface DraftCohort {
  name: string;
  department: string;
  workers: string[];
}

export function CohortCreateForm() {
  const [name, setName] = useState("");
  const [department, setDepartment] = useState<(typeof DEPARTMENTS)[number]>("GTM");
  const [workerList, setWorkerList] = useState("Mariah\nPeta");
  const [draft, setDraft] = useState<DraftCohort | null>(null);

  const workers = useMemo(
    () =>
      workerList
        .split(/\n|,/)
        .map((worker) => worker.trim())
        .filter(Boolean),
    [workerList],
  );
  const canSubmit = name.trim().length > 1 && workers.length > 0;

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit) return;
    setDraft({
      name: name.trim(),
      department,
      workers,
    });
  }

  return (
    <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_320px]">
      <form
        className="space-y-5 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 text-[var(--card-foreground)] shadow-sm"
        onSubmit={onSubmit}
        data-testid="cohort-create-form"
      >
        <Field label="Cohort name" htmlFor="cohort-name">
          <input
            id="cohort-name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Launch support squad"
            className="h-10 w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-3 text-sm text-[var(--foreground)] outline-none transition focus:border-[var(--ring)] focus:ring-2 focus:ring-[var(--ring)]/20"
          />
        </Field>

        <Field label="Department" htmlFor="cohort-department">
          <select
            id="cohort-department"
            value={department}
            onChange={(event) =>
              setDepartment(event.target.value as (typeof DEPARTMENTS)[number])
            }
            className="h-10 w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-3 text-sm text-[var(--foreground)] outline-none transition focus:border-[var(--ring)] focus:ring-2 focus:ring-[var(--ring)]/20"
          >
            {DEPARTMENTS.map((dept) => (
              <option key={dept} value={dept}>
                {dept}
              </option>
            ))}
          </select>
        </Field>

        <Field label="Initial workers" htmlFor="cohort-workers">
          <textarea
            id="cohort-workers"
            value={workerList}
            onChange={(event) => setWorkerList(event.target.value)}
            rows={5}
            className="w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm text-[var(--foreground)] outline-none transition focus:border-[var(--ring)] focus:ring-2 focus:ring-[var(--ring)]/20"
          />
        </Field>

        <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--border)] pt-4">
          <p className="text-xs text-[var(--muted-foreground)]">
            {workers.length} {workers.length === 1 ? "worker" : "workers"} selected
          </p>
          <Button type="submit" disabled={!canSubmit} size="sm">
            <Plus aria-hidden />
            Create cohort plan
          </Button>
        </div>
      </form>

      <aside className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 text-[var(--card-foreground)] shadow-sm">
        <div className="space-y-2">
          <h2 className="text-sm font-semibold tracking-tight">Draft summary</h2>
          <p className="text-sm leading-6 text-[var(--muted-foreground)]">
            Cohort tiles appear after real worker rows share a department and trust
            band. This plan gives the roster a concrete assignment target.
          </p>
        </div>
        {draft ? (
          <div
            className="mt-5 space-y-3 rounded-[var(--radius-md)] border border-emerald-500/25 bg-emerald-500/10 p-4"
            data-testid="cohort-draft-ready"
          >
            <div className="flex items-center gap-2 text-sm font-semibold text-emerald-700 dark:text-emerald-300">
              <CheckCircle2 aria-hidden className="size-4" />
              Cohort draft ready
            </div>
            <dl className="space-y-2 text-sm">
              <div>
                <dt className="text-[var(--muted-foreground)]">Name</dt>
                <dd>{draft.name}</dd>
              </div>
              <div>
                <dt className="text-[var(--muted-foreground)]">Department</dt>
                <dd>{draft.department}</dd>
              </div>
              <div>
                <dt className="text-[var(--muted-foreground)]">Workers</dt>
                <dd>{draft.workers.join(", ")}</dd>
              </div>
            </dl>
          </div>
        ) : null}
      </aside>
    </div>
  );
}

function Field({
  label,
  htmlFor,
  children,
}: {
  label: string;
  htmlFor: string;
  children: ReactNode;
}) {
  return (
    <label className="block space-y-2" htmlFor={htmlFor}>
      <span className="text-sm font-medium text-[var(--foreground)]">{label}</span>
      {children}
    </label>
  );
}
