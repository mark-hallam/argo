import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CodeBlock } from "../CodeBlock";

describe("CodeBlock", () => {
  it("renders the code body, language label, and a copy button", () => {
    render(<CodeBlock code={"echo hi"} language="bash" />);
    expect(screen.getByText("bash")).toBeInTheDocument();
    expect(screen.getByText("echo hi")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /copy/i })).toBeInTheDocument();
  });

  it("hides the copy button when showCopy=false and language is omitted", () => {
    render(<CodeBlock code="x" showCopy={false} />);
    expect(screen.queryByRole("button")).toBeNull();
  });
});
