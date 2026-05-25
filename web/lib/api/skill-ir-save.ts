import { apiPost } from "@/lib/api/client";
import type { SkillGraph } from "@/lib/types/skill-ir";

export interface SaveSkillIrResult {
  skillId: string;
  tenantId?: string;
  version: string;
  irHash?: string;
  graph: SkillGraph;
  abTestScheduled: boolean;
  abTestRunId?: string | null;
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

export async function saveSkillIr(
  skillId: string,
  graph: SkillGraph,
): Promise<SaveSkillIrResult> {
  if (isAuthBypassed()) {
    return {
      skillId,
      version: `local-${Date.now()}`,
      graph,
      abTestScheduled: true,
      abTestRunId: "mock-ab-run",
    };
  }
  return apiPost<SaveSkillIrResult, { graph: SkillGraph }>(
    `/api/v1/skills/${encodeURIComponent(skillId)}/ir`,
    { graph },
  );
}
