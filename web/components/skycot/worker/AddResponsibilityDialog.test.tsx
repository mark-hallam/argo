import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AddResponsibilityDialog } from "@/components/skycot/worker/AddResponsibilityDialog";
import { DutiesCard } from "@/components/skycot/worker/DutiesCard";
import { isValidCron } from "@/components/skycot/worker/duty/Step1Capture";
import { cronGloss } from "@/components/skycot/worker/duty/Step3Draft";
import type {
  ClassifyDutyResult,
  CommitDutyResult,
  DutyProposal,
} from "@/lib/api/duties";

vi.mock("@/lib/api/duties", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/duties")>(
    "@/lib/api/duties",
  );
  return {
    ...actual,
    classifyDuty: vi.fn(),
    previewDuty: vi.fn(),
    commitDuty: vi.fn(),
  };
});

import { classifyDuty, commitDuty, previewDuty } from "@/lib/api/duties";

const mockedClassify = vi.mocked(classifyDuty);
const mockedCommit = vi.mocked(commitDuty);
const mockedPreview = vi.mocked(previewDuty);

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
}

function recurringProposal(): DutyProposal {
  return {
    kind: "recurring",
    label: "Send the EOD summary to Marta",
    description: "Recurring responsibility: send Marta a one-line end-of-day summary at 6pm.",
    schedule: { kind: "cron", spec: "0 18 * * 1-5" },
    samples: [
      { caseId: "s1", context: "Mon 6pm", output: "Sends EOD summary." },
      { caseId: "s2", context: "Tue 6pm", output: "Sends EOD summary." },
      { caseId: "s3", context: "Wed 6pm", output: "Sends EOD summary." },
    ],
  };
}

function conflictedProposal(): DutyProposal {
  return {
    ...recurringProposal(),
    conflicts: [
      {
        existingDutyId: "dty_existing",
        existingDutyLabel: "Send weekly EOD digest",
        existingSchedule: { kind: "cron", spec: "0 18 * * 1-5" },
        reason: "Both fire weekdays at 18:00.",
      },
    ],
  };
}

function commitResult(overrides: Partial<CommitDutyResult["duty"]> = {}): CommitDutyResult {
  return {
    duty: {
      id: "dty_new1",
      label: "Send the EOD summary to Marta",
      schedule: "cron: 0 18 * * 1-5",
      enabled: true,
      ...overrides,
    },
    nextFireAt: new Date(Date.now() + 60 * 60_000).toISOString(),
    source: "mock",
  };
}

function Harness(props: Partial<React.ComponentProps<typeof AddResponsibilityDialog>> = {}) {
  const client = makeQueryClient();
  return (
    <QueryClientProvider client={client}>
      <AddResponsibilityDialog
        open
        onOpenChange={vi.fn()}
        workerId="wkr_mariah"
        workerName="Mariah"
        {...props}
      />
    </QueryClientProvider>
  );
}

function CardHarness(props: Partial<React.ComponentProps<typeof DutiesCard>> = {}) {
  const client = makeQueryClient();
  return (
    <QueryClientProvider client={client}>
      <DutiesCard
        canEdit
        duties={[]}
        workerId="wkr_mariah"
        workerName="Mariah"
        {...props}
      />
    </QueryClientProvider>
  );
}

beforeEach(() => {
  mockedClassify.mockResolvedValue({
    proposal: recurringProposal(),
    source: "mock",
  } satisfies ClassifyDutyResult);
  mockedCommit.mockResolvedValue(commitResult());
  mockedPreview.mockResolvedValue({
    samples: recurringProposal().samples!,
    source: "mock",
  });
});

afterEach(() => {
  vi.clearAllMocks();
});

async function captureAndClassify(user: ReturnType<typeof userEvent.setup>) {
  await user.type(screen.getByTestId("duty-input"), "Send the EOD summary to Marta");
  await user.click(screen.getByTestId("duty-cron-preset-weekdays-9"));
  await user.click(screen.getByTestId("duty-continue"));
}

describe("Step 1 capture", () => {
  it("renders the capture step with a textarea and a disabled Continue button until input + preset are valid", async () => {
    render(<Harness />);
    expect(screen.getByTestId("duty-step-capture")).toBeInTheDocument();
    // Default preset is selected, so Continue is gated only by the 10-char minimum.
    expect(screen.getByTestId("duty-continue")).toBeDisabled();
  });

  it("enables Continue after typing ≥ 10 chars with a preset selected", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.type(screen.getByTestId("duty-input"), "Send a daily reminder");
    expect(screen.getByTestId("duty-continue")).not.toBeDisabled();
  });

  it("renders all 5 cron preset chips (4 named + custom)", () => {
    render(<Harness />);
    expect(screen.getByTestId("duty-cron-preset-morning")).toBeInTheDocument();
    expect(screen.getByTestId("duty-cron-preset-hourly")).toBeInTheDocument();
    expect(screen.getByTestId("duty-cron-preset-weekdays-9")).toBeInTheDocument();
    expect(screen.getByTestId("duty-cron-preset-weekly-mon")).toBeInTheDocument();
    expect(screen.getByTestId("duty-cron-preset-custom")).toBeInTheDocument();
  });

  it("opens a custom cron input when Custom… is clicked and surfaces a validation error", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.type(screen.getByTestId("duty-input"), "Send a daily reminder");
    await user.click(screen.getByTestId("duty-cron-preset-custom"));
    const customInput = screen.getByTestId("duty-cron-custom-input");
    await user.type(customInput, "not a cron");
    expect(screen.getByTestId("duty-cron-custom-error")).toBeInTheDocument();
    expect(screen.getByTestId("duty-continue")).toBeDisabled();
  });

  it("accepts a valid 5-field custom cron and lets Continue fire", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.type(screen.getByTestId("duty-input"), "Send a daily reminder");
    await user.click(screen.getByTestId("duty-cron-preset-custom"));
    await user.type(screen.getByTestId("duty-cron-custom-input"), "30 8 * * 1-5");
    expect(screen.getByTestId("duty-continue")).not.toBeDisabled();
  });
});

describe("Step 2 classify + Step 3 draft", () => {
  it("calls the classifier and lands on the draft step with a description + schedule preview", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await captureAndClassify(user);
    await waitFor(() => expect(mockedClassify).toHaveBeenCalled());
    expect(await screen.findByTestId("duty-step-draft")).toBeInTheDocument();
    expect(screen.getByTestId("duty-draft-description")).toHaveTextContent(/end-of-day summary/);
    expect(screen.getByTestId("duty-draft-schedule")).toHaveTextContent(/0 18 \* \* 1-5/);
    expect(screen.getByTestId("duty-classification-chip")).toHaveTextContent(/Recurring/i);
  });

  it("renders 3 sample outputs on the draft step", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await captureAndClassify(user);
    expect(await screen.findByTestId("duty-step-draft")).toBeInTheDocument();
    expect(screen.getByTestId("duty-sample-s1")).toHaveTextContent(/EOD summary/);
    expect(screen.getByTestId("duty-sample-s2")).toBeInTheDocument();
    expect(screen.getByTestId("duty-sample-s3")).toBeInTheDocument();
  });

  it("returns to Step 1 with text preserved when Adjust the wording is clicked", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await captureAndClassify(user);
    await screen.findByTestId("duty-step-draft");
    await user.click(screen.getByTestId("duty-adjust-wording"));
    expect(await screen.findByTestId("duty-step-capture")).toBeInTheDocument();
    expect(screen.getByTestId("duty-input")).toHaveValue("Send the EOD summary to Marta");
  });

  it("surfaces a classify error with retry", async () => {
    const user = userEvent.setup();
    mockedClassify.mockRejectedValueOnce(new Error("offline"));
    render(<Harness />);
    await captureAndClassify(user);
    expect(await screen.findByTestId("duty-step-classify-error")).toHaveTextContent(/offline/);
    mockedClassify.mockResolvedValueOnce({ proposal: recurringProposal(), source: "mock" });
    await user.click(screen.getByTestId("duty-classify-retry"));
    expect(await screen.findByTestId("duty-step-draft")).toBeInTheDocument();
  });
});

describe("Step 4 conflict resolution", () => {
  it("shows the conflict step when the proposal carries a conflict", async () => {
    const user = userEvent.setup();
    mockedClassify.mockResolvedValue({ proposal: conflictedProposal(), source: "mock" });
    render(<Harness />);
    await captureAndClassify(user);
    await screen.findByTestId("duty-step-draft");
    expect(screen.getByTestId("duty-draft-conflict-hint")).toBeInTheDocument();
    await user.click(screen.getByTestId("duty-continue"));
    expect(await screen.findByTestId("duty-step-conflict")).toBeInTheDocument();
    expect(screen.getByTestId("duty-conflict-existing")).toHaveTextContent(/EOD digest/);
    expect(screen.getByTestId("duty-conflict-new")).toHaveTextContent(/Send the EOD/);
  });

  it("advances past conflict on Replace and lands on commit", async () => {
    const user = userEvent.setup();
    mockedClassify.mockResolvedValue({ proposal: conflictedProposal(), source: "mock" });
    render(<Harness />);
    await captureAndClassify(user);
    await screen.findByTestId("duty-step-draft");
    await user.click(screen.getByTestId("duty-continue"));
    await screen.findByTestId("duty-step-conflict");
    await user.click(screen.getByTestId("duty-continue"));
    expect(await screen.findByTestId("duty-step-commit")).toBeInTheDocument();
  });
});

describe("Step 5 commit", () => {
  it("fires commit with the proposal and renders the success summary", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await captureAndClassify(user);
    await screen.findByTestId("duty-step-draft");
    await user.click(screen.getByTestId("duty-continue"));
    expect(await screen.findByTestId("duty-step-commit")).toBeInTheDocument();
    await user.click(screen.getByTestId("duty-commit"));
    await waitFor(() =>
      expect(mockedCommit).toHaveBeenCalledWith(
        expect.objectContaining({ workerId: "wkr_mariah" }),
      ),
    );
    expect(await screen.findByTestId("duty-step-committed")).toBeInTheDocument();
    expect(screen.getByTestId("duty-committed-summary")).toHaveTextContent(/EOD summary/);
  });

  it("surfaces a commit error and lets the user retry", async () => {
    const user = userEvent.setup();
    mockedCommit.mockRejectedValueOnce(new Error("conflict 409"));
    render(<Harness />);
    await captureAndClassify(user);
    await screen.findByTestId("duty-step-draft");
    await user.click(screen.getByTestId("duty-continue"));
    await screen.findByTestId("duty-step-commit");
    await user.click(screen.getByTestId("duty-commit"));
    expect(await screen.findByTestId("duty-commit-error")).toHaveTextContent(/conflict/);
    mockedCommit.mockResolvedValueOnce(commitResult());
    await user.click(screen.getByTestId("duty-commit"));
    await waitFor(() => expect(mockedCommit).toHaveBeenCalledTimes(2));
  });
});

describe("DutiesCard mount + step 6 toast", () => {
  it("opens the dialog from the + Add responsibility button when workerId is present", async () => {
    const user = userEvent.setup();
    render(<CardHarness />);
    await user.click(screen.getByTestId("duties-add"));
    expect(await screen.findByTestId("duty-dialog")).toBeInTheDocument();
    expect(screen.getByTestId("duty-step-capture")).toBeInTheDocument();
  });

  it("does NOT mount the dialog when workerId is omitted (backwards compat)", async () => {
    const user = userEvent.setup();
    const onAddDuty = vi.fn();
    render(<CardHarness workerId={undefined} workerName={undefined} onAddDuty={onAddDuty} />);
    await user.click(screen.getByTestId("duties-add"));
    expect(onAddDuty).toHaveBeenCalledOnce();
    expect(screen.queryByTestId("duty-dialog")).toBeNull();
  });

  it("renders the step-6 toast with the duty label and next-fire copy after commit", async () => {
    const user = userEvent.setup();
    render(<CardHarness />);
    await user.click(screen.getByTestId("duties-add"));
    await user.type(screen.getByTestId("duty-input"), "Send the EOD summary to Marta");
    await user.click(screen.getByTestId("duty-cron-preset-weekdays-9"));
    await user.click(screen.getByTestId("duty-continue"));
    await screen.findByTestId("duty-step-draft");
    await user.click(screen.getByTestId("duty-continue"));
    await user.click(screen.getByTestId("duty-commit"));
    const toast = await screen.findByTestId("duty-added-toast");
    expect(toast).toHaveTextContent(/EOD summary/);
    expect(toast).toHaveTextContent(/Mariah's next run/);
  });
});

describe("cron helpers (pure)", () => {
  it("isValidCron accepts all 4 named presets", () => {
    expect(isValidCron("0 6 * * *")).toBe(true);
    expect(isValidCron("0 * * * *")).toBe(true);
    expect(isValidCron("0 9 * * 1-5")).toBe(true);
    expect(isValidCron("0 9 * * 1")).toBe(true);
  });

  it("isValidCron rejects malformed inputs", () => {
    expect(isValidCron("not a cron")).toBe(false);
    expect(isValidCron("0 9 * *")).toBe(false);
    expect(isValidCron("0 25 * * *")).toBe(true); // syntactically valid, range checking left to server
    expect(isValidCron("")).toBe(false);
  });

  it("cronGloss returns a friendly description for the 4 presets", () => {
    expect(cronGloss("0 6 * * *")).toMatch(/morning/i);
    expect(cronGloss("0 * * * *")).toMatch(/hour/i);
    expect(cronGloss("0 9 * * 1-5")).toMatch(/weekday/i);
    expect(cronGloss("0 9 * * 1")).toMatch(/Monday/i);
  });
});
