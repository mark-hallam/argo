import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ProposeWorkflowChat } from "./ProposeWorkflowChat";

const SAMPLE_TOML = `schema_version = "1"

[workflow]
name        = "weekly_feedback_triage"
description = "Test workflow."

[[step]]
name = "read_inbox"
kind = "tool_call"
tool = "file_read"
args = { path = "workspace/inbound/last-week.jsonl" }
`;

function stringStream(body: string): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  const chunks = [body.slice(0, 32), body.slice(32, 96), body.slice(96)];
  return new ReadableStream<Uint8Array>({
    start(controller) {
      for (const chunk of chunks) {
        if (chunk.length > 0) controller.enqueue(encoder.encode(chunk));
      }
      controller.close();
    },
  });
}

function makeFetch(handlers: Record<string, (init: RequestInit) => Response>): typeof fetch {
  return ((input: RequestInfo | URL, init: RequestInit = {}) => {
    const url = typeof input === "string" ? input : input.toString();
    for (const [prefix, handler] of Object.entries(handlers)) {
      if (url.startsWith(prefix)) {
        return Promise.resolve(handler(init));
      }
    }
    return Promise.reject(new Error(`unstubbed fetch: ${url}`));
  }) as unknown as typeof fetch;
}

describe("ProposeWorkflowChat", () => {
  it("streams a proposal then renders approve / edit / cancel", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    const onApproved = vi.fn();
    const fetchImpl = makeFetch({
      "/api/workflows/propose": () =>
        new Response(stringStream(SAMPLE_TOML), {
          status: 200,
          headers: { "Content-Type": "text/plain" },
        }),
    });
    render(
      <ProposeWorkflowChat
        workerId="wkr_1"
        verticalId="product-manager"
        approveEndpoint="/api/v1/verticals/product-manager/workflows"
        onCancel={onCancel}
        onApproved={onApproved}
        fetchImpl={fetchImpl}
      />,
    );

    const input = screen.getByLabelText("Describe the responsibility in plain English");
    await user.type(input, "every Monday cluster feedback");
    await user.click(screen.getByTestId("propose-workflow-submit"));

    await waitFor(() => {
      expect(screen.getByTestId("workflow-toml-body")).toHaveTextContent(
        /weekly_feedback_triage/,
      );
    });
    expect(screen.getByTestId("propose-workflow-edit")).toBeEnabled();
    expect(screen.getByTestId("propose-workflow-approve")).toBeEnabled();
  });

  it("posts the streamed TOML to the approve endpoint when Approve is clicked", async () => {
    const user = userEvent.setup();
    const onApproved = vi.fn();
    let approvedBody: string | null = null;
    let approvedUrl: string | null = null;
    const fetchImpl = ((input: RequestInfo | URL, init: RequestInit = {}) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.startsWith("/api/workflows/propose")) {
        return Promise.resolve(
          new Response(stringStream(SAMPLE_TOML), {
            status: 200,
            headers: { "Content-Type": "text/plain" },
          }),
        );
      }
      if (url.startsWith("/api/v1/verticals/")) {
        approvedUrl = url;
        approvedBody = typeof init.body === "string" ? init.body : "";
        return Promise.resolve(new Response("{}", { status: 201 }));
      }
      return Promise.reject(new Error(`unstubbed fetch: ${url}`));
    }) as unknown as typeof fetch;

    render(
      <ProposeWorkflowChat
        workerId="wkr_1"
        verticalId="product-manager"
        approveEndpoint="/api/v1/verticals/product-manager/workflows"
        onCancel={vi.fn()}
        onApproved={onApproved}
        fetchImpl={fetchImpl}
      />,
    );

    await user.type(
      screen.getByLabelText("Describe the responsibility in plain English"),
      "every Monday cluster feedback",
    );
    await user.click(screen.getByTestId("propose-workflow-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("propose-workflow-approve")).toBeEnabled();
    });
    await user.click(screen.getByTestId("propose-workflow-approve"));

    await waitFor(() => {
      expect(onApproved).toHaveBeenCalledWith("weekly_feedback_triage");
    });
    expect(approvedUrl).toBe(
      "/api/v1/verticals/product-manager/workflows/weekly_feedback_triage",
    );
    expect(approvedBody).toContain("weekly_feedback_triage");
  });

  it("Edit drops the user into a textarea pre-filled with the proposed TOML", async () => {
    const user = userEvent.setup();
    const fetchImpl = makeFetch({
      "/api/workflows/propose": () =>
        new Response(stringStream(SAMPLE_TOML), {
          status: 200,
          headers: { "Content-Type": "text/plain" },
        }),
    });
    render(
      <ProposeWorkflowChat
        workerId="wkr_1"
        verticalId="product-manager"
        approveEndpoint="/api/v1/verticals/product-manager/workflows"
        onCancel={vi.fn()}
        onApproved={vi.fn()}
        fetchImpl={fetchImpl}
      />,
    );

    await user.type(
      screen.getByLabelText("Describe the responsibility in plain English"),
      "draft something",
    );
    await user.click(screen.getByTestId("propose-workflow-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("propose-workflow-edit")).toBeEnabled();
    });
    await user.click(screen.getByTestId("propose-workflow-edit"));
    const editor = await screen.findByLabelText("Edit the proposed TOML");
    expect((editor as HTMLTextAreaElement).value).toContain("weekly_feedback_triage");
  });

  it("surfaces the parser error inline when the approve route returns 400", async () => {
    const user = userEvent.setup();
    const fetchImpl = ((input: RequestInfo | URL) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.startsWith("/api/workflows/propose")) {
        return Promise.resolve(
          new Response(stringStream(SAMPLE_TOML), {
            status: 200,
            headers: { "Content-Type": "text/plain" },
          }),
        );
      }
      if (url.startsWith("/api/v1/verticals/")) {
        return Promise.resolve(
          new Response("workflow: expected one of skill_invoke|prompt|tool_call", { status: 400 }),
        );
      }
      return Promise.reject(new Error(`unstubbed fetch: ${url}`));
    }) as unknown as typeof fetch;

    render(
      <ProposeWorkflowChat
        workerId="wkr_1"
        verticalId="product-manager"
        approveEndpoint="/api/v1/verticals/product-manager/workflows"
        onCancel={vi.fn()}
        onApproved={vi.fn()}
        fetchImpl={fetchImpl}
      />,
    );

    await user.type(
      screen.getByLabelText("Describe the responsibility in plain English"),
      "draft something",
    );
    await user.click(screen.getByTestId("propose-workflow-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("propose-workflow-approve")).toBeEnabled();
    });
    await user.click(screen.getByTestId("propose-workflow-approve"));
    const alert = await screen.findByTestId("propose-workflow-error");
    expect(alert).toHaveTextContent(/skill_invoke/);
  });

  it("Cancel calls the onCancel callback", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    const fetchImpl = makeFetch({
      "/api/workflows/propose": () =>
        new Response(stringStream(SAMPLE_TOML), {
          status: 200,
          headers: { "Content-Type": "text/plain" },
        }),
    });
    render(
      <ProposeWorkflowChat
        workerId="wkr_1"
        verticalId="product-manager"
        approveEndpoint="/api/v1/verticals/product-manager/workflows"
        onCancel={onCancel}
        onApproved={vi.fn()}
        fetchImpl={fetchImpl}
      />,
    );
    await user.type(
      screen.getByLabelText("Describe the responsibility in plain English"),
      "anything",
    );
    await user.click(screen.getByTestId("propose-workflow-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("propose-workflow-cancel")).toBeEnabled();
    });
    await user.click(screen.getByTestId("propose-workflow-cancel"));
    expect(onCancel).toHaveBeenCalled();
  });

  it("output region is announced via aria-live for streaming progress", async () => {
    const user = userEvent.setup();
    const fetchImpl = makeFetch({
      "/api/workflows/propose": () =>
        new Response(stringStream(SAMPLE_TOML), {
          status: 200,
          headers: { "Content-Type": "text/plain" },
        }),
    });
    render(
      <ProposeWorkflowChat
        workerId="wkr_1"
        verticalId="product-manager"
        approveEndpoint="/api/v1/verticals/product-manager/workflows"
        onCancel={vi.fn()}
        onApproved={vi.fn()}
        fetchImpl={fetchImpl}
      />,
    );
    await user.type(
      screen.getByLabelText("Describe the responsibility in plain English"),
      "draft something",
    );
    await user.click(screen.getByTestId("propose-workflow-submit"));
    const output = screen.getByTestId("propose-workflow-output");
    expect(output).toHaveAttribute("aria-live", "polite");
  });
});
