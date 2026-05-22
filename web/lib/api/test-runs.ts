/**
 * Workflow test-run history API client (Stream
 * `claude-followup-pr-2021-test-run-gateway-persist`).
 *
 * Wraps:
 *   - `POST /api/v1/verticals/{verticalId}/workflows/{name}/test_runs`
 *     — persist one completed test run from the builder's "Test run"
 *     overlay. Body mirrors `useTestRun.ts::CompletedTestRun`.
 *   - `GET  /api/v1/verticals/{verticalId}/workflows/{name}/test_runs?limit=N`
 *     — fetch the last N runs newest-first.
 *
 * The POST is best-effort: a failure to persist must not break the
 * in-memory run overlay. The GET is consumed by the builder mount-
 * time hydration so the history panel can seed from server state on
 * first paint.
 */
import { apiGet, apiPost } from "@/lib/api/client";
import type {
  CompletedTestRun,
  RunSummary,
  StepInput,
  StepRunState,
} from "@/app/(app)/workflows/[verticalId]/[name]/build/useTestRun";

export type TestRunStatus = "complete" | "failed";

export interface TestRunPersistArgs {
  verticalId: string;
  name: string;
  run: CompletedTestRun;
}

export interface TestRunPersistResponse {
  id: string;
  vertical_id: string;
  workflow_name: string;
}

export async function persistTestRun(
  args: TestRunPersistArgs,
): Promise<TestRunPersistResponse> {
  const path = `/api/v1/verticals/${encodeURIComponent(
    args.verticalId,
  )}/workflows/${encodeURIComponent(args.name)}/test_runs`;
  return apiPost<TestRunPersistResponse>(path, {
    status: args.run.status,
    started_at: args.run.startedAt,
    ended_at: args.run.endedAt,
    summary: args.run.summary,
    step_states: args.run.stepStates,
    transport_error: args.run.transportError,
  });
}

export interface TestRunHistoryItem {
  id: string;
  vertical_id: string;
  workflow_name: string;
  started_at: number;
  ended_at: number;
  status: TestRunStatus | string;
  /** Opaque JSON string — `RunSummary` shape or `"null"`. */
  summary_json: string;
  /** Opaque JSON string — `Record<stepId, StepRunState>` shape. */
  step_states_json: string;
  transport_error: string | null;
}

interface TestRunHistoryEnvelope {
  vertical_id?: string;
  workflow_name?: string;
  count?: number;
  items?: TestRunHistoryItem[];
}

export async function listTestRuns(args: {
  verticalId: string;
  name: string;
  limit?: number;
}): Promise<TestRunHistoryItem[]> {
  const limit = args.limit ?? 10;
  const path = `/api/v1/verticals/${encodeURIComponent(
    args.verticalId,
  )}/workflows/${encodeURIComponent(args.name)}/test_runs?limit=${limit}`;
  const envelope = await apiGet<TestRunHistoryEnvelope>(path);
  return envelope.items ?? [];
}

/**
 * Decode one history row's opaque JSON blobs back into a
 * `CompletedTestRun`-shaped record. Tolerant of malformed blobs —
 * returns null fields rather than throwing, so a single corrupt row
 * never breaks the history panel.
 */
export function hydrateHistoryItem(item: TestRunHistoryItem): {
  id: string;
  startedAt: number;
  endedAt: number;
  status: TestRunStatus | string;
  transportError: string | null;
  summary: RunSummary | null;
  stepStates: Record<string, StepRunState>;
} {
  let summary: RunSummary | null = null;
  try {
    const parsed = JSON.parse(item.summary_json) as unknown;
    if (parsed && typeof parsed === "object") {
      summary = parsed as RunSummary;
    }
  } catch {
    summary = null;
  }
  let stepStates: Record<string, StepRunState> = {};
  try {
    const parsed = JSON.parse(item.step_states_json) as unknown;
    if (parsed && typeof parsed === "object") {
      stepStates = parsed as Record<string, StepRunState>;
    }
  } catch {
    stepStates = {};
  }
  return {
    id: item.id,
    startedAt: item.started_at,
    endedAt: item.ended_at,
    status: item.status,
    transportError: item.transport_error,
    summary,
    stepStates,
  };
}

// Re-export the input types from useTestRun for downstream callers
// that want to type a CompletedTestRun without a separate import.
export type { CompletedTestRun, RunSummary, StepInput, StepRunState };
