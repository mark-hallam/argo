import { describe, expect, it } from "vitest";

import {
  decodeStepsFromToml,
  encodeStepsToToml,
} from "@/app/(app)/workflows/[verticalId]/[name]/build/step-encoder";
import {
  SAMPLE_WORKFLOW_SPECS,
  SAMPLE_WORKFLOW_SUMMARIES,
} from "@/lib/sample-workflows";

/**
 * Shape + catalog-coverage tests for the sample-workflows seed.
 *
 * `web/lib/sample-workflows.ts` is what new users see in the
 * "Try a workflow" empty state when the gateway has no live data.
 * These tests guarantee that every sample is renderable, that the
 * catalog still surfaces an HTTP-request example and a Code example
 * (added in PR #2011's follow-up brief), and that those two samples
 * round-trip cleanly through the canonical builder encoder.
 */
describe("sample-workflows catalog", () => {
  it("every summary has a matching spec keyed `<vertical>/<name>`", () => {
    for (const summary of SAMPLE_WORKFLOW_SUMMARIES) {
      const key = `${summary.vertical}/${summary.name}`;
      expect(SAMPLE_WORKFLOW_SPECS[key], `missing spec for ${key}`).toBeDefined();
    }
  });

  it("includes at least one workflow that uses an http_request step", () => {
    const found = Object.values(SAMPLE_WORKFLOW_SPECS).some((spec) =>
      spec.steps.some((step) => step.kind === "http_request" && step.httpRequest),
    );
    expect(found).toBe(true);
  });

  it("includes at least one workflow that uses a code step", () => {
    const found = Object.values(SAMPLE_WORKFLOW_SPECS).some((spec) =>
      spec.steps.some((step) => step.kind === "code" && step.code),
    );
    expect(found).toBe(true);
  });

  it("HTTP sample round-trips through the canonical builder encoder", () => {
    const spec = SAMPLE_WORKFLOW_SPECS["sales/morning_pipeline_digest"];
    expect(spec).toBeDefined();
    const httpStep = spec.steps.find((step) => step.kind === "http_request");
    expect(httpStep).toBeDefined();
    expect(httpStep?.httpRequest?.url).toContain("hubspot");

    const toml = encodeStepsToToml(spec.steps, { workflowName: spec.name });
    const decoded = decodeStepsFromToml(toml);
    const decodedHttp = decoded.steps.find((step) => step.kind === "http_request");
    expect(decodedHttp).toBeDefined();
    expect(decodedHttp?.httpRequest).toEqual(httpStep?.httpRequest);
  });

  it("Code sample round-trips through the canonical builder encoder", () => {
    const spec = SAMPLE_WORKFLOW_SPECS["finance/clean_customer_list"];
    expect(spec).toBeDefined();
    const codeStep = spec.steps.find((step) => step.kind === "code");
    expect(codeStep).toBeDefined();
    expect(codeStep?.code?.language).toBe("javascript");

    const toml = encodeStepsToToml(spec.steps, { workflowName: spec.name });
    const decoded = decodeStepsFromToml(toml);
    const decodedCode = decoded.steps.find((step) => step.kind === "code");
    expect(decodedCode).toBeDefined();
    expect(decodedCode?.code).toEqual(codeStep?.code);
  });

  it("every spec encodes to TOML without throwing", () => {
    for (const [key, spec] of Object.entries(SAMPLE_WORKFLOW_SPECS)) {
      expect(
        () => encodeStepsToToml(spec.steps, { workflowName: spec.name }),
        `encode failed for ${key}`,
      ).not.toThrow();
    }
  });

  it("sample names use plain underscore_case (no internal jargon)", () => {
    const ok = /^[a-z][a-z0-9_]*$/;
    for (const summary of SAMPLE_WORKFLOW_SUMMARIES) {
      expect(summary.name, `bad name: ${summary.name}`).toMatch(ok);
    }
  });
});
