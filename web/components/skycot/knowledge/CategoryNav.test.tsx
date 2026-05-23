import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { CategoryNav } from "./CategoryNav";
import type { KnowledgeDoc } from "@/lib/api/workplace-knowledge";

function docs(): KnowledgeDoc[] {
  const base = {
    tenant_id: "demo",
    body_markdown: "",
    version: 1,
    parent_revision_id: null,
    created_at_micros: 1,
    created_by: "x",
    updated_at_micros: 1,
    updated_by: "x",
  };
  return [
    { id: "1", slug: "a", title: "A", category: "brand", ...base },
    { id: "2", slug: "b", title: "B", category: "brand", ...base },
    { id: "3", slug: "c", title: "C", category: "compliance", ...base },
  ] as KnowledgeDoc[];
}

describe("CategoryNav", () => {
  it("renders an All entry plus the five categories with counts", () => {
    render(<CategoryNav docs={docs()} active="all" onSelect={() => {}} />);
    expect(screen.getByTestId("knowledge-nav-all")).toHaveTextContent("All3");
    expect(screen.getByTestId("knowledge-nav-brand")).toHaveTextContent("Brand2");
    expect(screen.getByTestId("knowledge-nav-business-rules")).toHaveTextContent("Business rules0");
    expect(screen.getByTestId("knowledge-nav-compliance")).toHaveTextContent("Compliance1");
    expect(screen.getByTestId("knowledge-nav-internal")).toHaveTextContent("Internal0");
    expect(screen.getByTestId("knowledge-nav-faq")).toHaveTextContent("FAQ0");
  });

  it("highlights the active category and invokes onSelect on click", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<CategoryNav docs={docs()} active="brand" onSelect={onSelect} />);

    expect(screen.getByTestId("knowledge-nav-brand").className).toMatch(/knowledge-nav-item-active/);

    await user.click(screen.getByTestId("knowledge-nav-compliance"));
    expect(onSelect).toHaveBeenCalledWith("compliance");

    await user.click(screen.getByTestId("knowledge-nav-all"));
    expect(onSelect).toHaveBeenCalledWith("all");
  });
});
