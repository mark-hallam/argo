import type {
  Edge,
  EdgeCond,
  Node,
  NodeAnnotations,
  NodeId,
  NodeKind,
  SkillGraph,
  SkillGraphMetadata,
  SubSkillRef,
  ToolBinding,
  Variable,
  VariableScope,
} from "./types";

export interface BuilderNodeOptions {
  annotations?: NodeAnnotations;
  toolBinding?: ToolBinding;
}

export interface SkillBuilderInput {
  name?: string;
  version?: string;
  description?: string;
  source?: string;
}

type NodeOptions = BuilderNodeOptions | undefined;

export function createSkill(input: SkillBuilderInput = {}): SkillBuilder {
  return new SkillBuilder(input);
}

export class SkillBuilder {
  private readonly nodes = new Map<NodeId, Node>();
  private readonly edges: Edge[] = [];
  private readonly vars = new Map<string, Variable>();
  private entryId?: NodeId;
  private readonly metadata: SkillGraphMetadata;

  constructor(input: SkillBuilderInput = {}) {
    this.metadata = { ...input };
  }

  action(id: NodeId, label: string, instruction: string, options?: NodeOptions): this {
    return this.addNode(id, label, { type: "action", instruction }, options);
  }

  fetch(id: NodeId, label: string, target: string, method?: string, options?: NodeOptions): this {
    return this.addNode(id, label, { type: "fetch", target, method }, options);
  }

  transform(id: NodeId, label: string, template: string, options?: NodeOptions): this {
    return this.addNode(id, label, { type: "transform", template }, options);
  }

  condition(id: NodeId, label: string, expression: string, options?: NodeOptions): this {
    return this.addNode(id, label, { type: "condition", expression }, options);
  }

  approval(
    id: NodeId,
    label: string,
    approverRole: string,
    timeoutSeconds?: number,
    options?: NodeOptions,
  ): this {
    return this.addNode(
      id,
      label,
      { type: "await-approval", approverRole, timeoutSeconds },
      options,
    );
  }

  branch(
    id: NodeId,
    label: string,
    arms: Array<{ label: string; pattern?: string }>,
    options?: NodeOptions,
  ): this {
    return this.addNode(id, label, { type: "branch", arms }, options);
  }

  subSkill(id: NodeId, label: string, target: SubSkillRef, options?: NodeOptions): this {
    return this.addNode(id, label, { type: "sub-skill-call", target }, options);
  }

  loop(id: NodeId, label: string, iterVar: string, body: NodeId[], options?: NodeOptions): this {
    return this.addNode(id, label, { type: "loop", iterVar, body }, options);
  }

  tryCatch(
    id: NodeId,
    label: string,
    tryBody: NodeId[],
    catchBody: NodeId[],
    options?: NodeOptions,
  ): this {
    return this.addNode(id, label, { type: "try-catch", tryBody, catchBody }, options);
  }

  edge(id: string, from: NodeId, to: NodeId, cond: EdgeCond = { kind: "always" }): this {
    if (this.edges.some((edge) => edge.id === id)) {
      throw new Error(`duplicate edge id: ${id}`);
    }
    this.edges.push({ id, from, to, cond });
    return this;
  }

  variable(name: string, varType: string, scope: VariableScope = "graph"): this {
    if (this.vars.has(name)) {
      throw new Error(`duplicate variable: ${name}`);
    }
    this.vars.set(name, { name, varType, scope });
    return this;
  }

  start(id: NodeId): this {
    this.entryId = id;
    return this;
  }

  build(): SkillGraph {
    const graph: SkillGraph = {
      entry: this.entryId ?? this.nodes.keys().next().value ?? "",
      nodes: [...this.nodes.values()],
      edges: [...this.edges],
      variables: [...this.vars.values()],
      metadata: { ...this.metadata },
    };
    validateSkillGraph(graph);
    return graph;
  }

  private addNode(id: NodeId, label: string, kind: NodeKind, options?: NodeOptions): this {
    if (this.nodes.has(id)) {
      throw new Error(`duplicate node id: ${id}`);
    }
    const annotations = { ...options?.annotations };
    if (options?.toolBinding) {
      annotations.toolBinding = options.toolBinding;
    }
    this.nodes.set(id, { id, label, kind, annotations });
    if (!this.entryId) {
      this.entryId = id;
    }
    return this;
  }
}

export function validateSkillGraph(graph: SkillGraph): SkillGraph {
  const nodeIds = new Set(graph.nodes.map((node) => node.id));
  if (!graph.entry || !nodeIds.has(graph.entry)) {
    throw new Error(`entry node not found: ${graph.entry || "(empty)"}`);
  }
  for (const edge of graph.edges) {
    if (!nodeIds.has(edge.from) || !nodeIds.has(edge.to)) {
      throw new Error(`edge ${edge.id} references a missing node`);
    }
  }
  for (const node of graph.nodes) {
    validateNodeKind(node);
    const confidence = node.annotations.confidence;
    if (confidence !== undefined && (confidence < 0 || confidence > 1)) {
      throw new Error(`node ${node.id} confidence must be 0..1`);
    }
  }
  return graph;
}

function validateNodeKind(node: Node): void {
  const kind = node.kind;
  if (kind.type === "branch" && kind.arms.length === 0) {
    throw new Error(`branch node ${node.id} needs at least one arm`);
  }
  if (kind.type === "loop" && kind.body.length === 0) {
    throw new Error(`loop node ${node.id} needs a body`);
  }
  if (kind.type === "try-catch" && (kind.tryBody.length === 0 || kind.catchBody.length === 0)) {
    throw new Error(`try-catch node ${node.id} needs try and catch bodies`);
  }
}

export function sampleSkills(): SkillGraph[] {
  return [
    createSkill({ name: "recruiter-screen", version: "1.0.0" })
      .fetch("inbox", "Read inbound candidate mail", "gmail.messages.unread")
      .condition("qualified", "Candidate matches role", "candidate.years >= 4")
      .approval("approve", "Ask recruiter lead", "recruiter-lead", 3600)
      .edge("e1", "inbox", "qualified")
      .edge("e2", "qualified", "approve", { kind: "on-success" })
      .variable("candidate", "Candidate", "graph")
      .build(),
    createSkill({ name: "ap-invoice-check", version: "1.0.0" })
      .fetch("invoice", "Load invoice", "document.invoice")
      .transform("normalize", "Normalize totals", "{{invoice.total}}")
      .approval("approve", "Route exception", "ap-manager")
      .edge("e1", "invoice", "normalize")
      .edge("e2", "normalize", "approve", { kind: "on-pattern", pattern: "exception" })
      .build(),
    createSkill({ name: "support-tier1-triage", version: "1.0.0" })
      .fetch("ticket", "Read ticket", "zendesk.ticket")
      .branch("route", "Classify priority", [
        { label: "billing", pattern: "billing|invoice" },
        { label: "technical", pattern: "error|bug" },
      ])
      .action("reply", "Draft response", "Write a concise first response")
      .edge("e1", "ticket", "route")
      .edge("e2", "route", "reply")
      .build(),
    createSkill({ name: "vendor-renewal", version: "1.0.0" })
      .action("score", "Score vendor", "Compare price, SLA, and incidents")
      .subSkill("negotiate", "Draft negotiation", {
        skillId: "canonical.negotiate-renewal",
        version: "1.0.0",
      })
      .edge("e1", "score", "negotiate")
      .build(),
    createSkill({ name: "deal-desk-validation", version: "1.0.0" })
      .action("validate", "Validate quote", "Check discount policy")
      .tryCatch("guard", "Handle policy failure", ["validate"], ["escalate"])
      .action("escalate", "Escalate non-standard terms", "Notify deal desk")
      .edge("e1", "guard", "escalate", { kind: "on-failure" })
      .start("guard")
      .build(),
  ];
}
