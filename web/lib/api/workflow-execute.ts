/**
 * Workflow execute API client (Stream
 * `mission-workflow-execute-route`).
 *
 * Wraps `POST /api/v1/verticals/{verticalId}/workflows/{name}/execute`
 * — the production "Run now" surface backing the SPA's workflow
 * detail page action. The gateway resolves the workflow_definition
 * head, executes the DAG against the workflow runner, persists one
 * `workflow_run` row, and emits one `WorkflowExecuteDispatched`
 * audit row.
 *
 * Returns the synchronous run report. Mission claim 3 (audit
 * 2026-05-23T03-39 §2 P1 #4) flips LIVE once this endpoint is
 * reachable from the SPA.
 */
import { apiPost } from "@/lib/api/client";

export interface WorkflowExecuteRequest {
  verticalId: string;
  name: string;
  inputs?: Record<string, unknown>;
  idempotencyKey?: string;
}

export interface WorkflowExecuteResponse {
  run_id: string;
  vertical_id: string;
  workflow_name: string;
  ok: boolean;
  task_verdicts: unknown;
  dispatched_at: string;
}

export async function executeWorkflow(
  args: WorkflowExecuteRequest,
): Promise<WorkflowExecuteResponse> {
  const path = `/api/v1/verticals/${encodeURIComponent(
    args.verticalId,
  )}/workflows/${encodeURIComponent(args.name)}/execute`;
  return apiPost<WorkflowExecuteResponse>(path, {
    inputs: args.inputs ?? {},
    idempotency_key: args.idempotencyKey,
  });
}
