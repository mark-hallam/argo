/**
 * Skill IR — TypeScript mirror of `crates/argo-skill/src/ir/types.rs`.
 *
 * Hand-maintained. Rust is canonical; this file MUST match the Rust
 * schema field-for-field. The L1 brief's CONTRACT-tier guarantee is
 * that downstream Track L consumers (L4 canvas viewer, L4-gw gateway
 * endpoint, L7 editor, L9 diff view, L10 export/import, L16
 * marketplace templates) can rely on these names not shifting.
 *
 * Serialization rules locked at L1:
 *  - All struct field names: camelCase.
 *  - NodeKind: internally tagged on `type` (kebab-case).
 *  - EdgeCond: internally tagged on `kind` (kebab-case).
 *  - NodePolicy / VariableScope: kebab-case scalar enums.
 *
 * Source: streams/L1-skill-ir-schema.md.
 */

/** Stable identifier for a Node within a SkillGraph. Opaque string. */
export type NodeId = string;

/** Stable identifier for an Edge within a SkillGraph. Opaque string. */
export type EdgeId = string;

/** Execution policy — autonomous vs ask-operator. */
export type NodePolicy = "auto" | "ask";

/** Where a variable is visible. */
export type VariableScope = "graph" | "node";

/** MCP tool bound to an Action or Fetch node. */
export interface ToolBinding {
  mcpServer: string;
  toolName: string;
}

/** Per-node observability + policy metadata. All fields optional. */
export interface NodeAnnotations {
  confidence?: number;
  sampleCount?: number;
  lastUsedAt?: string;
  successRate?: number;
  policy?: NodePolicy;
  toolBinding?: ToolBinding;
}

/** One arm of a Branch node. `pattern` absent = catch-all. */
export interface BranchArm {
  label: string;
  pattern?: string;
}

/** Reference to another skill (skill id + version). */
export interface SubSkillRef {
  skillId: string;
  version: string;
}

/**
 * The nine canonical node kinds — discriminated union on `type`.
 *
 * Variants must stay aligned with the Rust `NodeKind` enum:
 *   action / condition / branch / await-approval / fetch / transform /
 *   sub-skill-call / loop / try-catch.
 */
export type NodeKind =
  | { type: "action"; instruction: string }
  | { type: "condition"; expression: string }
  | { type: "branch"; arms: BranchArm[] }
  | {
      type: "await-approval";
      approverRole: string;
      timeoutSeconds?: number;
    }
  | { type: "fetch"; target: string; method?: string }
  | { type: "transform"; template: string }
  | { type: "sub-skill-call"; target: SubSkillRef }
  | { type: "loop"; iterVar: string; body: NodeId[] }
  | { type: "try-catch"; tryBody: NodeId[]; catchBody: NodeId[] };

/** Edge transition condition — discriminated union on `kind`. */
export type EdgeCond =
  | { kind: "always" }
  | { kind: "on-success" }
  | { kind: "on-failure" }
  | { kind: "on-pattern"; pattern: string };

/** A node — kind-specific config plus shared id/label/annotations. */
export interface Node {
  id: NodeId;
  label: string;
  kind: NodeKind;
  annotations: NodeAnnotations;
}

/** A directed transition between two nodes. */
export interface Edge {
  id: EdgeId;
  from: NodeId;
  to: NodeId;
  cond: EdgeCond;
}

/** A scoped input or output variable that flows through edges. */
export interface Variable {
  name: string;
  varType: string;
  scope: VariableScope;
}

/** Graph-level metadata — skill identity + provenance hints. */
export interface SkillGraphMetadata {
  name?: string;
  version?: string;
  description?: string;
  source?: string;
}

/**
 * The canonical structured representation of a skill. Every Track L
 * consumer reads or writes this shape. JSON serialization uses
 * camelCase field names for TS interop.
 */
export interface SkillGraph {
  entry: NodeId;
  nodes: Node[];
  edges: Edge[];
  variables: Variable[];
  metadata: SkillGraphMetadata;
}
