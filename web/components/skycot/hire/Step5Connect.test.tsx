import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Step5Connect } from "./Step5Connect";
import type { WorkerDraft } from "@/lib/hire/draft-types";

const connectMock = vi.fn();
let pendingConnectorId: string | null = null;
let storedOnResult: ((r: {
  connectorId: string;
  status:
    | "connected"
    | "not_connected"
    | "connect_failed"
    | "connecting"
    | "reauth_needed";
  error?: string;
}) => void) | null = null;

vi.mock("@/lib/hire/use-connect-flow", () => ({
  useConnectFlow: (opts: { onResult: typeof storedOnResult }) => {
    storedOnResult = opts.onResult;
    return {
      connect: connectMock,
      pendingConnectorId,
    };
  },
}));

const disconnectMock = vi.fn<(workerId: string, connectorId: string) => Promise<unknown>>(
  async () => ({ workerId: "w_1", connectorId: "gmail", ok: true }),
);

vi.mock("@/lib/api/connectors", async (orig) => {
  const actual = (await orig()) as Record<string, unknown>;
  return {
    ...actual,
    disconnectConnector: (workerId: string, connectorId: string) =>
      disconnectMock(workerId, connectorId),
  };
});

const apiPatchMock = vi.fn<(path: string, body?: unknown) => Promise<unknown>>(
  async () => undefined,
);
vi.mock("@/lib/api/client", async (orig) => {
  const actual = (await orig()) as Record<string, unknown>;
  return {
    ...actual,
    apiPatch: (path: string, body?: unknown) => apiPatchMock(path, body),
  };
});

const BASE_DRAFT: WorkerDraft & { workerId: string } = {
  suggestedName: "Mariah",
  persona: {
    tagline: "Marketplace assistant",
    avatarKind: "initials",
    voiceProfile: { tone: "warm", formality: "medium", signaturePhrases: [] },
  },
  charter: "Helps with messages.",
  duties: [],
  exampleReplies: [],
  approvalPolicy: { level: "balanced" },
  suggestedVertical: "marketplace-seller",
  workerId: "w_1",
};

function renderStep5(overrides?: {
  draft?: WorkerDraft & { workerId: string };
  onContinue?: () => void;
  onSkip?: () => void;
  onBack?: () => void;
  hint?: string | null;
}) {
  const onContinue = overrides?.onContinue ?? vi.fn();
  const onSkip = overrides?.onSkip ?? vi.fn();
  const onBack = overrides?.onBack ?? vi.fn();
  return {
    onContinue,
    onSkip,
    onBack,
    ...render(
      <Step5Connect
        workerDraft={overrides?.draft ?? BASE_DRAFT}
        defaultChannelHint={overrides?.hint ?? null}
        onContinue={onContinue}
        onSkip={onSkip}
        onBack={onBack}
      />,
    ),
  };
}

beforeEach(() => {
  connectMock.mockClear();
  disconnectMock.mockClear();
  apiPatchMock.mockClear();
  pendingConnectorId = null;
  storedOnResult = null;
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Step5Connect", () => {
  it("renders the headline using the worker's name and a vertical-relevant connector list", () => {
    renderStep5();
    expect(
      screen.getByRole("heading", { name: /Mariah needs access to a few things\./i }),
    ).toBeInTheDocument();
    // Marketplace-seller surfaces fb / gumtree / ebay at minimum.
    expect(
      screen.getByTestId("hire-step5-connector-facebook-marketplace"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("hire-step5-connector-ebay")).toBeInTheDocument();
    expect(screen.getByTestId("hire-step5-connector-gmail")).toBeInTheDocument();
    // Cap honored — only the row containers carry data-status.
    const list = screen.getByTestId("hire-step5-connectors");
    const rows = list.querySelectorAll("[data-status]");
    expect(rows.length).toBeGreaterThanOrEqual(3);
    expect(rows.length).toBeLessThanOrEqual(6);
  });

  it("connecting an OAuth row opens the popup via the hook and flips status to Connecting…", async () => {
    const user = userEvent.setup();
    renderStep5();
    await user.click(screen.getByTestId("hire-step5-connector-gmail-connect"));
    expect(connectMock).toHaveBeenCalledWith({
      connectorId: "gmail",
      authType: "oauth",
    });
    expect(
      screen.getByTestId("hire-step5-connector-gmail-status"),
    ).toHaveTextContent(/Connecting…/);
  });

  it("computer-use rows use the 'Sign in on this device' button label", () => {
    renderStep5();
    expect(
      screen.getByTestId("hire-step5-connector-facebook-marketplace-connect"),
    ).toHaveTextContent(/Sign in on this device/);
  });

  it("success postMessage flips the row to Connected", async () => {
    const user = userEvent.setup();
    renderStep5();
    await user.click(screen.getByTestId("hire-step5-connector-gmail-connect"));
    act(() => {
      storedOnResult?.({ connectorId: "gmail", status: "connected" });
    });
    await waitFor(() =>
      expect(
        screen.getByTestId("hire-step5-connector-gmail-status"),
      ).toHaveTextContent(/Connected/),
    );
    expect(
      screen.getByTestId("hire-step5-connector-gmail-disconnect"),
    ).toBeInTheDocument();
  });

  it("popup closed before completion reverts the row to Not connected", async () => {
    const user = userEvent.setup();
    renderStep5();
    await user.click(screen.getByTestId("hire-step5-connector-gmail-connect"));
    act(() => {
      storedOnResult?.({ connectorId: "gmail", status: "not_connected" });
    });
    await waitFor(() =>
      expect(
        screen.getByTestId("hire-step5-connector-gmail-status"),
      ).toHaveTextContent(/Not connected/),
    );
  });

  it("connect failure shows the error reason and a Retry CTA", async () => {
    const user = userEvent.setup();
    renderStep5();
    await user.click(screen.getByTestId("hire-step5-connector-gmail-connect"));
    act(() => {
      storedOnResult?.({
        connectorId: "gmail",
        status: "connect_failed",
        error: "Token denied",
      });
    });
    await waitFor(() =>
      expect(
        screen.getByTestId("hire-step5-connector-gmail-status"),
      ).toHaveTextContent(/Connect failed/),
    );
    expect(
      screen.getByTestId("hire-step5-connector-gmail-error"),
    ).toHaveTextContent("Token denied");
    expect(
      screen.getByTestId("hire-step5-connector-gmail-retry"),
    ).toBeInTheDocument();
  });

  it("disconnect raises a confirm and clears the row after confirmation", async () => {
    const user = userEvent.setup();
    renderStep5();
    // Drive into Connected via the postMessage path.
    await user.click(screen.getByTestId("hire-step5-connector-gmail-connect"));
    act(() => {
      storedOnResult?.({ connectorId: "gmail", status: "connected" });
    });
    await user.click(
      screen.getByTestId("hire-step5-connector-gmail-disconnect"),
    );
    expect(screen.getByText(/Disconnect this tool\?/i)).toBeInTheDocument();
    fireEvent.click(screen.getAllByTestId("alert-dialog-confirm")[0]);
    await waitFor(() =>
      expect(
        screen.getByTestId("hire-step5-connector-gmail-status"),
      ).toHaveTextContent(/Not connected/),
    );
    expect(disconnectMock).toHaveBeenCalledWith("w_1", "gmail");
  });

  it("Skip button opens a confirm dialog and onSkip fires on confirm", async () => {
    const user = userEvent.setup();
    const onSkip = vi.fn();
    renderStep5({ onSkip });
    await user.click(screen.getByTestId("hire-step5-skip"));
    expect(
      screen.getByText(
        /Mariah will be hired but won't have anything to work on yet\./i,
      ),
    ).toBeInTheDocument();
    fireEvent.click(screen.getAllByTestId("alert-dialog-confirm")[0]);
    expect(onSkip).toHaveBeenCalledWith("w_1");
  });

  it("Continue PATCHes the channel state and routes via onContinue", async () => {
    const user = userEvent.setup();
    const onContinue = vi.fn();
    renderStep5({ onContinue });
    await user.click(screen.getByTestId("hire-step5-continue"));
    await waitFor(() => expect(onContinue).toHaveBeenCalledWith("w_1"));
    expect(apiPatchMock).toHaveBeenCalledWith(
      "/api/v1/workers/w_1",
      expect.objectContaining({
        channels: expect.objectContaining({ primary: "slack" }),
      }),
    );
  });

  it("Back button calls onBack", async () => {
    const user = userEvent.setup();
    const onBack = vi.fn();
    renderStep5({ onBack });
    await user.click(screen.getByTestId("hire-step5-back"));
    expect(onBack).toHaveBeenCalled();
  });

  it("toggling a second channel adds it as fallback and the star moves to it on demand", async () => {
    const user = userEvent.setup();
    renderStep5();
    // Default primary is Slack — selected with star.
    expect(screen.getByTestId("hire-step5-channel-slack")).toHaveAttribute(
      "data-on",
      "true",
    );
    expect(screen.getByTestId("hire-step5-channel-slack")).toHaveAttribute(
      "data-primary",
      "true",
    );
    await user.click(screen.getByTestId("hire-step5-channel-email-toggle"));
    expect(screen.getByTestId("hire-step5-channel-email")).toHaveAttribute(
      "data-on",
      "true",
    );
    // Star still on Slack.
    expect(screen.getByTestId("hire-step5-channel-email")).toHaveAttribute(
      "data-primary",
      "false",
    );
    await user.click(screen.getByTestId("hire-step5-channel-email-primary"));
    expect(screen.getByTestId("hire-step5-channel-email")).toHaveAttribute(
      "data-primary",
      "true",
    );
    expect(screen.getByTestId("hire-step5-channel-slack")).toHaveAttribute(
      "data-primary",
      "false",
    );
  });

  it("auto-detects the primary channel from the hint", () => {
    renderStep5({ hint: "email" });
    expect(screen.getByTestId("hire-step5-channel-email")).toHaveAttribute(
      "data-primary",
      "true",
    );
    expect(screen.getByTestId("hire-step5-channel-slack")).toHaveAttribute(
      "data-on",
      "false",
    );
  });
});
