/**
 * Server-side metadata resolver for workflow `<title>`/OG cards.
 *
 * Used by `app/(app)/workflows/[verticalId]/[name]/layout.tsx` (and the
 * sibling `build/layout.tsx`) so SSR stamps the workflow-specific title
 * before the client component takes over. Before this brief the viewer
 * + builder were `"use client"` with no `generateMetadata`, so SSR
 * stamped the parent `/workflows` title and share-link previews showed
 * the catalog card instead of the specific workflow.
 *
 * Reads exclusively from the seed catalogue; no network hop in
 * `generateMetadata` (cold-load latency would widen the skeleton-flash
 * window, not narrow it). The seed is the source of truth for
 * canonical workflow names on the v1.0 demo path. Falls back to a
 * humanised slug when the key is unknown so the title still reads
 * cleanly for fresh workflows that aren't in the seed yet.
 *
 * Split from `workflows.ts` to keep that module under the 500-LOC
 * ceiling — the helper is pure (no fetch), so a separate file is the
 * right home rather than padding the API client further.
 */
import { SAMPLE_WORKFLOW_SPECS } from "@/lib/sample-workflows";

export interface WorkflowDisplayMetadata {
  title: string;
  description: string;
}

export function resolveWorkflowDisplayMetadata(
  verticalId: string,
  name: string,
): WorkflowDisplayMetadata {
  const seed = SAMPLE_WORKFLOW_SPECS[`${verticalId}/${name}`];
  if (seed) {
    return { title: humanizeSlug(seed.name), description: seed.description };
  }
  return {
    title: humanizeSlug(name),
    description: `Workflow ${name} in ${verticalId}.`,
  };
}

function humanizeSlug(slug: string): string {
  const cleaned = slug.replace(/[_-]+/g, " ").trim();
  if (!cleaned) return slug;
  return cleaned.charAt(0).toUpperCase() + cleaned.slice(1);
}
