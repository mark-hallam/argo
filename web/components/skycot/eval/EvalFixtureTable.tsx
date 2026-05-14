import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { cn } from "@/lib/utils";
import type { EvalFixtureResult } from "@/lib/api/evals";

export function EvalFixtureTable({ fixtures }: { fixtures: EvalFixtureResult[] }) {
  return (
    <Card data-testid="eval-fixture-table">
      <CardHeader>
        <CardTitle>Fixture detail</CardTitle>
      </CardHeader>
      <CardBody className="pt-1">
        <div className="overflow-x-auto">
          <table className="min-w-full text-sm">
            <thead className="text-left text-xs text-[var(--muted-foreground)]">
              <tr className="border-b border-[var(--border)]">
                <th className="py-2 pr-4 font-medium">Fixture</th>
                <th className="py-2 pr-4 font-medium">Result</th>
                <th className="py-2 pr-4 font-medium">Score</th>
                <th className="py-2 pr-4 font-medium">Diff</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[var(--border)]/60">
              {fixtures.map((fixture) => (
                <tr key={fixture.fixture} data-testid={`eval-fixture-${fixture.fixture}`}>
                  <td className="max-w-[18rem] py-3 pr-4 align-top">
                    <p className="font-medium text-[var(--foreground)]">{fixture.fixture}</p>
                    <p className="mt-1 text-xs text-[var(--muted-foreground)]">{fixture.metric}</p>
                  </td>
                  <td className="py-3 pr-4 align-top">
                    <span
                      data-testid={`eval-fixture-status-${fixture.fixture}`}
                      className={cn(
                        "inline-flex rounded-full border px-2 py-0.5 text-xs font-medium",
                        fixture.passed
                          ? "border-[var(--success)]/30 bg-[var(--success)]/10 text-[var(--success-foreground)]"
                          : "border-[var(--destructive)]/30 bg-[var(--destructive)]/10 text-[var(--destructive)]",
                      )}
                    >
                      {fixture.passed ? "passed" : "failed"}
                    </span>
                    <p className="mt-2 text-xs text-[var(--muted-foreground)]">{fixture.actual}</p>
                  </td>
                  <td className="py-3 pr-4 align-top tabular-nums">
                    {Math.round(fixture.score * 100)}%
                  </td>
                  <td
                    className={cn(
                      "py-3 pr-4 align-top tabular-nums",
                      fixture.scoreDelta < 0
                        ? "text-[var(--destructive)]"
                        : "text-[var(--success-foreground)]",
                    )}
                  >
                    {fixture.scoreDelta >= 0 ? "+" : ""}
                    {Math.round(fixture.scoreDelta * 100)} pts
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </CardBody>
    </Card>
  );
}
