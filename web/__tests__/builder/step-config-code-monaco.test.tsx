import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { CodeConfigEditor } from "@/app/(app)/workflows/[verticalId]/[name]/build/step-config-code";
import type { CodeConfig } from "@/app/(app)/workflows/[verticalId]/[name]/build/types";

// Monaco doesn't run in happy-dom (needs a real DOM with workers +
// canvas). Mock the lazy-loaded wrapper at the module boundary with a
// controlled `<textarea>` that mirrors the wrapper's value / language /
// onChange contract — that's the surface step-config-code.tsx
// integrates against, and it's what we need to assert.
vi.mock("@/components/builder/MonacoCodeEditor", () => ({
  __esModule: true,
  default: ({
    value,
    language,
    onChange,
  }: {
    value: string;
    language: "javascript" | "python";
    onChange: (next: string) => void;
  }) => (
    <textarea
      data-testid="monaco-fake"
      data-language={language}
      value={value}
      onChange={(e) => onChange(e.target.value)}
    />
  ),
}));

function Harness({ initial }: { initial: CodeConfig }) {
  const [cfg, setCfg] = useState<CodeConfig>(initial);
  return <CodeConfigEditor config={cfg} onChange={setCfg} />;
}

describe("CodeConfigEditor (Monaco swap-in)", () => {
  it("mounts the Monaco editor with JavaScript language for a JS step", async () => {
    render(<Harness initial={{ language: "javascript", code: "return 1;" }} />);
    const editor = await screen.findByTestId("monaco-fake");
    expect(editor).toBeInTheDocument();
    expect(editor.getAttribute("data-language")).toBe("javascript");
    expect((editor as HTMLTextAreaElement).value).toBe("return 1;");
  });

  it("mounts the Monaco editor with Python language for a Python step", async () => {
    render(<Harness initial={{ language: "python", code: "return 1\n" }} />);
    const editor = await screen.findByTestId("monaco-fake");
    expect(editor.getAttribute("data-language")).toBe("python");
  });

  it("propagates edits via onChange (onStepChange path)", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <CodeConfigEditor
        config={{ language: "javascript", code: "" }}
        onChange={onChange}
      />,
    );
    const editor = await screen.findByTestId("monaco-fake");
    await user.type(editor, "x");
    expect(onChange).toHaveBeenCalled();
    const last = onChange.mock.calls[onChange.mock.calls.length - 1][0];
    expect(last).toEqual({ language: "javascript", code: "x" });
  });

  it("keeps the empty-code inline error when the source is blank", () => {
    render(<CodeConfigEditor config={{ language: "javascript", code: "" }} onChange={() => {}} />);
    expect(screen.getByTestId("builder-code-empty-error")).toBeInTheDocument();
  });

  it("hides the empty-code error once the source is non-blank", () => {
    render(
      <CodeConfigEditor
        config={{ language: "javascript", code: "return 1;" }}
        onChange={() => {}}
      />,
    );
    expect(screen.queryByTestId("builder-code-empty-error")).toBeNull();
  });

  it("rebinds language without losing the code when toggled JS → Python", async () => {
    const user = userEvent.setup();
    render(
      <Harness initial={{ language: "javascript", code: "console.log('hi')" }} />,
    );
    const editor = (await screen.findByTestId("monaco-fake")) as HTMLTextAreaElement;
    expect(editor.getAttribute("data-language")).toBe("javascript");
    expect(editor.value).toBe("console.log('hi')");

    const select = screen.getByTestId("builder-code-language") as HTMLSelectElement;
    await user.selectOptions(select, "python");

    const editorAfter = (await screen.findByTestId("monaco-fake")) as HTMLTextAreaElement;
    expect(editorAfter.getAttribute("data-language")).toBe("python");
    expect(editorAfter.value).toBe("console.log('hi')");
  });
});
