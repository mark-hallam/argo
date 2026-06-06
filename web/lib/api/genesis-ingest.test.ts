import { afterEach, describe, expect, it, vi } from "vitest";

const apiPostMock = vi.fn();
vi.mock("./client", () => ({
  apiPost: (path: string, body?: unknown) => apiPostMock(path, body),
}));

import {
  ingestOnboardingDocuments,
  INGEST_PHASES,
  ONBOARDING_MAX_FILES,
  type IngestResponse,
} from "./genesis-ingest";

afterEach(() => {
  apiPostMock.mockReset();
});

describe("ingestOnboardingDocuments", () => {
  it("posts the vertical id + documents to the ingest endpoint", async () => {
    const resp: IngestResponse = {
      results: [
        { filename: "policy.md", status: "ingested", chunks: 3, phases: [...INGEST_PHASES] },
      ],
      ingested: 1,
      rejected: 0,
      labels: ["policy.md"],
      namespace: "onboarding_documents:acme",
    };
    apiPostMock.mockResolvedValueOnce(resp);

    const out = await ingestOnboardingDocuments("genesis-sam-ab12", [
      { filename: "policy.md", format: "text", content: "some body" },
    ]);

    expect(apiPostMock).toHaveBeenCalledWith("/api/v1/genesis/ingest-document", {
      vertical_id: "genesis-sam-ab12",
      documents: [{ filename: "policy.md", format: "text", content: "some body" }],
    });
    expect(out.namespace).toBe("onboarding_documents:acme");
    expect(out.labels).toEqual(["policy.md"]);
  });

  it("exposes the canonical 4-phase order + file cap", () => {
    expect(INGEST_PHASES).toEqual(["uploaded", "parsed", "chunked", "embedded"]);
    expect(ONBOARDING_MAX_FILES).toBe(50);
  });
});
