"use client";

import { RotateCcw } from "lucide-react";
import { cn } from "@/lib/utils";

export function RetryBadge({
  attempt,
  max,
  backoff,
}: {
  attempt: number;
  max: number;
  backoff: "none" | "exponential";
}) {
  const exhausted = attempt >= max;
  return (
    <span
      className={cn(
        "inline-flex h-7 items-center gap-1 rounded-[var(--radius-sm)] border px-2 text-xs",
        exhausted
          ? "border-red-300 bg-red-50 text-red-700"
          : "border-[var(--border)] bg-[var(--card)] text-[var(--muted-foreground)]",
      )}
      title={`${attempt} of ${max} attempts, ${backoff} backoff`}
    >
      <RotateCcw className="size-3.5" aria-hidden />
      <span>
        {attempt}/{max}
      </span>
    </span>
  );
}
