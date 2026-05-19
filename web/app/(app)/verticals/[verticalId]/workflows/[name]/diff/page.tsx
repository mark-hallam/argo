/**
 * `/verticals/<v>/workflows/<n>/diff?from=<id>&to=<id>` — read-only
 * side-by-side comparison of two workflow versions.
 *
 * Server-rendered: the route fetches both versions through the
 * gateway (`mission-pm-workflow-edit-api`, P1 #6) using the viewer's
 * bearer token, then hands the TOML strings to `<WorkflowDiff>` for
 * pure rendering. The diff computation runs at request time.
 */

import { notFound } from "next/navigation";
import Link from "next/link";
import { apiGet, ApiError } from "@/lib/api/client";
import { Breadcrumb, BreadcrumbItem, BreadcrumbSeparator } from "@/components/ui/Breadcrumb";
import { WorkflowDiff, type VersionMeta } from "@/components/skycot/WorkflowDiff";

interface VersionResponse {
  id: string;
  parent_id?: string | null;
  toml: string;
  created_at?: string;
  created_by?: string;
  label?: string;
}

interface DiffPageProps {
  params: Promise<{ verticalId: string; name: string }>;
  searchParams: Promise<{ from?: string; to?: string }>;
}

export default async function WorkflowDiffPage({ params, searchParams }: DiffPageProps) {
  const { verticalId, name } = await params;
  const { from, to } = await searchParams;
  if (!from || !to) notFound();

  const [fromVersion, toVersion] = await Promise.all([
    fetchVersion(verticalId, name, from),
    fetchVersion(verticalId, name, to),
  ]);

  const fromMeta: VersionMeta = toMeta(fromVersion);
  const toMetaValue: VersionMeta = toMeta(toVersion);
  const verticalLabel = decodeURIComponent(verticalId);
  const workflowLabel = decodeURIComponent(name);

  return (
    <div data-testid="workflow-diff-page" className="space-y-4">
      <Breadcrumb>
        <BreadcrumbItem href={`/verticals/${verticalId}/workflows/${name}`}>
          {workflowLabel}
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem current>Diff</BreadcrumbItem>
      </Breadcrumb>

      <WorkflowDiff
        fromToml={fromVersion.toml}
        toToml={toVersion.toml}
        fromMeta={fromMeta}
        toMeta={toMetaValue}
      />

      <Link
        href={`/verticals/${verticalId}/workflows/${name}`}
        className="inline-flex text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline"
        data-testid="workflow-diff-back"
      >
        Back to {workflowLabel} in {verticalLabel}
      </Link>
    </div>
  );
}

function toMeta(version: VersionResponse): VersionMeta {
  return {
    id: version.id,
    label: version.label ?? version.id.slice(0, 8),
    createdAt: version.created_at,
  };
}

async function fetchVersion(
  verticalId: string,
  name: string,
  versionId: string,
): Promise<VersionResponse> {
  const path = `/api/v1/verticals/${encodeURIComponent(verticalId)}/workflows/${encodeURIComponent(name)}/versions/${encodeURIComponent(versionId)}`;
  try {
    return await apiGet<VersionResponse>(path);
  } catch (err) {
    if (err instanceof ApiError) {
      if (err.status === 404) notFound();
      // 401 / 403 surface via Next.js error boundary; rethrow so the
      // framework's error.tsx handles them. The gateway's 401 envelope
      // is preserved on err.body for the error boundary's use.
      throw err;
    }
    throw err;
  }
}
