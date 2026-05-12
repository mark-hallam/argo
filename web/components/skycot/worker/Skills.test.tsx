import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  FetchWorkerSkillsResult,
  WorkerSkills,
} from "@/lib/api/worker-skills";
import type { Worker } from "@/lib/types/worker";

vi.mock("next/navigation", () => ({
  useParams: () => ({ workerId: "wkr_01J0PRIYA" }),
}));

type WorkerQueryShape = {
  data: Worker | undefined;
  isPending: boolean;
  error?: Error | null;
};
const useWorkerMock = vi.fn<() => WorkerQueryShape>();
vi.mock("@/lib/hooks/use-worker", () => ({
  useWorker: () => useWorkerMock(),
}));

type SkillsQueryShape = {
  data: FetchWorkerSkillsResult | undefined;
  isPending: boolean;
  isError: boolean;
  error: Error | null;
  refetch: () => void;
};
const useSkillsMock = vi.fn<() => SkillsQueryShape>();
const onDisableMock = vi.fn<(skillId: string) => void>();
const onTryMock = vi.fn<(skill: unknown) => void>();
const onPromoteMock = vi.fn<(id: string) => void>();
const onDiscardMock = vi.fn<(id: string) => void>();
vi.mock("@/lib/hooks/use-worker-skills", () => ({
  useWorkerSkills: () => useSkillsMock(),
  useSkillsActions: () => ({
    onDisable: onDisableMock,
    onTry: onTryMock,
    onPromote: onPromoteMock,
    onDiscard: onDiscardMock,
  }),
  workerSkillsQueryKey: () => ["worker-skills"],
}));

import WorkerSkillsPage from "@/app/(app)/workers/[workerId]/skills/page";

const WORKER: Worker = {
  id: "wkr_01J0PRIYA",
  name: "Priya",
  role: { tagline: "screens candidates", vertical: "recruiting" },
  status: "waiting",
};

function makeSkills(overrides: Partial<WorkerSkills> = {}): WorkerSkills {
  return {
    active: [
      {
        id: "sk-screen",
        name: "senior-eng-screening",
        version: "v3.0.0",
        trustScore: 94,
        usageCount7d: 128,
      },
      {
        id: "sk-slot",
        name: "interview-slot-proposer",
        version: "v2.0.0",
        trustScore: 75,
        usageCount7d: 42,
      },
    ],
    library: [
      {
        id: "sk-comp",
        name: "compensation-research",
        version: "v2.0.0",
        trustScore: 88,
        source: "workspace",
      },
      {
        id: "sk-refs",
        name: "references-checker",
        version: "v1.0.0",
        trustScore: 65,
        source: "marketplace",
      },
    ],
    learned: [
      {
        id: "lp-decline",
        label: "auto-decline-incomplete-applications",
        successCount: 8,
        failCount: 0,
        sampleOutputs: ["sample one", "sample two", "sample three"],
      },
    ],
    ...overrides,
  };
}

function skillsState(overrides: Partial<SkillsQueryShape> = {}): SkillsQueryShape {
  return {
    data: { skills: makeSkills(), source: "mock-offline" },
    isPending: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
    ...overrides,
  };
}

beforeEach(() => {
  useWorkerMock.mockReset();
  useWorkerMock.mockReturnValue({ data: WORKER, isPending: false, error: null });
  useSkillsMock.mockReset();
  onDisableMock.mockReset();
  onTryMock.mockReset();
  onPromoteMock.mockReset();
  onDiscardMock.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkerSkillsPage", () => {
  it("renders the three sub-sections in fixed order: Active → Library → Learned", () => {
    useSkillsMock.mockReturnValue(skillsState());
    render(<WorkerSkillsPage />);
    const panel = screen.getByTestId("worker-tab-panel-skills");
    const titles = within(panel)
      .getAllByRole("heading", { level: 3 })
      .map((h) => h.textContent);
    expect(titles).toEqual(["Active", "Library", "Learned"]);
  });

  it("Active section renders trust badge, usage pill, and three actions per row", () => {
    useSkillsMock.mockReturnValue(skillsState());
    render(<WorkerSkillsPage />);
    const row = screen.getByTestId("skill-row-sk-screen");
    expect(within(row).getByTestId("trust-badge")).toHaveAttribute("data-tone", "success");
    expect(within(row).getByTestId("skill-usage-sk-screen")).toHaveTextContent(
      "used 128× last 7d",
    );
    expect(within(row).getByTestId("skill-disable-sk-screen")).toBeInTheDocument();
    expect(within(row).getByTestId("skill-view-sk-screen")).toBeInTheDocument();
    expect(within(row).getByTestId("skill-audit-sk-screen")).toHaveAttribute(
      "href",
      "/audit?worker=wkr_01J0PRIYA&skill=sk-screen",
    );
  });

  it("trust badge tones map success ≥90, warning 70-90, destructive <70", () => {
    useSkillsMock.mockReturnValue(skillsState());
    render(<WorkerSkillsPage />);
    const high = screen.getByTestId("skill-row-sk-screen");
    expect(within(high).getByTestId("trust-badge")).toHaveAttribute("data-tone", "success");
    const mid = screen.getByTestId("skill-row-sk-slot");
    expect(within(mid).getByTestId("trust-badge")).toHaveAttribute("data-tone", "warning");
    const low = screen.getByTestId("skill-row-sk-refs");
    expect(within(low).getByTestId("trust-badge")).toHaveAttribute("data-tone", "destructive");
  });

  it("Disable confirm fires onDisable with the skill id after user confirms", async () => {
    const user = userEvent.setup();
    useSkillsMock.mockReturnValue(skillsState());
    render(<WorkerSkillsPage />);
    await user.click(screen.getByTestId("skill-disable-sk-screen"));
    await user.click(screen.getByTestId("alert-dialog-confirm"));
    expect(onDisableMock).toHaveBeenCalledWith("sk-screen");
  });

  it("Library 'Try it' on a high-trust skill calls onTry immediately (no confirm)", async () => {
    const user = userEvent.setup();
    useSkillsMock.mockReturnValue(skillsState());
    render(<WorkerSkillsPage />);
    await user.click(screen.getByTestId("skill-try-sk-comp"));
    expect(onTryMock).toHaveBeenCalledTimes(1);
    expect(onTryMock).toHaveBeenCalledWith(
      expect.objectContaining({ id: "sk-comp", trustScore: 88 }),
    );
  });

  it("Library 'Try it' on a low-trust skill confirms before firing onTry", async () => {
    const user = userEvent.setup();
    useSkillsMock.mockReturnValue(skillsState());
    render(<WorkerSkillsPage />);
    await user.click(screen.getByTestId("skill-try-sk-refs"));
    expect(onTryMock).not.toHaveBeenCalled();
    // The confirm dialog quotes the trust score back at the operator.
    const dialog = screen.getByTestId("alert-dialog");
    expect(within(dialog).getByText(/trust: 65%/)).toBeInTheDocument();
    await user.click(screen.getByTestId("alert-dialog-confirm"));
    expect(onTryMock).toHaveBeenCalledWith(
      expect.objectContaining({ id: "sk-refs", trustScore: 65 }),
    );
  });

  it("Promote dialog shows 3 sample outputs and fires onPromote on confirm", async () => {
    const user = userEvent.setup();
    useSkillsMock.mockReturnValue(skillsState());
    render(<WorkerSkillsPage />);
    await user.click(screen.getByTestId("learned-promote-lp-decline"));
    const preview = screen.getByTestId("promote-preview");
    expect(within(preview).getByText("auto-decline-incomplete-applications")).toBeInTheDocument();
    expect(screen.getByTestId("promote-sample-0")).toHaveTextContent("sample one");
    expect(screen.getByTestId("promote-sample-1")).toHaveTextContent("sample two");
    expect(screen.getByTestId("promote-sample-2")).toHaveTextContent("sample three");
    await user.click(screen.getByTestId("alert-dialog-confirm"));
    expect(onPromoteMock).toHaveBeenCalledWith("lp-decline");
  });

  it("Discard confirm fires onDiscard with the pattern id", async () => {
    const user = userEvent.setup();
    useSkillsMock.mockReturnValue(skillsState());
    render(<WorkerSkillsPage />);
    await user.click(screen.getByTestId("learned-discard-lp-decline"));
    await user.click(screen.getByTestId("alert-dialog-confirm"));
    expect(onDiscardMock).toHaveBeenCalledWith("lp-decline");
  });

  it("renders empty-state copy for each empty section, threading the worker's name", () => {
    useSkillsMock.mockReturnValue(
      skillsState({
        data: {
          skills: { active: [], library: [], learned: [] },
          source: "mock-offline",
        },
      }),
    );
    render(<WorkerSkillsPage />);
    expect(screen.getByTestId("skills-active-empty")).toHaveTextContent(
      "Priya doesn't have any skills assigned yet.",
    );
    expect(screen.getByTestId("skills-library-empty")).toHaveTextContent(
      /No skills available beyond the ones in use/,
    );
    expect(screen.getByTestId("skills-learned-empty")).toHaveTextContent(
      /Priya will suggest patterns here once she's used them successfully/,
    );
  });

  it("renders loading skeletons when the query is pending", () => {
    useSkillsMock.mockReturnValue(skillsState({ isPending: true, data: undefined }));
    render(<WorkerSkillsPage />);
    expect(screen.getByTestId("worker-skills-loading")).toBeInTheDocument();
    expect(screen.getByTestId("card-skills-active-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("card-skills-library-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("card-skills-learned-skeleton")).toBeInTheDocument();
  });

  it("renders an error state with Try again that refetches", async () => {
    const user = userEvent.setup();
    const refetch = vi.fn();
    useSkillsMock.mockReturnValue(
      skillsState({
        isError: true,
        data: undefined,
        error: new Error("boom"),
        refetch,
      }),
    );
    render(<WorkerSkillsPage />);
    const err = screen.getByTestId("worker-skills-error");
    expect(within(err).getByText(/boom/)).toBeInTheDocument();
    await user.click(within(err).getByRole("button", { name: /Try again/ }));
    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it("View source click announces a stub message in the live region", async () => {
    const user = userEvent.setup();
    useSkillsMock.mockReturnValue(skillsState());
    render(<WorkerSkillsPage />);
    await user.click(screen.getByTestId("skill-view-sk-screen"));
    const announcement = screen.getByTestId("skills-announcement");
    expect(announcement).toHaveTextContent(/Skill source viewer/);
    expect(screen.getByTestId("skills-toast")).toHaveTextContent(/sk-screen/);
  });
});
