import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { WorkerRetroDialog } from "@/components/skycot/worker/WorkerRetroDialog";
import type { Retro } from "@/lib/api/retros";

function sampleRetro(overrides: Partial<Retro> = {}): Retro {
  return {
    retroId: "retro_1",
    workerId: "wkr_priya",
    windowStart: 1_700_000_000,
    windowEnd: 1_700_604_800,
    metrics: {
      actions_total: 12,
      actions_approved: 9,
      actions_refused: 1,
      escalations: 2,
      learned_skills_added: 1,
      error_count_by_category: { halt: 2 },
    },
    sections: {
      surface_metrics: "",
      what_went_well: "",
      surprises: "",
      what_to_change: "",
      action_items: "",
    },
    createdAt: 1_700_604_900,
    finalizedAt: null,
    finalized: false,
    ...overrides,
  };
}

function renderDialog(
  retro: Retro | null = sampleRetro(),
  onSaveSections = vi.fn().mockResolvedValue({}),
  onFinalize = vi.fn().mockResolvedValue({}),
) {
  const onOpenChange = vi.fn();
  render(
    <WorkerRetroDialog
      open
      onOpenChange={onOpenChange}
      retro={retro}
      workerName="Priya"
      onSaveSections={onSaveSections}
      onFinalize={onFinalize}
    />,
  );
  return { onSaveSections, onFinalize, onOpenChange };
}

describe("WorkerRetroDialog", () => {
  it("opens on step 1 (surface metrics) and renders the aggregated counts", () => {
    renderDialog();
    expect(screen.getByTestId("worker-retro-dialog")).toBeInTheDocument();
    expect(screen.getByTestId("retro-dialog-stepper")).toHaveTextContent(
      /Step 1 of 5/,
    );
    expect(screen.getByTestId("retro-dialog-metrics-summary")).toHaveTextContent(
      /12 actions/,
    );
    expect(screen.getByTestId("retro-dialog-error-summary")).toHaveTextContent(/halt/);
  });

  it("shows a loading placeholder while the retro is null", () => {
    renderDialog(null);
    expect(screen.getByTestId("retro-dialog-loading")).toBeInTheDocument();
    // No metrics, no textareas yet.
    expect(screen.queryByTestId("retro-dialog-metrics")).toBeNull();
  });

  it("advances past metrics without calling onSaveSections", async () => {
    const { onSaveSections } = renderDialog();
    await userEvent.click(screen.getByTestId("retro-dialog-next"));
    expect(screen.getByTestId("retro-dialog-stepper")).toHaveTextContent(
      /Step 2 of 5/,
    );
    expect(onSaveSections).not.toHaveBeenCalled();
  });

  it("persists draft text via onSaveSections before advancing past a text step", async () => {
    const { onSaveSections } = renderDialog();
    // Advance from metrics to step 2.
    await userEvent.click(screen.getByTestId("retro-dialog-next"));
    const textarea = screen.getByTestId("retro-dialog-section-what_went_well");
    await userEvent.type(textarea, "Recruiter loop stayed under SLA all week.");
    await userEvent.click(screen.getByTestId("retro-dialog-next"));
    expect(onSaveSections).toHaveBeenCalledTimes(1);
    expect(onSaveSections).toHaveBeenCalledWith({
      what_went_well: "Recruiter loop stayed under SLA all week.",
    });
  });

  it("disables Back on the first step and re-enables it after Next", async () => {
    renderDialog();
    expect(screen.getByTestId("retro-dialog-back")).toBeDisabled();
    await userEvent.click(screen.getByTestId("retro-dialog-next"));
    expect(screen.getByTestId("retro-dialog-back")).not.toBeDisabled();
  });

  it("renders Finalize on the last step", async () => {
    renderDialog();
    for (let i = 0; i < 4; i++) {
      await userEvent.click(screen.getByTestId("retro-dialog-next"));
    }
    expect(screen.getByTestId("retro-dialog-stepper")).toHaveTextContent(
      /Step 5 of 5/,
    );
    expect(screen.getByTestId("retro-dialog-finalize")).toBeInTheDocument();
    expect(screen.queryByTestId("retro-dialog-next")).toBeNull();
  });

  it("Finalize button calls onSaveSections then onFinalize and closes the dialog", async () => {
    const onSaveSections = vi.fn().mockResolvedValue({});
    const onFinalize = vi.fn().mockResolvedValue({});
    const { onOpenChange } = renderDialog(sampleRetro(), onSaveSections, onFinalize);
    for (let i = 0; i < 4; i++) {
      await userEvent.click(screen.getByTestId("retro-dialog-next"));
    }
    await userEvent.click(screen.getByTestId("retro-dialog-finalize"));
    expect(onSaveSections).toHaveBeenCalled();
    expect(onFinalize).toHaveBeenCalledTimes(1);
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("renders the read-only finalised summary when retro.finalized is true", () => {
    const retro = sampleRetro({
      finalized: true,
      finalizedAt: 1_700_604_999,
      sections: {
        surface_metrics: "",
        what_went_well: "Smooth handoffs.",
        surprises: "",
        what_to_change: "Tighten escalation latency.",
        action_items: "Add p95 alert.",
      },
    });
    renderDialog(retro);
    expect(screen.getByTestId("retro-dialog-finalized")).toBeInTheDocument();
    expect(
      screen.getByTestId("retro-dialog-readonly-what_went_well"),
    ).toHaveTextContent("Smooth handoffs.");
    expect(
      screen.getByTestId("retro-dialog-readonly-action_items"),
    ).toHaveTextContent("Add p95 alert.");
    // No Finalize / Next buttons once locked.
    expect(screen.queryByTestId("retro-dialog-finalize")).toBeNull();
    expect(screen.queryByTestId("retro-dialog-next")).toBeNull();
  });

  it("Close button calls onOpenChange(false) without touching mutations", async () => {
    const onSaveSections = vi.fn();
    const onFinalize = vi.fn();
    const { onOpenChange } = renderDialog(sampleRetro(), onSaveSections, onFinalize);
    await userEvent.click(screen.getByTestId("retro-dialog-close"));
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(onSaveSections).not.toHaveBeenCalled();
    expect(onFinalize).not.toHaveBeenCalled();
  });
});
