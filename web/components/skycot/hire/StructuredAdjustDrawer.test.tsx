import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { StructuredAdjustDrawer } from "./StructuredAdjustDrawer";
import type { WorkerDraft } from "@/lib/hire/draft-types";

const BASE_DRAFT: WorkerDraft = {
  suggestedName: "Mariah",
  persona: {
    tagline: "Marketplace assistant",
    avatarKind: "initials",
    voiceProfile: { tone: "warm", formality: "medium", signaturePhrases: ["Cheers!"] },
  },
  charter:
    "Mariah triages your Marketplace inbox, drafts replies in your voice, and checks in twice a day.",
  duties: [
    {
      label: "Morning summary",
      schedule: { kind: "cron", spec: "0 8 * * *" },
      defaultEnabled: true,
    },
    {
      label: "Draft replies",
      schedule: { kind: "event", spec: "message.inbound" },
      defaultEnabled: true,
    },
  ],
  exampleReplies: [
    { inboundContext: "Is this still available?", mariahResponse: "Yes!" },
  ],
  approvalPolicy: { level: "balanced" },
  suggestedVertical: "marketplace",
};

interface HarnessProps {
  initialDraft?: WorkerDraft;
  onSave?: (next: WorkerDraft, rejected: string[]) => void;
}

function Harness({ initialDraft = BASE_DRAFT, onSave }: HarnessProps) {
  const [open, setOpen] = useState(true);
  const [draft, setDraft] = useState(initialDraft);
  return (
    <>
      <button type="button" data-testid="harness-open" onClick={() => setOpen(true)}>
        open
      </button>
      <StructuredAdjustDrawer
        open={open}
        onOpenChange={setOpen}
        draft={draft}
        onSave={(next, rejected) => {
          setDraft(next);
          setOpen(false);
          onSave?.(next, rejected);
        }}
      />
    </>
  );
}

describe("StructuredAdjustDrawer", () => {
  it("opens and renders all three sections", () => {
    render(<Harness />);
    expect(screen.getByTestId("hire-step4-structured-drawer")).toBeInTheDocument();
    expect(screen.getByTestId("hire-step4-structured-charter")).toBeInTheDocument();
    expect(screen.getByTestId("hire-step4-structured-duties")).toBeInTheDocument();
    expect(screen.getByTestId("hire-step4-voice-editor")).toBeInTheDocument();
  });

  it("closes via Cancel without firing onSave", async () => {
    const onSave = vi.fn();
    render(<Harness onSave={onSave} />);
    await userEvent.click(screen.getByTestId("hire-step4-structured-cancel"));
    expect(onSave).not.toHaveBeenCalled();
    expect(screen.queryByTestId("hire-step4-structured-drawer")).not.toBeInTheDocument();
  });

  it("propagates a charter edit to onSave", async () => {
    const onSave = vi.fn();
    render(<Harness onSave={onSave} />);
    const charter = screen.getByTestId("hire-step4-structured-charter-input");
    fireEvent.change(charter, {
      target: {
        value: "Mariah handles the inbox carefully and triples the response speed.",
      },
    });
    await userEvent.click(screen.getByTestId("hire-step4-structured-save"));
    expect(onSave).toHaveBeenCalledTimes(1);
    const [next] = onSave.mock.calls[0]!;
    expect((next as WorkerDraft).charter).toMatch(/Mariah handles the inbox carefully/);
  });

  it("disables Save when charter is too short", () => {
    const tiny: WorkerDraft = { ...BASE_DRAFT, charter: "short" };
    render(<Harness initialDraft={tiny} />);
    expect(screen.getByTestId("hire-step4-structured-save")).toBeDisabled();
  });

  it("adds and removes duty rows", async () => {
    const onSave = vi.fn();
    render(<Harness onSave={onSave} />);
    expect(screen.getAllByTestId(/^hire-step4-duty-editor-\d+$/)).toHaveLength(2);
    await userEvent.click(screen.getByTestId("hire-step4-structured-add-duty"));
    expect(screen.getAllByTestId(/^hire-step4-duty-editor-\d+$/)).toHaveLength(3);
    await userEvent.click(screen.getByTestId("hire-step4-duty-editor-0-remove"));
    expect(screen.getAllByTestId(/^hire-step4-duty-editor-\d+$/)).toHaveLength(2);
  });

  it("disables Save when every duty is toggled off", async () => {
    render(<Harness />);
    await userEvent.click(screen.getByTestId("hire-step4-duty-editor-0-toggle"));
    await userEvent.click(screen.getByTestId("hire-step4-duty-editor-1-toggle"));
    expect(screen.getByTestId("hire-step4-structured-save")).toBeDisabled();
    expect(screen.getByTestId("hire-step4-structured-no-duty-error")).toBeInTheDocument();
  });

  it("flags an invalid cron expression inline and disables Save", () => {
    render(<Harness />);
    const cronInput = screen.getByTestId("hire-step4-duty-editor-0-spec");
    fireEvent.change(cronInput, { target: { value: "99 * * * *" } });
    expect(screen.getByTestId("hire-step4-duty-editor-0-error")).toBeInTheDocument();
    expect(screen.getByTestId("hire-step4-structured-save")).toBeDisabled();
  });

  it("toggles the tone selection", async () => {
    const onSave = vi.fn();
    render(<Harness onSave={onSave} />);
    await userEvent.click(screen.getByTestId("hire-step4-voice-tone-formal"));
    await userEvent.click(screen.getByTestId("hire-step4-structured-save"));
    const [next] = onSave.mock.calls[0]!;
    expect((next as WorkerDraft).persona.voiceProfile.tone).toBe("formal");
  });

  it("filters profanity from signature phrases and reports them", async () => {
    const onSave = vi.fn();
    render(<Harness onSave={onSave} />);
    const phrases = screen.getByTestId("hire-step4-voice-phrases");
    fireEvent.change(phrases, {
      target: { value: "Cheers!\nfuck off\nThanks a lot" },
    });
    await userEvent.click(screen.getByTestId("hire-step4-structured-save"));
    expect(onSave).toHaveBeenCalledTimes(1);
    const [next, rejected] = onSave.mock.calls[0]!;
    expect((next as WorkerDraft).persona.voiceProfile.signaturePhrases).toEqual([
      "Cheers!",
      "Thanks a lot",
    ]);
    expect(rejected).toEqual(["fuck off"]);
  });

  it("drops too-short signature lines on save", async () => {
    const onSave = vi.fn();
    render(<Harness onSave={onSave} />);
    fireEvent.change(screen.getByTestId("hire-step4-voice-phrases"), {
      target: { value: "Cheers!\nok\nThanks heaps" },
    });
    await userEvent.click(screen.getByTestId("hire-step4-structured-save"));
    const [next, rejected] = onSave.mock.calls[0]!;
    expect((next as WorkerDraft).persona.voiceProfile.signaturePhrases).toEqual([
      "Cheers!",
      "Thanks heaps",
    ]);
    expect(rejected).toContain("ok");
  });

  it("save commits all fields and clears cached example replies", async () => {
    const onSave = vi.fn();
    render(<Harness onSave={onSave} />);
    fireEvent.change(screen.getByTestId("hire-step4-structured-charter-input"), {
      target: {
        value: "A precise, formal teammate focused on inbox triage and replies.",
      },
    });
    await userEvent.click(screen.getByTestId("hire-step4-voice-tone-formal"));
    await userEvent.click(screen.getByTestId("hire-step4-structured-save"));
    const [next] = onSave.mock.calls[0]!;
    const d = next as WorkerDraft;
    expect(d.charter).toMatch(/precise, formal teammate/);
    expect(d.persona.voiceProfile.tone).toBe("formal");
    expect(d.exampleReplies).toEqual([]);
  });

  it("applies a cron preset chip", async () => {
    const onSave = vi.fn();
    render(<Harness onSave={onSave} />);
    await userEvent.click(screen.getByTestId("hire-step4-duty-editor-0-presets"));
    await userEvent.click(screen.getByTestId("hire-step4-duty-editor-0-preset-weekdays-9am"));
    await userEvent.click(screen.getByTestId("hire-step4-structured-save"));
    const [next] = onSave.mock.calls[0]!;
    const d = next as WorkerDraft;
    expect(d.duties[0]!.schedule.spec).toBe("0 9 * * 1-5");
  });
});
