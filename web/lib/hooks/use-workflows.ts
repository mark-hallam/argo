"use client";

import { useQuery } from "@tanstack/react-query";
import {
  fetchAllWorkflows,
  fetchWorkflow,
  type FetchAllWorkflowsResult,
  type FetchWorkflowResult,
} from "@/lib/api/workflows";

const WORKFLOWS_KEY = ["workflows"] as const;
const WORKFLOW_KEY = ["workflow"] as const;

/**
 * All workflows across all verticals, served as `WorkflowSummary[]` for
 * the `/workflows` catalog grid. The fetcher fans the per-vertical
 * call out in parallel and falls through to the local seed when the
 * gateway returns nothing.
 */
export function useWorkflows() {
  return useQuery<FetchAllWorkflowsResult>({
    queryKey: WORKFLOWS_KEY,
    queryFn: fetchAllWorkflows,
  });
}

/**
 * One workflow including step spec for the visual viewer. Disabled
 * while either route param is empty so the page can hold the
 * suspense boundary without firing a query.
 */
export function useWorkflow(verticalId: string | null, name: string | null) {
  return useQuery<FetchWorkflowResult>({
    queryKey: [...WORKFLOW_KEY, verticalId, name] as const,
    queryFn: () => {
      if (!verticalId || !name) {
        throw new Error("workflow vertical + name required");
      }
      return fetchWorkflow(verticalId, name);
    },
    enabled: !!verticalId && !!name,
  });
}
