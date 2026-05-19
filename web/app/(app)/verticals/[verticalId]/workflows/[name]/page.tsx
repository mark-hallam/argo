/**
 * Minimal workflow viewer at `/verticals/<v>/workflows/<n>`. Lists the
 * workflow's versions (newest first) and exposes a "Compare versions"
 * affordance that picks the two most recent versions as defaults and
 * deep-links to the side-by-side diff route.
 *
 * Track L Phase 2 will replace this with the full editor surface; for
 * now it is the lightest page that satisfies the diff-route entry
 * point and proves the gateway wiring.
 */

import { notFound } from "next/navigation";
import Link from "next/link";
import { apiGet, ApiError } from "@/lib/api/client";
import { Breadcrumb, BreadcrumbItem, BreadcrumbSeparator } from "@/components/ui/Breadcrumb";

interface VersionListItem {
  id: string;
  parent_id?: string | null;
  created_at?: string;
  created_by?: string;
  is_head?: boolean;
  label?: string;
}

interface WorkflowPageProps {
  params: Promise<{ verticalId: string; name: string }>;
}

export default async function WorkflowViewerPage({ params }: WorkflowPageProps) {
  const { verticalId, name } = await params;
  const versions = await fetchVersions(verticalId, name);
  const workflowLabel = decodeURIComponent(name);
  const verticalLabel = decodeURIComponent(verticalId);

  const defaultTo = versions[0]?.id;
  const defaultFrom = versions[1]?.id ?? versions[0]?.id;
  const compareHref =
    defaultFrom && defaultTo
      ? `/verticals/${verticalId}/workflows/${name}/diff?from=${encodeURIComponent(defaultFrom)}&to=${encodeURIComponent(defaultTo)}`
      : null;

  return (
    <div data-testid="workflow-viewer-page" className="space-y-4">
      <Breadcrumb>
        <BreadcrumbItem href="/orchestration">Orchestration</BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem current>{workflowLabel}</BreadcrumbItem>
      </Breadcrumb>

      <header className="flex flex-col gap-3 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0">
          <p className="text-xs uppercase tracking-wide text-[var(--muted-foreground)]">
            {verticalLabel}
          </p>
          <h1 className="mt-1 text-xl font-semibold text-[var(--foreground)]">
            {workflowLabel}
          </h1>
        </div>
        {compareHref ? (
          <Link
            href={compareHref}
            data-testid="workflow-compare-cta"
            className="inline-flex items-center gap-1 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-1.5 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40"
          >
            Compare versions
          </Link>
        ) : null}
      </header>

      {versions.length === 0 ? (
        <p data-testid="workflow-versions-empty" className="text-sm text-[var(--muted-foreground)]">
          No versions yet.
        </p>
      ) : (
        <VersionTable
          versions={versions}
          verticalId={verticalId}
          name={name}
          defaultFrom={defaultFrom}
          defaultTo={defaultTo}
        />
      )}
    </div>
  );
}

function VersionTable({
  versions,
  verticalId,
  name,
  defaultFrom,
  defaultTo,
}: {
  versions: VersionListItem[];
  verticalId: string;
  name: string;
  defaultFrom?: string;
  defaultTo?: string;
}) {
  return (
    <ol
      data-testid="workflow-versions-list"
      className="divide-y divide-[var(--border)] overflow-hidden rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)]"
    >
      {versions.map((v) => {
        const label = v.label ?? v.id.slice(0, 8);
        const compareHref =
          defaultTo && defaultTo !== v.id
            ? `/verticals/${verticalId}/workflows/${name}/diff?from=${encodeURIComponent(v.id)}&to=${encodeURIComponent(defaultTo)}`
            : defaultFrom && defaultFrom !== v.id
              ? `/verticals/${verticalId}/workflows/${name}/diff?from=${encodeURIComponent(defaultFrom)}&to=${encodeURIComponent(v.id)}`
              : null;
        return (
          <li
            key={v.id}
            data-testid="workflow-version-row"
            className="flex items-center justify-between gap-3 px-4 py-3 text-sm"
          >
            <div className="min-w-0 flex-1">
              <p className="font-mono font-medium text-[var(--foreground)]">
                {label}
                {v.is_head ? (
                  <span className="ml-2 rounded-full bg-[var(--accent)]/40 px-2 py-0.5 text-[10px] uppercase tracking-wide text-[var(--accent-foreground)]">
                    head
                  </span>
                ) : null}
              </p>
              {v.created_at ? (
                <p className="text-xs text-[var(--muted-foreground)]">{v.created_at}</p>
              ) : null}
            </div>
            {compareHref ? (
              <Link
                href={compareHref}
                className="text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline"
              >
                Compare with head
              </Link>
            ) : null}
          </li>
        );
      })}
    </ol>
  );
}

async function fetchVersions(verticalId: string, name: string): Promise<VersionListItem[]> {
  const path = `/api/v1/verticals/${encodeURIComponent(verticalId)}/workflows/${encodeURIComponent(name)}/versions`;
  try {
    const result = await apiGet<{ items: VersionListItem[] } | VersionListItem[]>(path);
    if (Array.isArray(result)) return result;
    return result.items ?? [];
  } catch (err) {
    if (err instanceof ApiError) {
      if (err.status === 404) notFound();
      throw err;
    }
    throw err;
  }
}
