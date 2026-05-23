import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { KnowledgeEditor } from "./KnowledgeEditor";
import { renderMarkdown } from "./render-markdown";

function Harness({
  initial = "# Hello",
  readOnly = false,
  onSave = () => {},
  isSaving = false,
}: {
  initial?: string;
  readOnly?: boolean;
  onSave?: (input: { body: string; editReason: string }) => void;
  isSaving?: boolean;
}) {
  const [body, setBody] = useState(initial);
  return (
    <KnowledgeEditor
      body={body}
      onBodyChange={setBody}
      onSave={onSave}
      isSaving={isSaving}
      readOnly={readOnly}
    />
  );
}

describe("KnowledgeEditor", () => {
  it("starts on the Edit tab and switches to Preview rendering markdown", async () => {
    const user = userEvent.setup();
    render(<Harness initial={"# Brand voice\n\nbe **warm**"} />);

    expect(screen.getByTestId("knowledge-editor-textarea")).toBeInTheDocument();

    await user.click(screen.getByTestId("knowledge-editor-tab-preview"));
    const preview = screen.getByTestId("knowledge-editor-preview");
    expect(preview.innerHTML).toContain("<h1>Brand voice</h1>");
    expect(preview.innerHTML).toContain("<strong>warm</strong>");
  });

  it("updates Preview content when the Edit textarea changes", async () => {
    const user = userEvent.setup();
    render(<Harness initial="" />);
    const textarea = screen.getByTestId("knowledge-editor-textarea");
    await user.type(textarea, "## Hello");
    await user.click(screen.getByTestId("knowledge-editor-tab-preview"));
    expect(screen.getByTestId("knowledge-editor-preview").innerHTML).toContain(
      "<h2>Hello</h2>",
    );
  });

  it("disables the Save button while in flight", () => {
    render(<Harness isSaving={true} />);
    expect(screen.getByTestId("knowledge-editor-save")).toBeDisabled();
    expect(screen.getByTestId("knowledge-editor-save")).toHaveTextContent(/Saving/);
  });

  it("calls onSave with the current body + edit reason", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(<Harness initial="hello" onSave={onSave} />);
    await user.type(screen.getByTestId("knowledge-editor-edit-reason"), "Tighten copy");
    await user.click(screen.getByTestId("knowledge-editor-save"));
    expect(onSave).toHaveBeenCalledWith({ body: "hello", editReason: "Tighten copy" });
  });

  it("read-only mode hides the Save row and switches the tab label", () => {
    render(<Harness readOnly={true} />);
    expect(screen.queryByTestId("knowledge-editor-save")).not.toBeInTheDocument();
    expect(screen.queryByTestId("knowledge-editor-edit-reason")).not.toBeInTheDocument();
    expect(screen.getByTestId("knowledge-editor-tab-edit")).toHaveTextContent("View");
  });

  it("renderMarkdown escapes HTML and emits only safe tags", () => {
    const html = renderMarkdown("# <script>alert('x')</script>");
    expect(html).not.toContain("<script>");
    expect(html).toContain("&lt;script&gt;");
  });
});
