import { getTemplate } from "@/lib/workflow-templates";

import { SEED_WORKFLOWS } from "./seed-workflows";
import type { WorkflowSpec } from "./types";

/**
 * Pure resolver for the builder's "what workflow am I editing?" decision.
 *
 * Priority:
 *   1. `?from_template=` → in-memory template copy (never hits gateway).
 *   2. Gateway hit (data.workflow non-null) → render that.
 *   3. Gateway miss in demo mode → fall back to the rich SEED_WORKFLOWS
 *      so the v1.0 recorded demo replay still plays when the gateway
 *      has no head row for this key.
 *   4. Otherwise → undefined (caller shows empty / not-found state).
 *
 * Demo-mode gate matches the pattern shipped in #2054 for the other SPA
 * API surfaces (`lib/api/{audit,team,cohorts,approvals}.ts`).
 */
export interface ResolveBuilderWorkflowInput {
  verticalId: string;
  name: string;
  templateId: string | null;
  /** TanStack Query `data` (undefined while loading). */
  data: { workflow: WorkflowSpec | null } | undefined;
  demoMode: boolean;
}

export function resolveBuilderWorkflowSource(
  input: ResolveBuilderWorkflowInput,
): WorkflowSpec | undefined {
  if (input.templateId) {
    const tpl = getTemplate(input.templateId);
    if (!tpl) return undefined;
    return {
      name: tpl.name,
      vertical: tpl.vertical,
      description: tpl.description,
      version: "draft",
      risk: tpl.risk,
      steps: tpl.steps,
    };
  }
  const fromGateway = input.data?.workflow;
  if (fromGateway) return fromGateway;
  if (input.demoMode) return SEED_WORKFLOWS[`${input.verticalId}/${input.name}`];
  return undefined;
}
