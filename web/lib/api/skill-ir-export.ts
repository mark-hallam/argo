import { apiGet, apiPost } from "@/lib/api/client";
import type { SkillGraph } from "@/lib/types/skill-ir";

export const SKILL_IR_BUNDLE_SCHEMA = "skycot.skill_ir.bundle.v1";

export interface SkillIrBundle {
  schemaVersion: typeof SKILL_IR_BUNDLE_SCHEMA;
  sourceSkillId: string;
  version: string;
  irHash: string;
  graph: SkillGraph;
}

export interface ImportSkillIrResult {
  skillId: string;
  tenantId?: string;
  version: string;
  irHash: string;
  importedHash: string;
  graph: SkillGraph;
  abTestScheduled: boolean;
  abTestRunId?: string | null;
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

export function buildLocalSkillIrBundle(
  skillId: string,
  version: string,
  graph: SkillGraph,
  irHash = "local-preview",
): SkillIrBundle {
  return {
    schemaVersion: SKILL_IR_BUNDLE_SCHEMA,
    sourceSkillId: skillId,
    version,
    irHash,
    graph,
  };
}

export function parseSkillIrBundle(raw: unknown): SkillIrBundle {
  if (!raw || typeof raw !== "object") throw new Error("Import file must be a JSON object.");
  const bundle = raw as Partial<SkillIrBundle>;
  if (bundle.schemaVersion !== SKILL_IR_BUNDLE_SCHEMA) {
    throw new Error("Unsupported skill bundle schema.");
  }
  if (!bundle.sourceSkillId || !bundle.version || !bundle.irHash) {
    throw new Error("Skill bundle metadata is incomplete.");
  }
  const graph = bundle.graph;
  if (!graph || !Array.isArray(graph.nodes) || !Array.isArray(graph.edges) || !graph.entry) {
    throw new Error("Skill bundle graph is incomplete.");
  }
  if (!graph.nodes.some((node) => node.id === graph.entry)) {
    throw new Error("Skill bundle graph entry does not match a node.");
  }
  return bundle as SkillIrBundle;
}

export async function exportSkillIr(
  skillId: string,
  version?: string,
  fallbackGraph?: SkillGraph,
  fallbackHash?: string,
): Promise<SkillIrBundle> {
  if (isAuthBypassed()) {
    if (!fallbackGraph || !version) throw new Error("Local export requires a loaded graph.");
    return buildLocalSkillIrBundle(skillId, version, fallbackGraph, fallbackHash);
  }
  const path =
    `/api/v1/skills/${encodeURIComponent(skillId)}/ir/export` +
    (version ? `?version=${encodeURIComponent(version)}` : "");
  return apiGet<SkillIrBundle>(path);
}

export async function importSkillIr(
  skillId: string,
  bundle: SkillIrBundle,
): Promise<ImportSkillIrResult> {
  if (isAuthBypassed()) {
    return {
      skillId,
      version: `import-${Date.now()}`,
      irHash: bundle.irHash,
      importedHash: bundle.irHash,
      graph: bundle.graph,
      abTestScheduled: true,
      abTestRunId: "mock-import-ab-run",
    };
  }
  return apiPost<ImportSkillIrResult, { bundle: SkillIrBundle }>(
    `/api/v1/skills/${encodeURIComponent(skillId)}/ir/import`,
    { bundle },
  );
}
