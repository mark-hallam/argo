"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { Loader2 } from "lucide-react";
import { HireStarterPrompts } from "@/components/empty-state/HireStarterPrompts";
import { useTeam } from "@/lib/hooks/use-team";

/**
 * `/workers` — nav-bar entrypoint for the worker roster. When the
 * tenant has zero workers, render the shared `<HireStarterPrompts>`
 * empty state (the same 5 starter-prompt grid the `/team` page uses)
 * so the buyer sees the same first-touch CTA pattern from either
 * surface. When workers exist, redirect to `/team`, the canonical
 * roster surface — `useTeam()` is the shared data source. A dedicated
 * workers list with column-level filters is a separate Track-E brief;
 * this page only owns the empty-state experience until then.
 */
export default function WorkersPage() {
  const router = useRouter();
  const teamQuery = useTeam();
  const workers = teamQuery.data?.workers ?? [];
  const hasWorkers = !teamQuery.isPending && workers.length > 0;

  useEffect(() => {
    if (hasWorkers) router.replace("/team");
  }, [hasWorkers, router]);

  if (teamQuery.isPending || hasWorkers) {
    return (
      <div className="grid min-h-64 place-items-center" data-testid="workers-loading">
        <Loader2 className="size-5 animate-spin text-[var(--muted-foreground)]" aria-hidden />
      </div>
    );
  }

  return (
    <div
      className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8 space-y-6"
      data-testid="workers-empty"
    >
      <h1 className="sr-only">Workers</h1>
      <HireStarterPrompts />
    </div>
  );
}
