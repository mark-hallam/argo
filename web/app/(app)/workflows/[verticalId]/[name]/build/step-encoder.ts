/**
 * Step ↔ TOML encoder for the workflow builder Save flow.
 *
 * The gateway's `POST /api/v1/verticals/{vid}/workflows/{name}` endpoint
 * accepts a raw `workflow.toml` body. This module encodes the
 * builder's in-memory `WorkflowStep[]` to that wire shape and
 * round-trips it back (decoder) so the canvas state survives a Save
 * → reload through the gateway.
 *
 * Phase 1 ships the four fields every visual step carries: `id`,
 * `name`, `kind`, `description`. Advanced step shape (approval gate,
 * loop config, decision branches, sub-workflow refs) lands in a
 * follow-up brief; encoder/decoder are forward-compatible — unknown
 * keys are silently passed through on encode (none today) and
 * ignored on decode.
 */
import type {
  CodeConfig,
  CodeLanguage,
  HttpAuthMode,
  HttpHeader,
  HttpMethod,
  HttpRequestConfig,
  StepKind,
  WorkflowStep,
} from "./types";
import { HTTP_METHODS } from "./types";

const STEP_KINDS: ReadonlySet<StepKind> = new Set<StepKind>([
  "trigger",
  "skill_invoke",
  "approval",
  "tool_call",
  "notify",
  "decision",
  "data",
  "loop",
  "wait",
  "sub_workflow",
  "http_request",
  "code",
]);

const HTTP_METHOD_SET: ReadonlySet<HttpMethod> = new Set<HttpMethod>(HTTP_METHODS);
const HTTP_AUTH_MODES: ReadonlySet<HttpAuthMode> = new Set<HttpAuthMode>([
  "none",
  "bearer",
  "basic",
  "custom_header",
]);
const CODE_LANGUAGES: ReadonlySet<CodeLanguage> = new Set<CodeLanguage>([
  "javascript",
  "python",
]);

function escapeTomlString(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n");
}

function tomlString(value: string): string {
  return `"${escapeTomlString(value)}"`;
}

export interface EncodeOptions {
  workflowName: string;
  workflowDescription?: string;
}

export function encodeStepsToToml(
  steps: WorkflowStep[],
  opts: EncodeOptions,
): string {
  const lines: string[] = ["[workflow]"];
  lines.push(`name = ${tomlString(opts.workflowName)}`);
  if (opts.workflowDescription) {
    lines.push(`description = ${tomlString(opts.workflowDescription)}`);
  }
  for (const step of steps) {
    lines.push("", "[[step]]");
    lines.push(`id = ${tomlString(step.id)}`);
    lines.push(`name = ${tomlString(step.name)}`);
    lines.push(`kind = ${tomlString(step.kind)}`);
    if (step.description) {
      lines.push(`description = ${tomlString(step.description)}`);
    }
    if (step.kind === "http_request" && step.httpRequest) {
      appendHttpRequestLines(lines, step.httpRequest);
    }
    if (step.kind === "code" && step.code) {
      appendCodeLines(lines, step.code);
    }
  }
  lines.push("");
  return lines.join("\n");
}

function appendHttpRequestLines(lines: string[], cfg: HttpRequestConfig): void {
  lines.push(`http_method = ${tomlString(cfg.method)}`);
  lines.push(`http_url = ${tomlString(cfg.url)}`);
  lines.push(`http_auth = ${tomlString(cfg.auth)}`);
  lines.push(`http_timeout_seconds = ${Math.max(1, Math.floor(cfg.timeoutSeconds || 30))}`);
  if (cfg.body) lines.push(`http_body = ${tomlString(cfg.body)}`);
  if (cfg.headers.length > 0) {
    const encoded = cfg.headers
      .filter((h) => h.key.trim())
      .map((h) => `${h.key.trim()}: ${h.value}`)
      .join("\n");
    if (encoded) lines.push(`http_headers = ${tomlString(encoded)}`);
  }
  if (cfg.auth === "bearer" && cfg.authToken) {
    lines.push(`http_auth_token = ${tomlString(cfg.authToken)}`);
  }
  if (cfg.auth === "basic") {
    if (cfg.authUsername) lines.push(`http_auth_username = ${tomlString(cfg.authUsername)}`);
    if (cfg.authPassword) lines.push(`http_auth_password = ${tomlString(cfg.authPassword)}`);
  }
  if (cfg.auth === "custom_header") {
    if (cfg.authHeaderName) lines.push(`http_auth_header_name = ${tomlString(cfg.authHeaderName)}`);
    if (cfg.authHeaderValue) lines.push(`http_auth_header_value = ${tomlString(cfg.authHeaderValue)}`);
  }
}

function appendCodeLines(lines: string[], cfg: CodeConfig): void {
  lines.push(`code_language = ${tomlString(cfg.language)}`);
  lines.push(`code_body = ${tomlString(cfg.code)}`);
}

export interface DecodedWorkflow {
  workflowName: string | null;
  workflowDescription: string | null;
  steps: WorkflowStep[];
}

interface PendingStep {
  id?: string;
  name?: string;
  kind?: string;
  description?: string;
  http_method?: string;
  http_url?: string;
  http_auth?: string;
  http_timeout_seconds?: string;
  http_body?: string;
  http_headers?: string;
  http_auth_token?: string;
  http_auth_username?: string;
  http_auth_password?: string;
  http_auth_header_name?: string;
  http_auth_header_value?: string;
  code_language?: string;
  code_body?: string;
}

function unescapeTomlString(s: string): string {
  let out = "";
  for (let i = 0; i < s.length; i++) {
    if (s[i] === "\\" && i + 1 < s.length) {
      const next = s[i + 1];
      if (next === "\\") { out += "\\"; i += 1; continue; }
      if (next === "\"") { out += "\""; i += 1; continue; }
      if (next === "n") { out += "\n"; i += 1; continue; }
      if (next === "t") { out += "\t"; i += 1; continue; }
    }
    out += s[i];
  }
  return out;
}

function stripQuoted(value: string): string {
  if (value.length >= 2 && value.startsWith("\"") && value.endsWith("\"")) {
    return unescapeTomlString(value.slice(1, -1));
  }
  return value;
}

function commit(pending: PendingStep, idx: number): WorkflowStep | null {
  const name = pending.name?.trim();
  if (!name) return null;
  const id = pending.id?.trim() || `step-${idx + 1}`;
  const rawKind = pending.kind?.trim() ?? "";
  const kind: StepKind = STEP_KINDS.has(rawKind as StepKind)
    ? (rawKind as StepKind)
    : "tool_call";
  const step: WorkflowStep = { id, kind, name, description: pending.description ?? "" };
  if (kind === "http_request" && hasHttpRequestKeys(pending)) {
    step.httpRequest = decodeHttpRequest(pending);
  }
  if (kind === "code" && hasCodeKeys(pending)) {
    step.code = decodeCode(pending);
  }
  return step;
}

function hasHttpRequestKeys(pending: PendingStep): boolean {
  return (
    pending.http_method !== undefined
    || pending.http_url !== undefined
    || pending.http_auth !== undefined
    || pending.http_timeout_seconds !== undefined
    || pending.http_body !== undefined
    || pending.http_headers !== undefined
  );
}

function hasCodeKeys(pending: PendingStep): boolean {
  return pending.code_language !== undefined || pending.code_body !== undefined;
}

function decodeHttpRequest(pending: PendingStep): HttpRequestConfig {
  const method = (pending.http_method ?? "GET").toUpperCase() as HttpMethod;
  const auth = (pending.http_auth ?? "none") as HttpAuthMode;
  const timeoutRaw = Number.parseInt(pending.http_timeout_seconds ?? "30", 10);
  return {
    method: HTTP_METHOD_SET.has(method) ? method : "GET",
    url: pending.http_url ?? "",
    headers: parseHeaderBlock(pending.http_headers),
    body: pending.http_body ?? "",
    auth: HTTP_AUTH_MODES.has(auth) ? auth : "none",
    authToken: pending.http_auth_token ?? "",
    authUsername: pending.http_auth_username ?? "",
    authPassword: pending.http_auth_password ?? "",
    authHeaderName: pending.http_auth_header_name ?? "",
    authHeaderValue: pending.http_auth_header_value ?? "",
    timeoutSeconds: Number.isFinite(timeoutRaw) && timeoutRaw > 0 ? timeoutRaw : 30,
  };
}

function decodeCode(pending: PendingStep): CodeConfig {
  const language = (pending.code_language ?? "javascript") as CodeLanguage;
  return {
    language: CODE_LANGUAGES.has(language) ? language : "javascript",
    code: pending.code_body ?? "",
  };
}

function parseHeaderBlock(raw: string | undefined): HttpHeader[] {
  if (!raw) return [];
  const out: HttpHeader[] = [];
  for (const line of raw.split("\n")) {
    const idx = line.indexOf(":");
    if (idx < 0) continue;
    const key = line.slice(0, idx).trim();
    const value = line.slice(idx + 1).trim();
    if (key) out.push({ key, value });
  }
  return out;
}

/**
 * Decode a TOML document produced by `encodeStepsToToml`. Tolerant of
 * comments, blank lines, and unknown sections so encoder evolution
 * never breaks a half-written document.
 */
export function decodeStepsFromToml(toml: string): DecodedWorkflow {
  let workflowName: string | null = null;
  let workflowDescription: string | null = null;
  const steps: WorkflowStep[] = [];
  let section: "none" | "workflow" | "step" = "none";
  let pending: PendingStep | null = null;
  for (const rawLine of toml.split(/\r?\n/)) {
    const line = rawLine.replace(/\s+#.*$/, "").replace(/^\s*#.*$/, "").trim();
    if (!line) continue;
    if (line === "[workflow]") {
      if (pending) {
        const s = commit(pending, steps.length);
        if (s) steps.push(s);
        pending = null;
      }
      section = "workflow";
      continue;
    }
    if (line === "[[step]]") {
      if (pending) {
        const s = commit(pending, steps.length);
        if (s) steps.push(s);
      }
      pending = {};
      section = "step";
      continue;
    }
    if (line.startsWith("[")) {
      if (pending) {
        const s = commit(pending, steps.length);
        if (s) steps.push(s);
        pending = null;
      }
      section = "none";
      continue;
    }
    const eq = line.indexOf("=");
    if (eq < 0) continue;
    const key = line.slice(0, eq).trim();
    const value = stripQuoted(line.slice(eq + 1).trim());
    if (section === "workflow") {
      if (key === "name") workflowName = value;
      else if (key === "description") workflowDescription = value;
    } else if (section === "step" && pending) {
      if (key === "id") pending.id = value;
      else if (key === "name") pending.name = value;
      else if (key === "kind") pending.kind = value;
      else if (key === "description") pending.description = value;
      else if (key === "http_method") pending.http_method = value;
      else if (key === "http_url") pending.http_url = value;
      else if (key === "http_auth") pending.http_auth = value;
      else if (key === "http_timeout_seconds") pending.http_timeout_seconds = value;
      else if (key === "http_body") pending.http_body = value;
      else if (key === "http_headers") pending.http_headers = value;
      else if (key === "http_auth_token") pending.http_auth_token = value;
      else if (key === "http_auth_username") pending.http_auth_username = value;
      else if (key === "http_auth_password") pending.http_auth_password = value;
      else if (key === "http_auth_header_name") pending.http_auth_header_name = value;
      else if (key === "http_auth_header_value") pending.http_auth_header_value = value;
      else if (key === "code_language") pending.code_language = value;
      else if (key === "code_body") pending.code_body = value;
    }
  }
  if (pending) {
    const s = commit(pending, steps.length);
    if (s) steps.push(s);
  }
  return { workflowName, workflowDescription, steps };
}
