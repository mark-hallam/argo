import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Step4Introduce } from "./Step4Introduce";
import type { WorkerDraft } from "@/lib/hire/draft-types";
import type { UseGenesisWorkerDraftResult } from "@/lib/hire/use-genesis-worker-draft";
import type { UseHireWorkerResult } from "@/lib/hooks/use-hire-worker";

const redraftStart = vi.fn();
const redraftReset = vi.fn();
let redraftMock: UseGenesisWorkerDraftResult = {
  start: redraftStart,
  reset: redraftReset,
  data: undefined,
  error: null,
  isPending: false,
  isSuccess: false,
  isError: false,
};

const hireCommit = vi.fn();
const hireReset = vi.fn();
let hireMock: UseHireWorkerResult = {
  commit: hireCommit,
  reset: hireReset,
  data: undefined,
  error: null,
  isPending: false,
  isSuccess: false,
  isError: false,
  mutation: {} as UseHireWorkerResult["mutation"],
};

vi.mock("@/lib/hire/use-genesis-worker-draft", () => ({
  useGenesisWorkerDraft: () => redraftMock,
}));
vi.mock("@/lib/hooks/use-hire-worker", () => ({
  useHireWorker: () => hireMock,
}));

function setRedraft(next: Partial<UseGenesisWorkerDraftResult>) {
  redraftMock = {
    start: redraftStart,
    reset: redraftReset,
    data: undefined,
    error: null,
    isPending: false,
    isSuccess: false,
    isError: false,
    ...next,
  };
}

function setHire(next: Partial<UseHireWorkerResult>) {
  hireMock = {
    commit: hireCommit,
    reset: hireReset,
    data: undefined,
    error: null,
    isPending: false,
    isSuccess: false,
    isError: false,
    mutation: {} as UseHireWorkerResult["mutation"],
    ...next,
  };
}

const BASE_DRAFT: WorkerDraft = {
  suggestedName: "Mariah",
  persona: {
    tagline: "Marketplace assistant",
    avatarKind: "initials",
    voiceProfile: { tone: "warm", formality: "medium", signaturePhrases: [] },
  },
  charter:
    "Mariah triages your Marketplace inbox, drafts replies in your voice, and checks in twice a day.",
  duties: [
    {
      label: "Morning summary of overnight messages",
      schedule: { kind: "cron", spec: "0 8 * * *" },
      defaultEnabled: true,
    },
    {
      label: "Draft replies to inbound messages",
      schedule: { kind: "event", spec: "message.inbound" },
      defaultEnabled: true,
    },
    {
      label: "Evening check-in",
      schedule: { kind: "cron", spec: "0 18 * * *" },
      defaultEnabled: false,
    },
  ],
  exampleReplies: [
    {
      inboundContext: "Is this still available?",
      mariahResponse: "Yes — would Thursday work for pickup?",
    },
    {
      inboundContext: "Will you take $40?",
      mariahResponse: "Let me check with the owner and get back to you.",
    },
    {
      inboundContext: "Can you ship?",
      mariahResponse: "I can check shipping — what's your zip code?",
    },
  ],
  approvalPolicy: { level: "balanced" },
  suggestedVertical: "marketplace-seller",
};

function Harness({
  initialDraft = BASE_DRAFT,
  onCommit,
  onPickTemplate,
}: {
  initialDraft?: WorkerDraft;
  onCommit?: (workerId: string) => void;
  onPickTemplate?: (slug: string) => void;
}) {
  const [draft, setDraft] = useState<WorkerDraft>(initialDraft);
  return (
    <div>
      <div data-testid="harness-name">{draft.suggestedName}</div>
      <div data-testid="harness-tagline">{draft.persona.tagline}</div>
      <div data-testid="harness-charter">{draft.charter}</div>
      <div data-testid="harness-duty-0-enabled">
        {String(draft.duties[0]?.defaultEnabled)}
      </div>
      <Step4Introduce
        workerDraft={draft}
        problem="I sell hockey gear."
        answers={{
          q1: { question_id: "q1", type: "free-text", value: "Mariah" },
        }}
        onChange={setDraft}
        onCommit={onCommit ?? (() => undefined)}
        onPickTemplate={(ex) => onPickTemplate?.(ex.id)}
      />
    </div>
  );
}

beforeEach(() => {
  redraftStart.mockClear();
  redraftReset.mockClear();
  hireCommit.mockClear();
  hireReset.mockClear();
  setRedraft({});
  setHire({});
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Step4Introduce", () => {
  it("renders the headline, worker card, duties, examples, and hire CTA", () => {
    render(<Harness />);
    expect(
      screen.getByRole("heading", { name: /Meet your new teammate\./i }),
    ).toBeInTheDocument();
    expect(screen.getByTestId("hire-step4-worker-card")).toBeInTheDocument();
    expect(screen.getByTestId("hire-step4-avatar")).toHaveTextContent("M");
    expect(screen.getAllByTestId(/^hire-step4-duty-\d+$/)).toHaveLength(3);
    expect(screen.getAllByTestId(/^hire-step4-example-\d+$/)).toHaveLength(3);
    expect(screen.getByTestId("hire-step4-hire")).toHaveTextContent(
      "Hire Mariah",
    );
  });

  it("commits name edits via the inline editor", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByTestId("hire-step4-name"));
    const input = await screen.findByTestId("hire-step4-name-input");
    await user.clear(input);
    await user.type(input, "Mara{Enter}");
    expect(screen.getByTestId("harness-name")).toHaveTextContent("Mara");
    // Hire button label updates to the new name.
    expect(screen.getByTestId("hire-step4-hire")).toHaveTextContent("Hire Mara");
  });

  it("toggling a duty updates the draft", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    expect(screen.getByTestId("harness-duty-0-enabled")).toHaveTextContent(
      "true",
    );
    await user.click(screen.getByTestId("hire-step4-duty-0-toggle"));
    expect(screen.getByTestId("harness-duty-0-enabled")).toHaveTextContent(
      "false",
    );
  });

  it("opens the Adjust drawer and fires draftWorker with the adjustment text", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByTestId("hire-step4-adjust"));
    expect(screen.getByTestId("hire-step4-adjust-drawer")).toBeInTheDocument();
    const textarea = await screen.findByTestId("hire-step4-adjust-textarea");
    await user.type(textarea, "Make her more formal");
    await user.click(screen.getByTestId("hire-step4-adjust-save"));
    expect(redraftStart).toHaveBeenCalledTimes(1);
    expect(redraftStart).toHaveBeenCalledWith({
      problem: "I sell hockey gear.",
      answers: {
        q1: { question_id: "q1", type: "free-text", value: "Mariah" },
      },
      adjustment: "Make her more formal",
    });
  });

  it("surfaces the template fallback in the drawer after 3 successful adjusts", async () => {
    const user = userEvent.setup();
    const onPickTemplate = vi.fn();

    const RERENDER_DRAFT: WorkerDraft = { ...BASE_DRAFT, charter: "v2" };

    // Drive 3 saves by flipping the redraft mock between pending → success
    // for each round. After 3 successes, the drawer's adjustCount === 3 and
    // the template fallback renders.
    const { rerender } = render(
      <Harness onPickTemplate={(slug) => onPickTemplate(slug)} />,
    );

    for (let i = 0; i < 3; i++) {
      await user.click(screen.getByTestId("hire-step4-adjust"));
      const ta = await screen.findByTestId("hire-step4-adjust-textarea");
      await user.type(ta, `round-${i}`);
      // Toggle to success with a fresh data object so the latch picks it up.
      act(() => {
        setRedraft({
          isSuccess: true,
          data: { draft: { ...RERENDER_DRAFT, charter: `v${i}` }, source: "stub" },
        });
      });
      rerender(<Harness onPickTemplate={(slug) => onPickTemplate(slug)} />);
      act(() => {
        setRedraft({});
      });
      rerender(<Harness onPickTemplate={(slug) => onPickTemplate(slug)} />);
    }

    // Re-open the drawer for the 4th attempt; fallback should now render.
    await user.click(screen.getByTestId("hire-step4-adjust"));
    await waitFor(() =>
      expect(
        screen.getByTestId("hire-step4-adjust-template-fallback"),
      ).toBeInTheDocument(),
    );
    await user.click(
      screen.getByTestId("hire-step4-adjust-template-marketplace-seller"),
    );
    expect(onPickTemplate).toHaveBeenCalledWith("marketplace-seller");
  });

  it("Hire button fires the hire commit mutation with the full hire context", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByTestId("hire-step4-hire"));
    expect(hireCommit).toHaveBeenCalledTimes(1);
    expect(hireCommit).toHaveBeenCalledWith({
      workerDraft: BASE_DRAFT,
      problem: "I sell hockey gear.",
      answers: {
        q1: { question_id: "q1", type: "free-text", value: "Mariah" },
      },
    });
  });

  it("calls onCommit with the workerId after a successful hire mutation", async () => {
    const user = userEvent.setup();
    const onCommit = vi.fn();
    const { rerender } = render(<Harness onCommit={onCommit} />);
    await user.click(screen.getByTestId("hire-step4-hire"));
    act(() => {
      setHire({
        isSuccess: true,
        data: { workerId: "stub-abc123", success: true },
      });
    });
    rerender(<Harness onCommit={onCommit} />);
    expect(onCommit).toHaveBeenCalledWith("stub-abc123");
  });

  it("disables the Hire button while the Adjust drawer is open", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByTestId("hire-step4-adjust"));
    expect(screen.getByTestId("hire-step4-hire")).toBeDisabled();
  });

  it("disables the Hire button while a redraft is in flight", () => {
    setRedraft({ isPending: true });
    render(<Harness />);
    expect(screen.getByTestId("hire-step4-hire")).toBeDisabled();
  });

  it("surfaces the error toast with a Try-again CTA on hire failure", async () => {
    const user = userEvent.setup();
    const { rerender } = render(<Harness />);
    await user.click(screen.getByTestId("hire-step4-hire"));
    act(() => {
      setHire({ isError: true, error: new Error("gateway 500") });
    });
    rerender(<Harness />);
    await waitFor(() =>
      expect(screen.getByTestId("hire-step4-error-toast")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("hire-step4-error-toast")).toHaveTextContent(
      "Couldn't hire Mariah",
    );
    expect(screen.getByTestId("hire-step4-error-toast")).toHaveTextContent(
      "gateway 500",
    );
    // Try-again resets the mutation and re-commits.
    act(() => {
      setHire({ isError: false, error: null });
    });
    rerender(<Harness />);
    await user.click(screen.getByTestId("hire-step4-error-retry"));
    expect(hireReset).toHaveBeenCalled();
    expect(hireCommit).toHaveBeenCalledTimes(2);
  });

  it("closes the Adjust drawer when the redraft succeeds", async () => {
    const user = userEvent.setup();
    const { rerender } = render(<Harness />);
    await user.click(screen.getByTestId("hire-step4-adjust"));
    const ta = await screen.findByTestId("hire-step4-adjust-textarea");
    await user.type(ta, "shorter replies");
    await user.click(screen.getByTestId("hire-step4-adjust-save"));
    act(() => {
      setRedraft({
        isSuccess: true,
        data: {
          draft: { ...BASE_DRAFT, charter: "Shorter, no exclamation marks." },
          source: "stub",
        },
      });
    });
    rerender(<Harness />);
    await waitFor(() =>
      expect(
        screen.queryByTestId("hire-step4-adjust-drawer"),
      ).not.toBeInTheDocument(),
    );
    // Updated charter is reflected via onChange.
    expect(screen.getByTestId("harness-charter")).toHaveTextContent(
      "Shorter, no exclamation marks.",
    );
  });

  it("cancels editing on Esc and rolls the field back", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByTestId("hire-step4-name"));
    const input = await screen.findByTestId("hire-step4-name-input");
    await user.clear(input);
    await user.type(input, "Different");
    // Use fireEvent so Esc isn't swallowed by user-event's Radix portal logic.
    fireEvent.keyDown(input, { key: "Escape" });
    expect(screen.getByTestId("harness-name")).toHaveTextContent("Mariah");
  });
});
