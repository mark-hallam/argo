import { afterEach, describe, expect, it, vi } from "vitest";

const apiPostMock = vi.fn();
vi.mock("./client", () => ({
  apiPost: (path: string, body?: unknown) => apiPostMock(path, body),
}));

import {
  ingestOnboardingDocuments,
  readFileForIngest,
  wikiUrlDoc,
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

describe("wikiUrlDoc", () => {
  it("builds a trimmed wiki_url document", () => {
    const doc = wikiUrlDoc("  https://wiki.example.com/refunds  ");
    expect(doc).toEqual({
      filename: "https://wiki.example.com/refunds",
      format: "wiki_url",
      content: "https://wiki.example.com/refunds",
    });
  });
});

describe("readFileForIngest", () => {
  it("reads a text file as text", async () => {
    const file = new File(["the escalation matrix routes billing disputes"], "sop.md", {
      type: "text/markdown",
    });
    const doc = await readFileForIngest(file);
    expect(doc.format).toBe("text");
    expect(doc.content).toContain("escalation matrix");
  });

  it("reads an image file as image_base64 with the operator caption", async () => {
    const file = new File([new Uint8Array([1, 2, 3, 4])], "board.png", {
      type: "image/png",
    });
    const doc = await readFileForIngest(file, "  escalation board screenshot  ");
    expect(doc.format).toBe("image_base64");
    expect(doc.caption).toBe("escalation board screenshot");
    expect(doc.content.length).toBeGreaterThan(0);
  });
});
