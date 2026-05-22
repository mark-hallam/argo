"use client";
import Editor from "@monaco-editor/react";
import { useTheme } from "next-themes";

export type MonacoCodeLanguage = "javascript" | "python";

interface Props {
  value: string;
  language: MonacoCodeLanguage;
  onChange: (next: string) => void;
}

// Default export so `next/dynamic`'s `() => import("...")` works without
// a `.then(m => m.X)` adapter — keeps the dynamic-import call site at
// the textarea swap-in site tidy.
export default function MonacoCodeEditor({ value, language, onChange }: Props) {
  const { resolvedTheme } = useTheme();
  const monacoTheme = resolvedTheme === "dark" ? "vs-dark" : "vs";

  return (
    <Editor
      data-testid="builder-code-monaco"
      height="320px"
      language={language}
      value={value}
      theme={monacoTheme}
      onChange={(next) => onChange(next ?? "")}
      options={{
        tabSize: 2,
        insertSpaces: true,
        minimap: { enabled: false },
        lineNumbers: "on",
        wordWrap: "off",
        fontSize: 12.5,
        fontFamily: "var(--font-mono), ui-monospace, monospace",
        scrollBeyondLastLine: false,
        automaticLayout: true,
        renderLineHighlight: "line",
        padding: { top: 8, bottom: 8 },
      }}
    />
  );
}
