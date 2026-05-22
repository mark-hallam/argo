import { describe, expect, it } from "vitest";

import { decodeStepsFromToml, encodeStepsToToml } from "../step-encoder";
import type { WorkflowStep } from "../types";

const httpStep: WorkflowStep = {
  id: "step-http",
  kind: "http_request",
  name: "Call Stripe",
  description: "Fetch a customer record from Stripe.",
  httpRequest: {
    method: "POST",
    url: "https://api.stripe.com/v1/customers",
    headers: [
      { key: "X-Idempotency-Key", value: "{{run.id}}" },
      { key: "X-Trace", value: "skycot" },
    ],
    body: "{\n  \"email\": \"{{input.email}}\"\n}",
    auth: "bearer",
    authToken: "sk_test_demo",
    authUsername: "",
    authPassword: "",
    authHeaderName: "",
    authHeaderValue: "",
    timeoutSeconds: 12,
  },
};

const codeStep: WorkflowStep = {
  id: "step-code",
  kind: "code",
  name: "Compute total",
  description: "Sum the line items.",
  code: {
    language: "python",
    code: "total = sum(input['line_items'])\nreturn { 'total': total }\n",
  },
};

describe("step-encoder http_request + code round-trip", () => {
  it("round-trips an http_request step with bearer auth, headers, body, timeout", () => {
    const toml = encodeStepsToToml([httpStep], { workflowName: "http_demo" });
    const decoded = decodeStepsFromToml(toml);
    expect(decoded.steps).toEqual([httpStep]);
  });

  it("round-trips an http_request step with basic auth", () => {
    const basic: WorkflowStep = {
      ...httpStep,
      httpRequest: {
        ...httpStep.httpRequest!,
        auth: "basic",
        authToken: "",
        authUsername: "demo",
        authPassword: "secret",
      },
    };
    const toml = encodeStepsToToml([basic], { workflowName: "basic_auth" });
    expect(decodeStepsFromToml(toml).steps).toEqual([basic]);
  });

  it("round-trips an http_request step with custom_header auth", () => {
    const custom: WorkflowStep = {
      ...httpStep,
      httpRequest: {
        ...httpStep.httpRequest!,
        auth: "custom_header",
        authToken: "",
        authHeaderName: "X-API-Key",
        authHeaderValue: "demo",
      },
    };
    const toml = encodeStepsToToml([custom], { workflowName: "custom_auth" });
    expect(decodeStepsFromToml(toml).steps).toEqual([custom]);
  });

  it("round-trips a code step with the python language", () => {
    const toml = encodeStepsToToml([codeStep], { workflowName: "code_demo" });
    expect(decodeStepsFromToml(toml).steps).toEqual([codeStep]);
  });

  it("emits http_method and code_language tokens in the TOML payload", () => {
    const toml = encodeStepsToToml([httpStep, codeStep], { workflowName: "mixed" });
    expect(toml).toContain("http_method = \"POST\"");
    expect(toml).toContain("http_url = \"https://api.stripe.com/v1/customers\"");
    expect(toml).toContain("http_auth = \"bearer\"");
    expect(toml).toContain("http_timeout_seconds = 12");
    expect(toml).toContain("code_language = \"python\"");
  });

  it("preserves embedded quotes in the code body through round-trip", () => {
    const escaped: WorkflowStep = {
      id: "step-js",
      kind: "code",
      name: "Quote handling",
      description: "",
      code: {
        language: "javascript",
        code: "return { message: \"hi \\\"there\\\"\\nline two\" };",
      },
    };
    const toml = encodeStepsToToml([escaped], { workflowName: "quotes" });
    expect(decodeStepsFromToml(toml).steps).toEqual([escaped]);
  });

  it("decodes an http_request kind without config keys as a bare step (no httpRequest field)", () => {
    const toml = [
      "[workflow]",
      "name = \"bare\"",
      "",
      "[[step]]",
      "id = \"step-1\"",
      "name = \"Bare HTTP\"",
      "kind = \"http_request\"",
      "description = \"\"",
      "",
    ].join("\n");
    const decoded = decodeStepsFromToml(toml);
    expect(decoded.steps).toEqual([
      { id: "step-1", kind: "http_request", name: "Bare HTTP", description: "" },
    ]);
  });

  it("falls back to safe defaults when http_method is unknown", () => {
    const toml = [
      "[workflow]",
      "name = \"weird\"",
      "",
      "[[step]]",
      "id = \"step-1\"",
      "name = \"Weird method\"",
      "kind = \"http_request\"",
      "http_method = \"TEAPOT\"",
      "http_url = \"https://example.test\"",
      "http_auth = \"alien\"",
      "http_timeout_seconds = 0",
      "",
    ].join("\n");
    const decoded = decodeStepsFromToml(toml);
    const cfg = decoded.steps[0].httpRequest!;
    expect(cfg.method).toBe("GET");
    expect(cfg.auth).toBe("none");
    expect(cfg.timeoutSeconds).toBe(30);
  });
});
