import type {
  EdgeCond,
  NodeAnnotations,
  NodeKind,
  SkillGraph,
  SkillGraphMetadata,
  ToolBinding,
} from "@/lib/types/skill-ir";

interface BuilderOptions {
  annotations?: Omit<NodeAnnotations, "toolBinding">;
  toolBinding?: ToolBinding;
}

export class SkillCodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "SkillCodeError";
  }
}

export function skillGraphToSdkSource(graph: SkillGraph): string {
  validateGraph(graph);
  const lines = [
    'import { createSkill } from "@skycot/skill-sdk";',
    'import type { SkillGraph } from "@skycot/skill-sdk";',
    "",
    `export const skill: SkillGraph = createSkill(${jsonArg(graph.metadata)})`,
  ];
  for (const node of graph.nodes) {
    lines.push(`  .${methodForKind(node.kind)}(${nodeArgs(node.kind, node.id, node.label, node.annotations).join(", ")})`);
  }
  for (const edge of graph.edges) {
    lines.push(`  .edge(${[jsonArg(edge.id), jsonArg(edge.from), jsonArg(edge.to), jsonArg(edge.cond)].join(", ")})`);
  }
  for (const variable of graph.variables) {
    lines.push(
      `  .variable(${[
        jsonArg(variable.name),
        jsonArg(variable.varType),
        jsonArg(variable.scope),
      ].join(", ")})`,
    );
  }
  lines.push(`  .start(${jsonArg(graph.entry)})`);
  lines.push("  .build();");
  return `${lines.join("\n")}\n`;
}

export function sdkSourceToSkillGraph(source: string): SkillGraph {
  const metadata = parseMetadata(source);
  const nodes: SkillGraph["nodes"] = [];
  const edges: SkillGraph["edges"] = [];
  const variables: SkillGraph["variables"] = [];
  let entry = "";

  for (const rawLine of source.split(/\r?\n/)) {
    const line = rawLine.trim().replace(/;$/, "");
    const call = line.match(/^\.(\w+)\((.*)\)$/);
    if (!call) continue;
    const [, method, rawArgs] = call;
    const args = parseArgs(rawArgs);
    switch (method) {
      case "action":
      case "fetch":
      case "transform":
      case "condition":
      case "approval":
      case "branch":
      case "subSkill":
      case "loop":
      case "tryCatch":
        nodes.push(parseNode(method, args));
        break;
      case "edge":
        edges.push(parseEdge(args));
        break;
      case "variable":
        variables.push(parseVariable(args));
        break;
      case "start":
        entry = stringArg(args, 0, "start id");
        break;
      case "build":
        break;
      default:
        throw new SkillCodeError(`Unsupported SDK builder call: ${method}`);
    }
  }

  if (nodes.length === 0) {
    throw new SkillCodeError("No SDK node builder calls found.");
  }
  const graph = {
    entry: entry || nodes[0].id,
    nodes,
    edges,
    variables,
    metadata,
  };
  validateGraph(graph);
  return graph;
}

export function stableSkillGraphHash(graph: SkillGraph): string {
  const text = stableStringify(graph);
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= BigInt(text.charCodeAt(i));
    hash = BigInt.asUintN(64, hash * prime);
  }
  return hash.toString(16).padStart(16, "0");
}

export function roundTripSkillGraph(graph: SkillGraph): {
  source: string;
  graph: SkillGraph;
  inputHash: string;
  outputHash: string;
} {
  const source = skillGraphToSdkSource(graph);
  const parsed = sdkSourceToSkillGraph(source);
  return {
    source,
    graph: parsed,
    inputHash: stableSkillGraphHash(graph),
    outputHash: stableSkillGraphHash(parsed),
  };
}

function methodForKind(kind: NodeKind): string {
  if (kind.type === "await-approval") return "approval";
  if (kind.type === "sub-skill-call") return "subSkill";
  if (kind.type === "try-catch") return "tryCatch";
  return kind.type;
}

function nodeArgs(
  kind: NodeKind,
  id: string,
  label: string,
  annotations: NodeAnnotations,
): string[] {
  const base = [jsonArg(id), jsonArg(label)];
  const options = optionsArg(annotations);
  switch (kind.type) {
    case "action":
      return appendOptional([...base, jsonArg(kind.instruction)], options);
    case "condition":
      return appendOptional([...base, jsonArg(kind.expression)], options);
    case "branch":
      return appendOptional([...base, jsonArg(kind.arms)], options);
    case "await-approval":
      return appendOptional(
        [...base, jsonArg(kind.approverRole), optionalArg(kind.timeoutSeconds)],
        options,
      );
    case "fetch":
      return appendOptional([...base, jsonArg(kind.target), optionalArg(kind.method)], options);
    case "transform":
      return appendOptional([...base, jsonArg(kind.template)], options);
    case "sub-skill-call":
      return appendOptional([...base, jsonArg(kind.target)], options);
    case "loop":
      return appendOptional([...base, jsonArg(kind.iterVar), jsonArg(kind.body)], options);
    case "try-catch":
      return appendOptional([...base, jsonArg(kind.tryBody), jsonArg(kind.catchBody)], options);
  }
}

function appendOptional(args: string[], options?: string): string[] {
  if (!options) return trimUndefinedTail(args);
  return [...args, options];
}

function trimUndefinedTail(args: string[]): string[] {
  const next = [...args];
  while (next[next.length - 1] === "undefined") next.pop();
  return next;
}

function optionsArg(annotations: NodeAnnotations): string | undefined {
  const { toolBinding, ...rest } = annotations;
  const cleanAnnotations = clean(rest) as Omit<NodeAnnotations, "toolBinding">;
  const options: BuilderOptions = {};
  if (Object.keys(cleanAnnotations).length > 0) options.annotations = cleanAnnotations;
  if (toolBinding) options.toolBinding = toolBinding;
  return Object.keys(options).length > 0 ? jsonArg(options) : undefined;
}

function parseMetadata(source: string): SkillGraphMetadata {
  const match = source.match(/createSkill\((\{[^\n]*\})\)/);
  if (!match) throw new SkillCodeError("Expected createSkill({...}) in source.");
  return parseJson(match[1], "skill metadata") as SkillGraphMetadata;
}

function parseNode(method: string, args: unknown[]): SkillGraph["nodes"][number] {
  const id = stringArg(args, 0, "node id");
  const label = stringArg(args, 1, "node label");
  const options = parseOptions(args[args.length - 1]);
  let kind: NodeKind;
  switch (method) {
    case "action":
      kind = { type: "action", instruction: stringArg(args, 2, "instruction") };
      break;
    case "condition":
      kind = { type: "condition", expression: stringArg(args, 2, "expression") };
      break;
    case "branch":
      kind = {
        type: "branch",
        arms: objectArg(args, 2, "branch arms") as Extract<NodeKind, { type: "branch" }>["arms"],
      };
      break;
    case "approval":
      kind = {
        type: "await-approval",
        approverRole: stringArg(args, 2, "approver role"),
        timeoutSeconds: optionalNumberArg(args, 3),
      };
      break;
    case "fetch":
      kind = {
        type: "fetch",
        target: stringArg(args, 2, "fetch target"),
        method: optionalStringArg(args, 3),
      };
      break;
    case "transform":
      kind = { type: "transform", template: stringArg(args, 2, "template") };
      break;
    case "subSkill":
      kind = {
        type: "sub-skill-call",
        target: objectArg(args, 2, "sub-skill target") as Extract<
          NodeKind,
          { type: "sub-skill-call" }
        >["target"],
      };
      break;
    case "loop":
      kind = {
        type: "loop",
        iterVar: stringArg(args, 2, "loop iter var"),
        body: objectArg(args, 3, "loop body") as string[],
      };
      break;
    case "tryCatch":
      kind = {
        type: "try-catch",
        tryBody: objectArg(args, 2, "try body") as string[],
        catchBody: objectArg(args, 3, "catch body") as string[],
      };
      break;
    default:
      throw new SkillCodeError(`Unsupported node method: ${method}`);
  }
  return { id, label, kind, annotations: optionsToAnnotations(options) };
}

function parseEdge(args: unknown[]): SkillGraph["edges"][number] {
  return {
    id: stringArg(args, 0, "edge id"),
    from: stringArg(args, 1, "edge from"),
    to: stringArg(args, 2, "edge to"),
    cond: objectArg(args, 3, "edge condition") as EdgeCond,
  };
}

function parseVariable(args: unknown[]): SkillGraph["variables"][number] {
  return {
    name: stringArg(args, 0, "variable name"),
    varType: stringArg(args, 1, "variable type"),
    scope: (optionalStringArg(args, 2) ?? "graph") as "graph" | "node",
  };
}

function parseOptions(value: unknown): BuilderOptions | undefined {
  if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
  const obj = value as BuilderOptions;
  if (!("annotations" in obj) && !("toolBinding" in obj)) return undefined;
  return obj;
}

function optionsToAnnotations(options?: BuilderOptions): NodeAnnotations {
  return { ...(options?.annotations ?? {}), ...(options?.toolBinding ? { toolBinding: options.toolBinding } : {}) };
}

function validateGraph(graph: SkillGraph): void {
  const ids = new Set(graph.nodes.map((node) => node.id));
  if (!graph.entry || !ids.has(graph.entry)) throw new SkillCodeError("Entry node is missing.");
  for (const edge of graph.edges) {
    if (!ids.has(edge.from) || !ids.has(edge.to)) {
      throw new SkillCodeError(`Edge ${edge.id} references a missing node.`);
    }
  }
}

function parseArgs(raw: string): unknown[] {
  const normalized = `[${raw}]`.replace(/([,[\s]\s*)undefined(?=\s*(,|\]))/g, "$1null");
  return parseJson(normalized, "SDK call arguments") as unknown[];
}

function parseJson(raw: string, label: string): unknown {
  try {
    return JSON.parse(raw);
  } catch {
    throw new SkillCodeError(`Invalid ${label}; only generated SDK code is supported.`);
  }
}

function stringArg(args: unknown[], index: number, label: string): string {
  const value = args[index];
  if (typeof value !== "string") throw new SkillCodeError(`Expected ${label}.`);
  return value;
}

function optionalStringArg(args: unknown[], index: number): string | undefined {
  const value = args[index];
  if (value === undefined || value === null) return undefined;
  if (typeof value !== "string") throw new SkillCodeError("Expected optional string argument.");
  return value;
}

function optionalNumberArg(args: unknown[], index: number): number | undefined {
  const value = args[index];
  if (value === undefined || value === null) return undefined;
  if (typeof value !== "number") throw new SkillCodeError("Expected optional number argument.");
  return value;
}

function objectArg(args: unknown[], index: number, label: string): object {
  const value = args[index];
  if (!value || typeof value !== "object") throw new SkillCodeError(`Expected ${label}.`);
  return value;
}

function optionalArg(value: unknown): string {
  return value === undefined ? "undefined" : jsonArg(value);
}

function jsonArg(value: unknown): string {
  return JSON.stringify(clean(value));
}

function clean(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(clean);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.entries(value)
      .filter(([, v]) => v !== undefined)
      .map(([k, v]) => [k, clean(v)]),
  );
}

function stableStringify(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(",")}]`;
  if (!value || typeof value !== "object") return JSON.stringify(value);
  return `{${Object.entries(value)
    .filter(([, v]) => v !== undefined)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([k, v]) => `${JSON.stringify(k)}:${stableStringify(v)}`)
    .join(",")}}`;
}
