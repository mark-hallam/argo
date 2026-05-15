import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SetupPage from "./page";

const replaceMock = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ replace: replaceMock }),
}));

const snapshots = [
  {
    step: "awaiting_provider_key",
    prompt: "Anthropic API key (sk-ant-...)",
    complete: false,
    message: "step 2/5  provider",
  },
  {
    step: "awaiting_operator_name",
    prompt: "operator name",
    complete: false,
    message: "next: operator name",
  },
];

beforeEach(() => {
  replaceMock.mockClear();
  let idx = 0;
  global.fetch = vi.fn(async (input: RequestInfo | URL) => {
    const url = input.toString();
    if (url.endsWith("/api/auth/session")) {
      return Response.json({});
    }
    if (url.endsWith("/api/v1/wizard/answer")) {
      idx = Math.min(idx + 1, snapshots.length - 1);
      return Response.json(snapshots[idx]);
    }
    if (url.endsWith("/api/v1/wizard/reset")) {
      idx = 0;
      return Response.json(snapshots[0]);
    }
    return Response.json(snapshots[idx]);
  });
});

describe("SetupPage", () => {
  it("loads the current wizard prompt and submits an answer", async () => {
    const user = userEvent.setup();
    render(<SetupPage />);

    expect(await screen.findByText("Anthropic API key (sk-ant-...)")).toBeInTheDocument();
    await user.type(screen.getByTestId("setup-answer"), "sk-ant-test");
    await user.click(screen.getByTestId("setup-submit"));

    expect(await screen.findByText("operator name")).toBeInTheDocument();
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringMatching(/\/api\/v1\/wizard\/answer$/),
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("redirects when the gateway reports completion", async () => {
    global.fetch = vi.fn(async () =>
      Response.json({
        step: "complete",
        prompt: "Setup complete",
        complete: true,
        message: "operator configuration already exists",
      }),
    );

    render(<SetupPage />);

    await waitFor(() => expect(replaceMock).toHaveBeenCalledWith("/"));
  });
});
