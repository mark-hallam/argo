import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { VersionHistorySidebar } from "./VersionHistorySidebar";
import type { KnowledgeRevision } from "@/lib/api/workplace-knowledge";

function rev(over: Partial<KnowledgeRevision>): KnowledgeRevision {
  return {
    id: over.id ?? "rev_default",
    doc_id: "doc_brand",
    tenant_id: "demo",
    version: over.version ?? 1,
    body_markdown: "...",
    diff_from_prior: over.diff_from_prior ?? null,
    edited_at_micros: over.edited_at_micros ?? Date.now() * 1000,
    edited_by: over.edited_by ?? "sam@acmeco.com",
    edit_reason: over.edit_reason ?? null,
  };
}

describe("VersionHistorySidebar", () => {
  it("renders a placeholder when there are no revisions yet", () => {
    render(<VersionHistorySidebar slug="brand-voice" revisions={[]} />);
    expect(screen.getByText(/No revisions yet/)).toBeInTheDocument();
  });

  it("links each row to the diff page with from/to params", () => {
    const revisions = [
      rev({ id: "rev_v3", version: 3, diff_from_prior: "- old\n+ new" }),
      rev({ id: "rev_v2", version: 2, diff_from_prior: "- prior\n+ updated" }),
      rev({ id: "rev_v1", version: 1, diff_from_prior: null }),
    ];
    render(<VersionHistorySidebar slug="brand-voice" revisions={revisions} />);

    expect(screen.getByTestId("knowledge-version-3")).toHaveAttribute(
      "href",
      "/settings/knowledge/brand-voice/diff?from=2&to=3",
    );
    expect(screen.getByTestId("knowledge-version-2")).toHaveAttribute(
      "href",
      "/settings/knowledge/brand-voice/diff?from=1&to=2",
    );
    expect(screen.getByTestId("knowledge-version-1")).toHaveAttribute(
      "href",
      "/settings/knowledge/brand-voice/diff?to=1",
    );
  });

  it("renders revision hash + edited-by + edit reason", () => {
    const revisions = [
      rev({
        id: "rev_v1_long_hash_for_truncation",
        version: 1,
        edited_by: "olivia@acmeco.com",
        edit_reason: "Tighten line on tone",
      }),
    ];
    render(<VersionHistorySidebar slug="brand-voice" revisions={revisions} />);
    expect(screen.getByText(/olivia@acmeco\.com/)).toBeInTheDocument();
    expect(screen.getByText(/Tighten line on tone/)).toBeInTheDocument();
    const hash = screen.getByTestId(
      "knowledge-version-hash-rev_v1_long_hash_for_truncation",
    );
    expect(hash).toBeInTheDocument();
    expect(hash).toHaveAttribute("data-hash", "rev_v1_long_hash_for_truncation");
  });
});
