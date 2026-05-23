import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { KnowledgeCard, relativeFromMicros } from "./KnowledgeCard";
import type { KnowledgeDoc } from "@/lib/api/workplace-knowledge";

function sampleDoc(over: Partial<KnowledgeDoc> = {}): KnowledgeDoc {
  return {
    id: "doc_brand",
    tenant_id: "demo",
    slug: "brand-voice",
    title: "Brand voice",
    category: "brand",
    body_markdown: "# Brand voice\n\nbe warm, be clear.",
    version: 3,
    parent_revision_id: null,
    created_at_micros: 1_000_000_000,
    created_by: "sam@acmeco.com",
    updated_at_micros: (Date.now() - 30 * 60_000) * 1000,
    updated_by: "sam@acmeco.com",
    ...over,
  };
}

describe("KnowledgeCard", () => {
  it("renders title, category pill, version chip, updated-by", () => {
    render(<KnowledgeCard doc={sampleDoc()} />);
    expect(screen.getByText("Brand voice")).toBeInTheDocument();
    expect(screen.getByText("Brand")).toBeInTheDocument();
    expect(screen.getByText("v3")).toBeInTheDocument();
    expect(screen.getByTestId("knowledge-card-updated-brand-voice")).toHaveTextContent(
      /sam@acmeco\.com/,
    );
  });

  it("links to the doc detail page using the slug", () => {
    render(<KnowledgeCard doc={sampleDoc({ slug: "weird slug?" })} />);
    const link = screen.getByTestId("knowledge-card-weird slug?");
    expect(link).toHaveAttribute(
      "href",
      "/settings/knowledge/weird%20slug%3F",
    );
  });

  it("formats minutes / hours / days correctly via relativeFromMicros", () => {
    const now = Date.now();
    expect(relativeFromMicros(now * 1000)).toBe("just now");
    expect(relativeFromMicros((now - 5 * 60_000) * 1000)).toBe("5 min ago");
    expect(relativeFromMicros((now - 3 * 60 * 60_000) * 1000)).toBe("3 h ago");
    expect(relativeFromMicros((now - 2 * 24 * 60 * 60_000) * 1000)).toBe("2 d ago");
  });
});
