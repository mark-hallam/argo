import { createElement } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ProvenanceBlock } from "@/components/skycot/audit/ProvenanceBlock";
import type { AuditDetail } from "@/lib/types/audit-detail";

const CHAIN_ANCHOR =
  "d37efe0d8c4a0b5f9a1c2d3e4f5061728394a5b6c7d8e9f00112233445566778";

const writeText = vi.fn(async (_text: string) => undefined);

function installClipboardSpy() {
  Object.defineProperty(globalThis.navigator, "clipboard", {
    configurable: true,
    writable: true,
    value: { writeText },
  });
}

function detailWithProvenance(
  provenance: Partial<AuditDetail["provenance"]> = {},
): AuditDetail {
  return {
    id: "act_priya_001",
    timestamp: "2026-05-11T14:32:11.000Z",
    worker: { id: "worker_priya", name: "Priya" },
    actionType: "candidate_screened",
    verb: "screened candidate",
    status: "succeeded",
    inputs: {
      triggerKind: "message",
      triggerSummary: "Application received.",
    },
    decisionChain: [],
    provenance: {
      model: { id: "claude-opus-4-7", version: "2026-04-30" },
      promptTemplate: { id: "recruiter.screen", hash: "8a3b9f1c2d4e5f6a" },
      skill: { id: "recruiter-screen", version: "4.2.0" },
      memorySnapshot: { id: "mem_4f3a9c12" },
      policy: { id: "recruiter-screen-policy", version: "2.0", hash: "c91e2b4f5a6d7c8e" },
      ...provenance,
    },
    output: { kind: "text", body: "Done." },
  };
}

function renderProvenance(detail: AuditDetail) {
  render(
    createElement(ProvenanceBlock, {
      data: detail.provenance,
      detail,
    }),
  );
}

beforeEach(() => {
  writeText.mockClear();
  installClipboardSpy();
});

describe("ProvenanceBlock chain anchor", () => {
  it("renders the gateway hash_chain_anchor row with hover and copy", async () => {
    const detail = detailWithProvenance({ hash_chain_anchor: CHAIN_ANCHOR });
    renderProvenance(detail);

    expect(screen.getByTestId("audit-provenance-row-chain-anchor")).toBeInTheDocument();
    expect(screen.getByText("Chain anchor")).toBeInTheDocument();
    expect(screen.getByTestId("audit-provenance-hash-chain-anchor")).toHaveTextContent(
      "0xd37efe0d8c4a…",
    );
    expect(screen.getByTestId("audit-provenance-hash-chain-anchor")).toHaveAttribute(
      "title",
      `${CHAIN_ANCHOR} — Click to copy`,
    );

    fireEvent.click(screen.getByTestId("audit-provenance-copy-chain-anchor"));

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith(CHAIN_ANCHOR);
    });
  });

  it("omits the chain-anchor row when the gateway field is absent", () => {
    renderProvenance(detailWithProvenance());

    expect(screen.queryByTestId("audit-provenance-row-chain-anchor")).toBeNull();
  });
});
