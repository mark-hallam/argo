/**
 * Workflow rollback API client (Stream `web-workflow-rollback-feature`).
 *
 * Wraps `POST /api/v1/verticals/{verticalId}/workflows/{name}/rollback`,
 * which produces an additive new version copied byte-for-byte from a
 * prior one and promotes it to head in one DB transaction.
 */
import { apiGet, apiPost } from "@/lib/api/client";

export interface RollbackResponse {
  id: string;
  version: number;
}

export interface RollbackArgs {
  verticalId: string;
  name: string;
  targetVersionId: string;
}

export async function rollbackWorkflow(args: RollbackArgs): Promise<RollbackResponse> {
  const path = `/api/v1/verticals/${encodeURIComponent(args.verticalId)}/workflows/${encodeURIComponent(
    args.name,
  )}/rollback`;
  return apiPost<RollbackResponse>(path, { target_version_id: args.targetVersionId });
}

export interface WorkflowVersionItem {
  id: string;
  version: number;
  parent_id: string | null;
  lifecycle_state: "draft" | "active" | "archived" | string;
  created_at: string;
  modified_by: string;
  definition_toml: string;
}

interface VersionListEnvelope {
  vertical_id?: string;
  name?: string;
  count?: number;
  items?: WorkflowVersionItem[];
}

export async function listWorkflowVersions(args: {
  verticalId: string;
  name: string;
}): Promise<WorkflowVersionItem[]> {
  const path = `/api/v1/verticals/${encodeURIComponent(args.verticalId)}/workflows/${encodeURIComponent(
    args.name,
  )}/versions`;
  const envelope = await apiGet<VersionListEnvelope>(path);
  return envelope.items ?? [];
}
