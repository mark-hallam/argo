"use client";

import { Clock3 } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { cn } from "@/lib/utils";
import type { EvalRun } from "@/lib/api/evals";

export function EvalRunHistory({
  runs,
  selectedRunId,
  onSelect,
}: {
  runs: EvalRun[];
  selectedRunId: string;
  onSelect: (runId: string) => void;
}) {
  return (
    <Card data-testid="eval-run-history">
      <CardHeader>
        <CardTitle>Run history</CardTitle>
      </CardHeader>
      <CardBody className="pt-1">
        <ul className="space-y-2">
          {runs.map((run) => {
            const selected = run.id === selectedRunId;
            return (
              <li key={run.id}>
                <Button
                  type="button"
                  variant="ghost"
                  data-testid={`eval-run-${run.id}`}
                  aria-pressed={selected}
                  onClick={() => onSelect(run.id)}
                  className={cn(
                    "h-auto w-full justify-start rounded-[var(--radius-md)] border px-3 py-3 text-left",
                    selected ? "border-[var(--ring)] bg-[var(--accent)]" : "border-transparent",
                  )}
                >
                  <span className="grid w-full gap-1">
                    <span className="flex items-center justify-between gap-3">
                      <span className="truncate text-sm font-medium">{run.version}</span>
                      <span className="text-xs text-[var(--muted-foreground)]">
                        {Math.round(run.passRate * 100)}%
                      </span>
                    </span>
                    <span className="flex items-center gap-1 text-xs text-[var(--muted-foreground)]">
                      <Clock3 className="size-3" aria-hidden />
                      {new Date(run.createdAt).toLocaleString()}
                    </span>
                  </span>
                </Button>
              </li>
            );
          })}
        </ul>
      </CardBody>
    </Card>
  );
}
