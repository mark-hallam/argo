"use client";

import { useMemo, useState } from "react";
import { useParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import { AlertCircle, Loader2 } from "lucide-react";
import { Card, CardBody } from "@/components/ui/Card";
import { EvalFixtureTable } from "@/components/skycot/eval/EvalFixtureTable";
import { EvalRunHistory } from "@/components/skycot/eval/EvalRunHistory";
import { RegressionBanner } from "@/components/skycot/eval/RegressionBanner";
import { fetchWorkerEvals } from "@/lib/api/evals";
import { useWorker } from "@/lib/hooks/use-worker";

export default function WorkerEvalsPage() {
  const routeParams = useParams<{ workerId: string }>();
  const workerId = routeParams?.workerId ?? "";
  const workerQuery = useWorker(workerId);
  const evalsQuery = useQuery({
    queryKey: ["worker", workerId, "evals"],
    queryFn: () => fetchWorkerEvals(workerId),
    retry: false,
  });
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);

  const selectedRun = useMemo(() => {
    const payload = evalsQuery.data;
    if (!payload) return null;
    return payload.history.find((run) => run.id === selectedRunId) ?? payload.latest;
  }, [evalsQuery.data, selectedRunId]);

  if (evalsQuery.isPending) {
    return (
      <div data-testid="worker-evals-loading" className="grid min-h-64 place-items-center">
        <Loader2 className="size-6 animate-spin text-[var(--muted-foreground)]" aria-hidden />
      </div>
    );
  }

  if (evalsQuery.isError || !evalsQuery.data || !selectedRun) {
    return (
      <div
        data-testid="worker-evals-error"
        className="rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 py-10 px-6 text-center"
      >
        <AlertCircle className="mx-auto size-8 text-[var(--destructive)]" aria-hidden />
        <p className="mt-3 text-sm text-[var(--foreground)]">
          {(evalsQuery.error as Error | null)?.message ?? "Couldn't load eval results"}
        </p>
      </div>
    );
  }

  const workerName = workerQuery.data?.name ?? "This worker";

  return (
    <div data-testid="worker-evals-page" className="space-y-4">
      <header className="space-y-1">
        <p className="text-sm text-[var(--muted-foreground)]">{workerName}</p>
        <h1 className="text-2xl font-semibold tracking-normal text-[var(--foreground)]">
          Eval results
        </h1>
      </header>

      <RegressionBanner run={selectedRun} />

      <section className="grid gap-3 sm:grid-cols-3" aria-label="Eval summary">
        <SummaryCard label="Pass rate" value={`${Math.round(selectedRun.passRate * 100)}%`} />
        <SummaryCard label="Mean score" value={`${Math.round(selectedRun.meanScore * 100)}%`} />
        <SummaryCard label="Fixtures" value={String(selectedRun.fixtureCount)} />
      </section>

      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_18rem]">
        <EvalFixtureTable fixtures={selectedRun.fixtures} />
        <EvalRunHistory
          runs={evalsQuery.data.history}
          selectedRunId={selectedRun.id}
          onSelect={setSelectedRunId}
        />
      </div>
    </div>
  );
}

function SummaryCard({ label, value }: { label: string; value: string }) {
  return (
    <Card>
      <CardBody className="py-3">
        <p className="text-xs text-[var(--muted-foreground)]">{label}</p>
        <p className="mt-1 text-xl font-semibold tabular-nums text-[var(--foreground)]">{value}</p>
      </CardBody>
    </Card>
  );
}
