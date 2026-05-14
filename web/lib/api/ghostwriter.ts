/**
 * Ghostwriter API client (Track L brief: L6).
 *
 * Backs the `/workers/<id>/skills/new` prompt-entry surface.
 * Mirrors the mock-fallback contract used by `skill-ir.ts`:
 *
 *   - `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → return a mock IR instantly.
 *   - 404 / network error in dev → mock IR so the UI is still testable.
 *   - Strict production (`NODE_ENV === 'production'` AND
 *     `NEXT_PUBLIC_ENV === 'production'`) propagates all errors.
 *
 * Gateway endpoint: `POST /api/v1/skills/draft-from-prompt`
 *   body:     { "prompt": "<≥15 chars>" }
 *   response: { skillId, tenantId, version, irHash, graph, source }
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import type { SkillGraph } from "@/lib/types/skill-ir";

export type GhostwriterSource = "gateway" | "mock-offline" | "mock-bypass";

export interface GhostwriterDraft {
  skillId: string;
  tenantId?: string;
  version: string;
  irHash?: string;
  graph: SkillGraph;
  source: GhostwriterSource;
}

interface GatewayPayload {
  skillId: string;
  tenantId?: string;
  version: string;
  irHash?: string;
  graph: SkillGraph;
}

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404 || err.status >= 500;
  return err instanceof TypeError;
}

/**
 * Deterministic 5-node mock IR matching the offline gateway default.
 * Used in dev when the gateway is unreachable so the UI can still
 * round-trip to the L4 canvas viewer.
 */
function mockDraft(prompt: string, source: GhostwriterSource): GhostwriterDraft {
  const skillId = mockUuid(prompt);
  const description = prompt.trim().slice(0, 280);
  const graph: SkillGraph = {
    entry: "n1-fetch",
    nodes: [
      {
        id: "n1-fetch",
        label: "Fetch input",
        kind: { type: "fetch", target: "inputs://mock", method: "GET" },
        annotations: {},
      },
      {
        id: "n2-transform",
        label: "Normalise records",
        kind: { type: "transform", template: "normalise(input)" },
        annotations: {},
      },
      {
        id: "n3-decide",
        label: "Decide whether human review is needed",
        kind: { type: "condition", expression: "needs_review" },
        annotations: {},
      },
      {
        id: "n4-approve",
        label: "Await operator approval",
        kind: {
          type: "await-approval",
          approverRole: "operator",
          timeoutSeconds: 3600,
        },
        annotations: {},
      },
      {
        id: "n5-act",
        label: "Carry out the action",
        kind: { type: "action", instruction: description || "Drafted skill" },
        annotations: {},
      },
    ],
    edges: [
      { id: "e1", from: "n1-fetch", to: "n2-transform", cond: { kind: "on-success" } },
      { id: "e2", from: "n2-transform", to: "n3-decide", cond: { kind: "on-success" } },
      { id: "e3", from: "n3-decide", to: "n4-approve", cond: { kind: "on-pattern", pattern: "needs_review" } },
      { id: "e4", from: "n4-approve", to: "n5-act", cond: { kind: "on-success" } },
      { id: "e5", from: "n3-decide", to: "n5-act", cond: { kind: "on-failure" } },
    ],
    variables: [],
    metadata: {
      name: description.slice(0, 64) || "Drafted skill",
      version: "1.0.0",
      description,
      source: "ghostwriter",
    },
  };
  return { skillId, version: "1.0.0", graph, source };
}

function mockUuid(seed: string): string {
  // Tiny deterministic-ish stub so dev links round-trip; not a real UUID.
  const hex = Array.from(seed)
    .map((c) => c.charCodeAt(0).toString(16).padStart(2, "0"))
    .join("")
    .padEnd(32, "0")
    .slice(0, 32);
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20, 32),
  ].join("-");
}

export async function draftSkillFromPrompt(prompt: string): Promise<GhostwriterDraft> {
  if (isAuthBypassed()) return mockDraft(prompt, "mock-bypass");
  try {
    const payload = await apiPost<GatewayPayload, { prompt: string }>(
      "/api/v1/skills/draft-from-prompt",
      { prompt },
    );
    return {
      skillId: payload.skillId,
      tenantId: payload.tenantId,
      version: payload.version,
      irHash: payload.irHash,
      graph: payload.graph,
      source: "gateway",
    };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockDraft(prompt, "mock-offline");
    }
    throw err;
  }
}
