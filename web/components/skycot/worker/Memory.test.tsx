import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MemoryFactsCard } from "@/components/skycot/worker/MemoryFactsCard";
import { ActiveConversationsCard } from "@/components/skycot/worker/ActiveConversationsCard";
import { RecentDecisionsCard } from "@/components/skycot/worker/RecentDecisionsCard";
import { VoiceProfileSummary } from "@/components/skycot/worker/VoiceProfileSummary";
import { AddFactDialog } from "@/components/skycot/worker/AddFactDialog";
import type {
  ActiveConversation, FetchWorkerMemoryResult, RecentDecision,
  VoiceProfile, WorkerFact, WorkerMemory,
} from "@/lib/api/worker-memory";
import { DEFAULT_DELEGATOR_PERMISSIONS, type Worker } from "@/lib/types/worker";

vi.mock("next/navigation", () => ({
  useParams: () => ({ workerId: "wkr_priya" }),
  useRouter: () => ({
    replace: vi.fn(), push: vi.fn(), prefetch: vi.fn(),
    refresh: vi.fn(), back: vi.fn(), forward: vi.fn(),
  }),
  usePathname: () => "/workers/wkr_priya/memory",
  useSearchParams: () => new URLSearchParams(),
}));
vi.mock("@/lib/api/worker-memory", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/worker-memory")>(
    "@/lib/api/worker-memory",
  );
  return {
    ...actual,
    fetchMemory: vi.fn(), addFact: vi.fn(), editFact: vi.fn(), deleteFact: vi.fn(),
  };
});
vi.mock("@/lib/api/workers", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/workers")>(
    "@/lib/api/workers",
  );
  return { ...actual, fetchWorker: vi.fn() };
});
vi.mock("@/lib/hooks/use-redaction-policy", async () => {
  const { POLICY_SHOW_ALL } = await import("@/lib/redaction/redact");
  return { useRedactionPolicy: () => POLICY_SHOW_ALL };
});

import { fetchWorker } from "@/lib/api/workers";
import {
  addFact, deleteFact, fetchMemory, WorkerMemoryNotFoundError,
} from "@/lib/api/worker-memory";
import WorkerMemoryPage from "@/app/(app)/workers/[workerId]/memory/page";

const mockedFetchMemory = vi.mocked(fetchMemory);
const mockedFetchWorker = vi.mocked(fetchWorker);
const mockedAddFact = vi.mocked(addFact);
const mockedDeleteFact = vi.mocked(deleteFact);

function fact(id: string, text: string): WorkerFact {
  return { factId: id, text, createdAt: new Date().toISOString() };
}

function sampleMemory(): WorkerMemory {
  const conversations: ActiveConversation[] = [{
    conversationId: "c1", counterpartyHandle: "Diego Alvarez",
    topic: "Interview scheduling — Thu 10:30",
    lastMessageAt: new Date(Date.now() - 18 * 60_000).toISOString(),
    href: "/audit/action/c1",
  }];
  const decisions: RecentDecision[] = [
    { decisionId: "d1", label: "Approved interview slot for candidate #4821",
      outcome: "approved",
      decidedAt: new Date(Date.now() - 45 * 60_000).toISOString(),
      href: "/audit/action/d1" },
    { decisionId: "d2", label: "Refused to forward candidate email outside org",
      outcome: "refused",
      decidedAt: new Date(Date.now() - 6 * 60 * 60_000).toISOString(),
      href: "/audit/action/d2" },
  ];
  const voiceProfile: VoiceProfile = {
    tone: "warm", formality: 35,
    signaturePhrases: ["Thanks for getting back to me", "Looking forward to chatting"],
  };
  return {
    namespaceId: "ns_priya_4f7a",
    tenantScope: "Tenant-scoped ✓",
    facts: [
      fact("f1", "Pre-screen all senior engineering candidates against JD-2024-08."),
      fact("f2", "Use 'guests' not 'candidates' externally per employer brand voice."),
      fact("f3", "Never agree to comp above the posted range without VP sign-off."),
    ],
    activeConversations: conversations,
    recentDecisions: decisions,
    voiceProfile,
  };
}

function sampleWorker(overrides: Partial<Worker> = {}): Worker {
  return {
    id: "wkr_priya", name: "Priya",
    role: { tagline: "Senior Recruiter (AI)", vertical: "recruiter" },
    status: "working",
    permissions: DEFAULT_DELEGATOR_PERMISSIONS,
    ...overrides,
  };
}

function PageWrapper() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } });
  return (
    <QueryClientProvider client={client}>
      <WorkerMemoryPage />
    </QueryClientProvider>
  );
}

function factsCardProps(overrides: Partial<React.ComponentProps<typeof MemoryFactsCard>> = {}) {
  return {
    workerName: "Priya", facts: sampleMemory().facts, canEdit: true,
    onAdd: vi.fn(), onEdit: vi.fn(), onDelete: vi.fn(), onRequestAdd: vi.fn(),
    ...overrides,
  };
}

beforeEach(() => {
  mockedFetchWorker.mockResolvedValue(sampleWorker());
  mockedFetchMemory.mockResolvedValue({
    memory: sampleMemory(), source: "mock-offline",
  } satisfies FetchWorkerMemoryResult);
  mockedAddFact.mockImplementation(async (_workerId, input) => ({
    factId: "f_new", text: input.text, createdAt: new Date().toISOString(),
  }));
  mockedDeleteFact.mockResolvedValue(undefined);
});
afterEach(() => { vi.clearAllMocks(); });

describe("WorkerMemoryPage — four cards render in order", () => {
  it("renders Facts, Active conversations, Recent decisions, Voice profile", async () => {
    render(<PageWrapper />);
    expect(await screen.findByTestId("card-memory-facts")).toBeInTheDocument();
    expect(screen.getByTestId("card-active-conversations")).toBeInTheDocument();
    expect(screen.getByTestId("card-recent-decisions")).toBeInTheDocument();
    expect(screen.getByTestId("card-voice-profile")).toBeInTheDocument();
  });

  it("shows the loading skeleton stack while memory is pending", async () => {
    mockedFetchMemory.mockImplementation(() => new Promise(() => {}));
    render(<PageWrapper />);
    expect(await screen.findByTestId("worker-memory-loading")).toBeInTheDocument();
    expect(screen.getByTestId("card-memory-facts-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("card-active-conversations-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("card-recent-decisions-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("card-voice-profile-skeleton")).toBeInTheDocument();
  });

  it("renders an error state with retry when memory fetch fails", async () => {
    mockedFetchMemory.mockRejectedValue(new WorkerMemoryNotFoundError("wkr_priya"));
    render(<PageWrapper />);
    const err = await screen.findByTestId("worker-memory-error");
    expect(err).toHaveTextContent(/not found/i);
    expect(within(err).getByRole("button", { name: /try again/i })).toBeInTheDocument();
  });

  it("hides the namespace badge for non-settings viewers", async () => {
    mockedFetchWorker.mockResolvedValue(sampleWorker({
      permissions: { ...DEFAULT_DELEGATOR_PERMISSIONS, canViewSettings: false },
    }));
    render(<PageWrapper />);
    await screen.findByTestId("card-memory-facts");
    expect(screen.queryByTestId("memory-namespace-badge")).toBeNull();
  });

  it("shows the namespace badge to operators with canViewSettings", async () => {
    mockedFetchWorker.mockResolvedValue(sampleWorker({
      permissions: { ...DEFAULT_DELEGATOR_PERMISSIONS, canViewSettings: true },
    }));
    render(<PageWrapper />);
    const badge = await screen.findByTestId("memory-namespace-badge");
    expect(badge).toHaveTextContent(/ns_priya_4f7a/);
    expect(badge).toHaveTextContent(/Tenant-scoped/);
  });
});

describe("MemoryFactsCard — search, edit, delete, add", () => {
  it("filters facts case-insensitively as the user types", async () => {
    const user = userEvent.setup();
    render(<MemoryFactsCard {...factsCardProps()} />);
    await user.type(screen.getByTestId("facts-search"), "GUESTS");
    expect(screen.getByTestId("fact-row-f2")).toBeInTheDocument();
    expect(screen.queryByTestId("fact-row-f1")).toBeNull();
    expect(screen.queryByTestId("fact-row-f3")).toBeNull();
  });

  it("renders the empty state when the worker has no facts", () => {
    render(<MemoryFactsCard {...factsCardProps({ facts: [] })} />);
    const empty = screen.getByTestId("facts-empty");
    expect(empty).toHaveTextContent(/Priya is still learning about your business/);
    expect(empty).toHaveTextContent(/Add what she should know/);
  });

  it("commits an inline edit on Enter and calls onEdit with trimmed text", async () => {
    const user = userEvent.setup();
    const onEdit = vi.fn();
    render(<MemoryFactsCard {...factsCardProps({ facts: [fact("f1", "Original fact")], onEdit })} />);
    await user.click(screen.getByTestId("fact-edit-f1"));
    const editor = await screen.findByTestId("fact-editor-f1");
    await user.clear(editor);
    await user.type(editor, "  Updated fact text  ");
    await user.keyboard("{Enter}");
    expect(onEdit).toHaveBeenCalledWith("f1", "Updated fact text");
  });

  it("opens a confirm dialog and calls onDelete after Delete is clicked", async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn();
    render(<MemoryFactsCard {...factsCardProps({ facts: [fact("f1", "About to be deleted")], onDelete })} />);
    await user.click(screen.getByTestId("fact-delete-f1"));
    expect(await screen.findByTestId("alert-dialog")).toBeInTheDocument();
    await user.click(screen.getByTestId("alert-dialog-confirm"));
    expect(onDelete).toHaveBeenCalledWith("f1");
  });

  it("fires onRequestAdd when '+ Add a fact' is clicked", async () => {
    const user = userEvent.setup();
    const onRequestAdd = vi.fn();
    render(<MemoryFactsCard {...factsCardProps({ facts: [], onRequestAdd })} />);
    await user.click(screen.getByTestId("facts-add"));
    expect(onRequestAdd).toHaveBeenCalledOnce();
  });

  it("hides edit/delete controls when canEdit is false", () => {
    render(<MemoryFactsCard {...factsCardProps({ facts: [fact("f1", "Read-only fact")], canEdit: false })} />);
    expect(screen.queryByTestId("fact-edit-f1")).toBeNull();
    expect(screen.queryByTestId("fact-delete-f1")).toBeNull();
    expect(screen.queryByTestId("facts-add")).toBeNull();
  });
});

describe("ActiveConversationsCard + RecentDecisionsCard + VoiceProfileSummary", () => {
  it("renders the conversations empty state when none are open", () => {
    render(<ActiveConversationsCard conversations={[]} />);
    expect(screen.getByTestId("conversations-empty"))
      .toHaveTextContent(/No open conversations right now/i);
  });

  it("links each conversation row to the audit drill-in", () => {
    render(<ActiveConversationsCard conversations={sampleMemory().activeConversations} />);
    const row = screen.getByTestId("conversation-row-c1");
    expect(row.querySelector("a")?.getAttribute("href")).toBe("/audit/action/c1");
  });

  it("renders decisions with approved/refused outcome chips", () => {
    render(<RecentDecisionsCard decisions={sampleMemory().recentDecisions} />);
    expect(screen.getByTestId("decision-outcome-d1")).toHaveTextContent(/approved/);
    expect(screen.getByTestId("decision-outcome-d2")).toHaveTextContent(/refused/);
  });

  it("renders the decisions empty state when none in the window", () => {
    render(<RecentDecisionsCard decisions={[]} />);
    expect(screen.getByTestId("decisions-empty"))
      .toHaveTextContent(/No material decisions in the last 14 days/i);
  });

  it("renders voice tone + formality + signature phrases", async () => {
    const user = userEvent.setup();
    const onAdjust = vi.fn();
    render(<VoiceProfileSummary voice={sampleMemory().voiceProfile} onAdjust={onAdjust} />);
    expect(screen.getByTestId("voice-tone")).toHaveTextContent("Warm");
    expect(screen.getByTestId("voice-formality")).toHaveTextContent("35");
    expect(within(screen.getByTestId("voice-phrases")).getAllByRole("listitem").length).toBe(2);
    await user.click(screen.getByTestId("voice-adjust"));
    expect(onAdjust).toHaveBeenCalledOnce();
  });
});

describe("AddFactDialog", () => {
  it("disables Save until the textarea has non-whitespace input", async () => {
    const user = userEvent.setup();
    render(<AddFactDialog open onOpenChange={vi.fn()} workerName="Priya" onSave={vi.fn().mockResolvedValue(undefined)} />);
    const save = await screen.findByTestId("add-fact-save");
    expect(save).toBeDisabled();
    await user.type(screen.getByTestId("add-fact-textarea"), "Hello");
    expect(save).not.toBeDisabled();
  });

  it("calls onSave with trimmed text and closes the dialog on success", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn().mockResolvedValue(undefined);
    const onOpenChange = vi.fn();
    render(<AddFactDialog open onOpenChange={onOpenChange} workerName="Priya" onSave={onSave} />);
    await user.type(screen.getByTestId("add-fact-textarea"), "  New policy fact  ");
    await user.click(screen.getByTestId("add-fact-save"));
    await waitFor(() => expect(onSave).toHaveBeenCalledWith("New policy fact"));
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
  });
});

describe("Memory page wiring", () => {
  it("opens the add-fact dialog and persists the new fact via addFact", async () => {
    const user = userEvent.setup();
    render(<PageWrapper />);
    await screen.findByTestId("card-memory-facts");
    await user.click(screen.getByTestId("facts-add"));
    const textarea = await screen.findByTestId("add-fact-textarea");
    await user.type(textarea, "Honor Q4 freeze window for invoice batches.");
    await user.click(screen.getByTestId("add-fact-save"));
    await waitFor(() => expect(mockedAddFact).toHaveBeenCalledWith("wkr_priya", {
      text: "Honor Q4 freeze window for invoice batches.",
    }));
  });

  it("opens a stub toast pointing at E4-g when Adjust voice is clicked", async () => {
    const user = userEvent.setup();
    render(<PageWrapper />);
    await user.click(await screen.findByTestId("voice-adjust"));
    await waitFor(() =>
      expect(screen.getByTestId("memory-toast")).toHaveTextContent(/E4-g/));
  });

  it("calls deleteFact when a row is confirmed for deletion", async () => {
    const user = userEvent.setup();
    render(<PageWrapper />);
    await screen.findByTestId("card-memory-facts");
    await user.click(screen.getByTestId("fact-delete-f1"));
    await user.click(await screen.findByTestId("alert-dialog-confirm"));
    await waitFor(() => expect(mockedDeleteFact).toHaveBeenCalledWith("wkr_priya", "f1"));
  });
});
