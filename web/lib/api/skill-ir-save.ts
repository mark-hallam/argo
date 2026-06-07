import { apiPost } from "@/lib/api/client";
import { hasRealSession, isDemoLive } from "@/lib/skycot/demo-mode";
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
  // Per-user gate (sprint, 2026-06-07, mock-leak class): a REAL
  // signed-in user (the `skycot-session=1` marker cookie is present) is
  // NEVER served the auth-bypass mock fixtures, even in a demo-flagged
  // deploy. The bypass short-circuit is for ANONYMOUS demo visitors only.
  if (hasRealSession()) return false;
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

export async function saveSkillIr(
  skillId: string,
  graph: SkillGraph,
): Promise<SaveSkillIrResult> {
  if (isAuthBypassed() && !isDemoLive()) {
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
