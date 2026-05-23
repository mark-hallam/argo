/**
 * Integration: RBAC-gated UI behavior.
 *
 * - guest: textarea read-only, Save hidden, Delete hidden, +New hidden.
 * - admin: editor + Save visible, Delete hidden.
 * - owner: editor + Save visible, Delete visible.
 *
 * Plus: 404 from the gateway in strict-production mode surfaces as an
 * error banner rather than as sample-data fallback.
 */

import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

const hoisted = vi.hoisted(() => ({
  mockRole: { current: "guest" },
  indexListMock: vi.fn(),
}));

vi.mock("@/lib/ui/use-toast", () => ({
  useToast: () => ({
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  }),
}));

vi.mock("next-auth/react", () => ({
  useSession: () => ({ data: { role: hoisted.mockRole.current } }),
}));

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), back: vi.fn(), prefetch: vi.fn(), refresh: vi.fn() }),
  useParams: () => ({ slug: "brand-voice" }),
  useSearchParams: () => new URLSearchParams(),
  usePathname: () => "/settings/knowledge/brand-voice",
}));

import type {
  KnowledgeDoc,
  KnowledgeRevision,
} from "@/lib/api/workplace-knowledge";

const doc: KnowledgeDoc = {
  id: "doc_brand",
  tenant_id: "demo",
  slug: "brand-voice",
  title: "Brand voice",
  category: "brand",
  body_markdown: "# Brand voice\n\nbe warm, be clear.",
  version: 1,
  parent_revision_id: null,
  created_at_micros: 1,
  created_by: "sam@acmeco.com",
  updated_at_micros: Date.now() * 1000,
  updated_by: "sam@acmeco.com",
};

const revisions: KnowledgeRevision[] = [
  {
    id: "rev_v1",
    doc_id: doc.id,
    tenant_id: doc.tenant_id,
    version: 1,
    body_markdown: doc.body_markdown,
    diff_from_prior: null,
    edited_at_micros: doc.updated_at_micros,
    edited_by: doc.updated_by,
    edit_reason: null,
  },
];

vi.mock("@/lib/api/workplace-knowledge", async () => {
  const mod = await vi.importActual<
    typeof import("@/lib/api/workplace-knowledge")
  >("@/lib/api/workplace-knowledge");
  hoisted.indexListMock.mockImplementation(async () => ({ docs: [doc], source: "gateway" as const }));
  return {
    ...mod,
    listKnowledgeDocs: hoisted.indexListMock,
    getKnowledgeDoc: vi.fn(async () => doc),
    listKnowledgeRevisions: vi.fn(async () => ({ revisions, source: "gateway" as const })),
    createKnowledgeDoc: vi.fn(async (input) => ({ ...doc, ...input, version: 1 })),
    updateKnowledgeDoc: vi.fn(),
    deleteKnowledgeDoc: vi.fn(),
  };
});

import DetailPage from "@/app/(app)/settings/knowledge/[slug]/page";
import IndexPage from "@/app/(app)/settings/knowledge/page";

function Wrap({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: 0 },
      mutations: { retry: false },
    },
  });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

beforeEach(() => {
  hoisted.indexListMock.mockClear();
  hoisted.indexListMock.mockImplementation(async () => ({ docs: [doc], source: "gateway" as const }));
});

describe("rbac gates", () => {
  it("guest sees a read-only view with no Save / Delete / +New", async () => {
    hoisted.mockRole.current = "guest";
    const { rerender } = render(
      <Wrap>
        <DetailPage />
      </Wrap>,
    );
    await waitFor(() => {
      expect(screen.getByTestId("knowledge-detail-version")).toHaveTextContent("v1");
    });
    const textarea = screen.getByTestId("knowledge-editor-textarea") as HTMLTextAreaElement;
    expect(textarea).toHaveAttribute("readonly");
    expect(screen.queryByTestId("knowledge-editor-save")).not.toBeInTheDocument();
    expect(screen.queryByTestId("knowledge-delete-btn")).not.toBeInTheDocument();
    // Editor tab labels as "View" rather than "Edit" in read-only mode.
    expect(screen.getByTestId("knowledge-editor-tab-edit")).toHaveTextContent("View");

    rerender(
      <Wrap>
        <IndexPage />
      </Wrap>,
    );
    await waitFor(() => {
      expect(screen.getByTestId("knowledge-card-brand-voice")).toBeInTheDocument();
    });
    expect(screen.queryByTestId("knowledge-new-doc")).not.toBeInTheDocument();
  });

  it("admin can edit + save but not delete", async () => {
    hoisted.mockRole.current = "admin";
    render(
      <Wrap>
        <DetailPage />
      </Wrap>,
    );
    await waitFor(() => {
      expect(screen.getByTestId("knowledge-detail-version")).toHaveTextContent("v1");
    });
    expect(screen.getByTestId("knowledge-editor-save")).toBeInTheDocument();
    expect(screen.queryByTestId("knowledge-delete-btn")).not.toBeInTheDocument();
    const textarea = screen.getByTestId("knowledge-editor-textarea") as HTMLTextAreaElement;
    expect(textarea).not.toHaveAttribute("readonly");
  });

  it("owner sees editor + save + delete button", async () => {
    hoisted.mockRole.current = "owner";
    render(
      <Wrap>
        <DetailPage />
      </Wrap>,
    );
    await waitFor(() => {
      expect(screen.getByTestId("knowledge-detail-version")).toHaveTextContent("v1");
    });
    expect(screen.getByTestId("knowledge-editor-save")).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByTestId("knowledge-delete-btn")).toBeInTheDocument();
    });
  });

  it("owner sees the +New doc CTA on the index", async () => {
    hoisted.mockRole.current = "owner";
    render(
      <Wrap>
        <IndexPage />
      </Wrap>,
    );
    await waitFor(() => {
      expect(screen.getByTestId("knowledge-new-doc")).toBeInTheDocument();
    });
  });
});

describe("strict-production: no mock fallback", () => {
  it("a 404 from the gateway surfaces as an error banner (no sample-data fallback)", async () => {
    // In strict-production mode, `lib/api/workplace-knowledge.ts`
    // re-throws the ApiError instead of falling back to
    // `SAMPLE_KNOWLEDGE`. The integration assertion is: that re-throw
    // surfaces as a visible error banner on the index page.
    hoisted.indexListMock.mockImplementationOnce(async () => {
      const { ApiError } = await import("@/lib/api/types");
      throw new ApiError(404, "not found", "not_found");
    });

    hoisted.mockRole.current = "guest";
    render(
      <Wrap>
        <IndexPage />
      </Wrap>,
    );

    await waitFor(() => {
      expect(
        screen.getByText(/Couldn't load workplace knowledge/),
      ).toBeInTheDocument();
    });
    expect(screen.queryByTestId("knowledge-card-brand-voice")).not.toBeInTheDocument();
  });
});
