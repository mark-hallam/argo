import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

/**
 * Genesis WikiImportSources tests
 * (mission-genesis-integration-import-slack-notion-confluence).
 *
 * Asserts the branded source chips re-point the ONE underlying URL input's
 * placeholder + helper copy and focus it, and that "Add link" submits through
 * the parent's `onAddLink` (the existing `format:"wiki_url"` ingest path — NO
 * mock fallback, NO new endpoint). The honest "connector coming soon" boundary
 * note is always present (NO_THEATER, DEC-007).
 */

import { WikiImportSources } from "./WikiImportSources";

beforeEach(() => {
  vi.restoreAllMocks();
});

describe("WikiImportSources", () => {
  it("shows the three branded source chips and the connector boundary note", () => {
    render(<WikiImportSources onAddLink={vi.fn()} />);
    expect(screen.getByTestId("genesis-import-source-notion")).toBeInTheDocument();
    expect(screen.getByTestId("genesis-import-source-confluence")).toBeInTheDocument();
    expect(screen.getByTestId("genesis-import-source-wiki")).toBeInTheDocument();
    expect(screen.getByTestId("genesis-import-connector-note")).toHaveTextContent(
      /workspace connector/i,
    );
  });

  it("clicking a source chip updates the input placeholder + helper and focuses it", () => {
    render(<WikiImportSources onAddLink={vi.fn()} />);
    const input = screen.getByTestId("genesis-url-input") as HTMLInputElement;
    expect(input.placeholder).toBe("…or paste a wiki / docs URL");

    fireEvent.click(screen.getByTestId("genesis-import-source-notion"));

    expect(input.placeholder).toBe("Paste a published / shared Notion page URL");
    expect(screen.getByTestId("genesis-import-helper")).toHaveTextContent(/Notion page URL/i);
    expect(document.activeElement).toBe(input);

    fireEvent.click(screen.getByTestId("genesis-import-source-confluence"));
    expect(input.placeholder).toBe("Paste a Confluence page URL your account can view");
    expect(screen.getByTestId("genesis-import-helper")).toHaveTextContent(/Confluence page URL/i);
  });

  it("Add link calls onAddLink with the typed URL (the wiki_url ingest path)", async () => {
    const onAddLink = vi.fn().mockResolvedValue(true);
    render(<WikiImportSources onAddLink={onAddLink} />);

    const input = screen.getByTestId("genesis-url-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "https://acme.notion.site/handbook" } });

    fireEvent.click(screen.getByTestId("genesis-url-add"));

    await waitFor(() => expect(onAddLink).toHaveBeenCalledWith("https://acme.notion.site/handbook"));
    // Accepted submissions clear the input.
    await waitFor(() => expect(input.value).toBe(""));
  });

  it("keeps the input value when onAddLink rejects (validation failed)", async () => {
    const onAddLink = vi.fn().mockResolvedValue(false);
    render(<WikiImportSources onAddLink={onAddLink} />);

    const input = screen.getByTestId("genesis-url-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "not-a-url" } });
    fireEvent.click(screen.getByTestId("genesis-url-add"));

    await waitFor(() => expect(onAddLink).toHaveBeenCalled());
    expect(input.value).toBe("not-a-url");
  });

  it("disables the chips and Add link button while disabled", () => {
    render(<WikiImportSources disabled onAddLink={vi.fn()} />);
    expect(screen.getByTestId("genesis-import-source-notion")).toBeDisabled();
    expect(screen.getByTestId("genesis-url-input")).toBeDisabled();
    // Add link stays disabled even with text because the panel is disabled.
    fireEvent.change(screen.getByTestId("genesis-url-input"), {
      target: { value: "https://x.test/y" },
    });
    expect(screen.getByTestId("genesis-url-add")).toBeDisabled();
  });
});
