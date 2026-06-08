import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

/**
 * Genesis DocumentUploader image-caption tests (#2746 review MEDIUM).
 *
 * The multi-modal backend grades an uploaded screenshot over an operator caption
 * and REJECTS a captionless one (no fake OCR). These assert the SPA now collects
 * that caption before ingesting: a dropped image stays un-ingestable until it has
 * a caption, and on confirm the caption reaches the ingest call.
 *
 * `ingestOnboardingDocuments` is mocked; `readFileForIngest` runs for real so the
 * test exercises the full path image → image_base64 + caption → ingest payload.
 */

const { ingestMock } = vi.hoisted(() => ({ ingestMock: vi.fn() }));
vi.mock("@/lib/api/genesis-ingest", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/genesis-ingest")>(
    "@/lib/api/genesis-ingest",
  );
  return { ...actual, ingestOnboardingDocuments: ingestMock };
});

import { DocumentUploader } from "./DocumentUploader";

beforeEach(() => {
  ingestMock.mockReset();
  ingestMock.mockResolvedValue({
    results: [{ filename: "board.png", status: "ingested", chunks: 2 }],
    ingested: 1,
    rejected: 0,
    labels: ["board.png"],
    namespace: "onboarding_documents:acme",
  });
});

function pickImage() {
  const input = screen.getByTestId("genesis-document-input") as HTMLInputElement;
  const file = new File([new Uint8Array([1, 2, 3, 4])], "board.png", { type: "image/png" });
  fireEvent.change(input, { target: { files: [file] } });
}

describe("DocumentUploader image caption", () => {
  it("stages a dropped image and disables ingest until a caption is typed", async () => {
    render(<DocumentUploader verticalId="genesis-x" />);
    pickImage();
    const add = await screen.findByTestId("genesis-image-caption-add");
    expect(add).toBeDisabled();
    expect(ingestMock).not.toHaveBeenCalled();
  });

  it("ingests an image WITH the typed caption as the gradeable text", async () => {
    render(<DocumentUploader verticalId="genesis-x" />);
    pickImage();

    const caption = await screen.findByTestId("genesis-image-caption-input");
    fireEvent.change(caption, { target: { value: "  escalation board screenshot  " } });

    const add = screen.getByTestId("genesis-image-caption-add");
    expect(add).not.toBeDisabled();
    fireEvent.click(add);

    await waitFor(() => expect(ingestMock).toHaveBeenCalled());
    const [verticalId, docs] = ingestMock.mock.calls[0];
    expect(verticalId).toBe("genesis-x");
    expect(docs).toHaveLength(1);
    expect(docs[0].format).toBe("image_base64");
    expect(docs[0].caption).toBe("escalation board screenshot");
    expect(docs[0].content.length).toBeGreaterThan(0);
  });

  it("does not send the image when the caption is whitespace-only", async () => {
    render(<DocumentUploader verticalId="genesis-x" />);
    pickImage();

    const caption = await screen.findByTestId("genesis-image-caption-input");
    fireEvent.change(caption, { target: { value: "   " } });

    const add = screen.getByTestId("genesis-image-caption-add");
    expect(add).toBeDisabled();
    fireEvent.click(add);
    expect(ingestMock).not.toHaveBeenCalled();
  });
});
