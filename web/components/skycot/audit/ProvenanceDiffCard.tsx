import { ArrowRight, Minus } from "lucide-react";
import type { ReplayResult } from "@/lib/api/replay";

/**
 * 5-axis provenance diff card. Each row reports `same` (muted) or
 * shows the version delta in warning tone with a `from → to` arrow.
 * The axes are locked: model · prompt · skill · memory · policy.
 *
 * The memory axis only carries `changed: boolean` — no `from` / `to`
 * strings, since memory snapshots are content-addressed identifiers
 * rather than versions; we display `same` or `updated`.
 */

type Axis = "model" | "promptTemplate" | "skill" | "memorySnapshot" | "policy";

const AXIS_ORDER: Axis[] = [
  "model",
  "promptTemplate",
  "skill",
  "memorySnapshot",
  "policy",
];

const AXIS_LABEL: Record<Axis, string> = {
  model: "Model",
  promptTemplate: "Prompt",
  skill: "Skill",
  memorySnapshot: "Memory",
  policy: "Policy",
};

export interface ProvenanceDiffCardProps {
  diff: ReplayResult["provenanceDiff"];
}

export function ProvenanceDiffCard({ diff }: ProvenanceDiffCardProps) {
  return (
    <section
      data-testid="replay-provenance-diff"
      className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 space-y-3"
    >
      <header>
        <h2 className="text-sm font-semibold text-[var(--foreground)]">
          Provenance diff
        </h2>
        <p className="text-xs text-[var(--muted-foreground)]">
          Each axis compares the original action's identity to the
          sandbox replay's identity.
        </p>
      </header>

      <dl className="space-y-1.5">
        {AXIS_ORDER.map((axis) => (
          <DiffRow key={axis} axis={axis} entry={diff[axis]} />
        ))}
      </dl>
    </section>
  );
}

interface AxisEntry {
  changed: boolean;
  from?: string;
  to?: string;
}

function DiffRow({ axis, entry }: { axis: Axis; entry: AxisEntry }) {
  const changed = entry.changed;
  return (
    <div
      data-testid={`replay-provenance-row-${axis}`}
      data-changed={changed ? "true" : "false"}
      className={`grid grid-cols-[120px_1fr] items-center gap-3 rounded-[var(--radius-sm)] px-2 py-1.5 ${
        changed ? "bg-[var(--warning)]/10" : "hover:bg-[var(--muted)]/30"
      }`}
    >
      <dt
        className={`text-xs uppercase tracking-wider ${
          changed ? "text-[var(--warning)]" : "text-[var(--muted-foreground)]"
        }`}
      >
        {AXIS_LABEL[axis]}
      </dt>
      <dd
        className={`text-xs font-mono ${
          changed ? "text-[var(--foreground)]" : "text-[var(--muted-foreground)]"
        }`}
      >
        {changed ? <ChangedValue entry={entry} /> : <SameValue />}
      </dd>
    </div>
  );
}

function SameValue() {
  return (
    <span className="inline-flex items-center gap-1.5">
      <Minus aria-hidden className="size-3" />
      <span>same</span>
    </span>
  );
}

function ChangedValue({ entry }: { entry: AxisEntry }) {
  if (entry.from && entry.to) {
    return (
      <span className="inline-flex items-center gap-1.5 flex-wrap">
        <span data-testid="replay-provenance-from">{entry.from}</span>
        <ArrowRight aria-hidden className="size-3 text-[var(--warning)]" />
        <span data-testid="replay-provenance-to">{entry.to}</span>
      </span>
    );
  }
  return <span>updated</span>;
}
