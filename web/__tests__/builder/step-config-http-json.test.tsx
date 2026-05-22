import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import { HttpRequestConfigEditor } from "@/app/(app)/workflows/[verticalId]/[name]/build/step-config-http";
import { defaultHttpRequestConfig } from "@/app/(app)/workflows/[verticalId]/[name]/build/builder-handlers";
import type { HttpRequestConfig } from "@/app/(app)/workflows/[verticalId]/[name]/build/types";

function Harness({ initial }: { initial?: Partial<HttpRequestConfig> }) {
  const [cfg, setCfg] = useState<HttpRequestConfig>({
    ...defaultHttpRequestConfig(),
    ...initial,
  });
  return <HttpRequestConfigEditor config={cfg} onChange={setCfg} />;
}

describe("HttpRequestConfigEditor — JSON body awareness", () => {
  it("pretty-prints valid JSON on blur", () => {
    render(<Harness initial={{ body: "" }} />);
    const ta = screen.getByTestId("builder-http-body") as HTMLTextAreaElement;
    fireEvent.change(ta, { target: { value: '{"a":1,"b":2}' } });
    fireEvent.blur(ta);
    const after = screen.getByTestId("builder-http-body") as HTMLTextAreaElement;
    expect(after.value).toBe('{\n  "a": 1,\n  "b": 2\n}');
    expect(screen.queryByTestId("builder-http-body-json-error")).toBeNull();
  });

  it("shows inline parse error when body looks like JSON but is invalid", () => {
    render(<Harness initial={{ body: "{a:1}" }} />);
    const err = screen.getByTestId("builder-http-body-json-error");
    expect(err.textContent).toMatch(/^JSON parse error at line 1: /);
  });

  it("shows no error for an empty body", () => {
    render(<Harness initial={{ body: "" }} />);
    expect(screen.queryByTestId("builder-http-body-json-error")).toBeNull();
  });

  it("shows no error for plain-text (non-JSON-shaped) body", () => {
    render(<Harness initial={{ body: "Hello world" }} />);
    expect(screen.queryByTestId("builder-http-body-json-error")).toBeNull();
  });

  it("shows no error for valid JSON and leaves the body unchanged on blur when already pretty", () => {
    const pretty = '{\n  "a": 1\n}';
    render(<Harness initial={{ body: pretty }} />);
    const ta = screen.getByTestId("builder-http-body") as HTMLTextAreaElement;
    expect(ta.value).toBe(pretty);
    fireEvent.blur(ta);
    const after = screen.getByTestId("builder-http-body") as HTMLTextAreaElement;
    expect(after.value).toBe(pretty);
    expect(screen.queryByTestId("builder-http-body-json-error")).toBeNull();
  });

  it("does not pretty-print when content is not JSON-shaped", () => {
    render(<Harness initial={{ body: "plain text body" }} />);
    const ta = screen.getByTestId("builder-http-body") as HTMLTextAreaElement;
    fireEvent.blur(ta);
    const after = screen.getByTestId("builder-http-body") as HTMLTextAreaElement;
    expect(after.value).toBe("plain text body");
  });
});
