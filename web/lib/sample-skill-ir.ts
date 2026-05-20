/**
 * Bundled sample Skill IR — feeds the L4 canvas viewer when the
 * gateway endpoint (L4-gw) hasn't merged yet, when a 404 fires in
 * dev, or when the SPA runs with `ARGO_GATEWAY_AUTH_BYPASS=1`.
 *
 * The "Recruiter screening" sample exercises all 9 NodeKinds so the
 * viewer's per-kind visual + annotations panel can be eyeballed
 * without spinning up the gateway. Mirrors the shape returned by
 * `fetchSkillIr`.
 */

import type { SkillGraph } from "@/lib/types/skill-ir";

export interface SampleSkillIr {
  graph: SkillGraph;
  version: string;
  irHash: string;
}

const RECRUITER_SCREENING: SampleSkillIr = {
  version: "v0.3.0",
  irHash: "sha256:a1b2c3d4e5f600000000000000000000000000000000000000000000000000aa",
  graph: {
    entry: "n_fetch_application",
    nodes: [
      {
        id: "n_fetch_application",
        label: "Fetch application",
        kind: { type: "fetch", target: "ats:applications/{id}", method: "GET" },
        annotations: {
          confidence: 0.96,
          sampleCount: 142,
          successRate: 0.99,
          lastUsedAt: "2026-05-13T18:42:00Z",
          policy: "auto",
          toolBinding: { mcpServer: "ats", toolName: "applications.get" },
        },
      },
      {
        id: "n_check_keywords",
        label: "Check role keywords",
        kind: { type: "condition", expression: "resume.contains_any(role.keywords)" },
        annotations: { confidence: 0.91, sampleCount: 142, successRate: 0.93, policy: "auto" },
      },
      {
        id: "n_branch_outcome",
        label: "Branch on score",
        kind: {
          type: "branch",
          arms: [
            { label: "strong fit", pattern: "score >= 0.8" },
            { label: "weak fit", pattern: "score < 0.4" },
            { label: "borderline" },
          ],
        },
        annotations: { confidence: 0.78, sampleCount: 138, policy: "auto" },
      },
      {
        id: "n_summarize",
        label: "Summarize candidate",
        kind: {
          type: "transform",
          template: "Tier: {{score_band}}. Strengths: {{strengths}}.",
        },
        annotations: { confidence: 0.88, sampleCount: 122, policy: "auto" },
      },
      {
        id: "n_loop_panel",
        label: "Notify each panelist",
        kind: { type: "loop", iterVar: "panelist", body: ["n_action_email"] },
        annotations: { confidence: 0.85, sampleCount: 90, policy: "auto" },
      },
      {
        id: "n_action_email",
        label: "Send email",
        kind: { type: "action", instruction: "Send shortlist email to {{panelist.email}}" },
        annotations: {
          confidence: 0.82,
          sampleCount: 408,
          successRate: 0.91,
          policy: "auto",
          toolBinding: { mcpServer: "gmail", toolName: "messages.send" },
        },
      },
      {
        id: "n_await_decision",
        label: "Await CEO decision",
        kind: {
          type: "await-approval",
          approverRole: "ceo",
          timeoutSeconds: 86400,
        },
        annotations: { confidence: 1.0, sampleCount: 31, policy: "ask" },
      },
      {
        id: "n_subskill_offer",
        label: "Compose offer letter",
        kind: {
          type: "sub-skill-call",
          target: { skillId: "skill_offer_letter", version: "v1.2.0" },
        },
        annotations: { confidence: 0.74, sampleCount: 18, policy: "auto" },
      },
      {
        id: "n_try_send",
        label: "Send offer (with retry)",
        kind: {
          type: "try-catch",
          tryBody: ["n_action_email"],
          catchBody: ["n_action_email"],
        },
        annotations: { confidence: 0.69, sampleCount: 16, successRate: 0.81, policy: "auto" },
      },
    ],
    edges: [
      { id: "e1", from: "n_fetch_application", to: "n_check_keywords", cond: { kind: "on-success" } },
      { id: "e2", from: "n_check_keywords", to: "n_branch_outcome", cond: { kind: "always" } },
      { id: "e3", from: "n_branch_outcome", to: "n_summarize", cond: { kind: "on-pattern", pattern: "strong fit" } },
      { id: "e4", from: "n_summarize", to: "n_loop_panel", cond: { kind: "always" } },
      { id: "e5", from: "n_loop_panel", to: "n_await_decision", cond: { kind: "on-success" } },
      { id: "e6", from: "n_await_decision", to: "n_subskill_offer", cond: { kind: "on-success" } },
      { id: "e7", from: "n_subskill_offer", to: "n_try_send", cond: { kind: "always" } },
      { id: "e8", from: "n_check_keywords", to: "n_branch_outcome", cond: { kind: "on-failure" } },
    ],
    variables: [
      { name: "score_band", varType: "string", scope: "graph" },
      { name: "panelist", varType: "object", scope: "node" },
    ],
    metadata: {
      name: "Screen inbound applications",
      version: "v0.3.0",
      description: "Triage inbound role applications: fetch, score, summarize, route to panel.",
      source: "observation",
    },
  },
};

const SAMPLES: Record<string, SampleSkillIr> = {
  skill_screen_applications: RECRUITER_SCREENING,
  // Default fallback so the viewer demos cleanly when arbitrary skill
  // IDs hit the page during dev. Real skill IDs replace this once the
  // gateway endpoint lands.
  default: RECRUITER_SCREENING,
};

/** Lookup the bundled IR for a skill id, optionally a specific version. */
export function findSampleSkillIr(
  skillId: string,
  version?: string,
): SampleSkillIr | undefined {
  const sample = SAMPLES[skillId] ?? SAMPLES.default;
  if (!sample) return undefined;
  if (version && sample.version !== version) return undefined;
  return sample;
}
