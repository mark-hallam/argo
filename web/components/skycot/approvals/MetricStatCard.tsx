import { cn } from "@/lib/utils";

export function MetricStatCard({
  title,
  value,
  subtitle,
  testId,
  tone,
}: {
  title: string;
  value: string;
  subtitle: string;
  testId: string;
  tone?: "risk" | "neutral";
}) {
  return (
    <article
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-5"
      data-testid={testId}
      data-percent="none"
      data-visual="stat"
    >
      <div
        className={cn(
          "mb-4 h-1.5 w-14 rounded-[999px]",
          tone === "risk" ? "bg-[var(--warning)]" : "bg-[var(--primary)]",
        )}
      />
      <h2 className="text-sm font-semibold text-[var(--foreground)]">{title}</h2>
      <div
        className="mt-2 font-mono text-3xl font-semibold tabular-nums text-[var(--foreground)]"
        data-testid={`${testId}-value`}
      >
        {value}
      </div>
      <p className="mt-2 text-xs leading-5 text-[var(--muted-foreground)]">
        {subtitle}
      </p>
    </article>
  );
}
