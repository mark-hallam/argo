"use client";

interface ChainEmptyStateProps {
  body: string;
  testId: string;
  title: string;
}

export function ChainEmptyState({ body, testId, title }: ChainEmptyStateProps) {
  return (
    <div
      data-testid={testId}
      className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] bg-[var(--muted)]/20 p-4"
    >
      <p className="text-sm font-medium text-[var(--foreground)]">{title}</p>
      <p className="mt-1 text-sm text-[var(--muted-foreground)]">{body}</p>
    </div>
  );
}
