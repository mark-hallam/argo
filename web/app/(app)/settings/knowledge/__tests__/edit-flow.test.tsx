/**
 * Integration: load a doc → edit body → click Save → optimistic
 * version bump + revisions list grows by one + Saved toast fires.
 *
 * Uses the real react-query client (`QueryClientProvider`) wrapped
 * around the detail page, with `lib/api/workplace-knowledge` mocked
 * to a controllable in-memory store.
 */

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const successToast = vi.fn();
const errorToast = vi.fn();
const pushMock = vi.fn();

vi.mock("@/lib/ui/use-toast", () => ({
  useToast: () => ({
    success: successToast,
    error: errorToast,
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  }),
}));

vi.mock("next-auth/react", () => ({
  useSession: () => ({ data: { role: "owner" } }),
}));

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, replace: vi.fn(), back: vi.fn(), prefetch: vi.fn(), refresh: vi.fn() }),
  useParams: () => ({ slug: "brand-voice" }),
  useSearchParams: () => new URLSearchParams(),
  usePathname: () => "/settings/knowledge/brand-voice",
}));

import type {
  KnowledgeDoc,
  KnowledgeRevision,
} from "@/lib/api/workplace-knowledge";

interface FakeStore {
  doc: KnowledgeDoc;
  revisions: KnowledgeRevision[];
}

const store: FakeStore = (() => {
  const doc: KnowledgeDoc = {
    id: "doc_brand",
    tenant_id: "demo",
    slug: "brand-voice",
    title: "Brand voice",
    category: "brand",
    body_markdown: "# Brand voice\n\nbe warm, be clear.",
    version: 2,
    parent_revision_id: null,
    created_at_micros: 1_000_000_000,
    created_by: "sam@acmeco.com",
    updated_at_micros: Date.now() * 1000,
    updated_by: "sam@acmeco.com",
  };
  const revisions: KnowledgeRevision[] = [
    {
      id: "rev_v2",
      doc_id: doc.id,
      tenant_id: doc.tenant_id,
      version: 2,
      body_markdown: doc.body_markdown,
      diff_from_prior: "- old\n+ new",
      edited_at_micros: doc.updated_at_micros,
      edited_by: doc.updated_by,
      edit_reason: "Tighten",
    },
    {
      id: "rev_v1",
      doc_id: doc.id,
      tenant_id: doc.tenant_id,
      version: 1,
      body_markdown: "# Initial",
      diff_from_prior: null,
      edited_at_micros: 1_000_000_000,
      edited_by: doc.created_by,
      edit_reason: null,
    },
  ];
  return { doc, revisions };
})();

vi.mock("@/lib/api/workplace-knowledge", async () => {
  const mod = await vi.importActual<
    typeof import("@/lib/api/workplace-knowledge")
  >("@/lib/api/workplace-knowledge");
  return {
    ...mod,
    listKnowledgeDocs: vi.fn(async () => ({ docs: [store.doc], source: "gateway" as const })),
    getKnowledgeDoc: vi.fn(async () => ({ ...store.doc })),
    listKnowledgeRevisions: vi.fn(async () => ({
      revisions: [...store.revisions],
      source: "gateway" as const,
    })),
    updateKnowledgeDoc: vi.fn(async (_slug: string, input: { body_markdown: string; edit_reason?: string }) => {
      store.doc = {
        ...store.doc,
        body_markdown: input.body_markdown,
        version: store.doc.version + 1,
        updated_at_micros: Date.now() * 1000,
      };
      store.revisions = [
        {
          id: `rev_v${store.doc.version}`,
          doc_id: store.doc.id,
          tenant_id: store.doc.tenant_id,
          version: store.doc.version,
          body_markdown: input.body_markdown,
          diff_from_prior: "- prev\n+ next",
          edited_at_micros: store.doc.updated_at_micros,
          edited_by: store.doc.updated_by,
          edit_reason: input.edit_reason ?? null,
        },
        ...store.revisions,
      ];
      return store.doc;
    }),
    deleteKnowledgeDoc: vi.fn(async (slug: string) => ({ ok: true as const, slug })),
    createKnowledgeDoc: vi.fn(),
  };
});

import DetailPage from "@/app/(app)/settings/knowledge/[slug]/page";

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
  successToast.mockReset();
  errorToast.mockReset();
  pushMock.mockReset();
});

afterEach(() => {
  store.doc = {
    ...store.doc,
    body_markdown: "# Brand voice\n\nbe warm, be clear.",
    version: 2,
  };
  store.revisions = store.revisions.filter((r) => r.version <= 2);
});

describe("edit flow", () => {
  it("loads the doc, edits, saves, and fires the Saved toast with the new version", async () => {
    const user = userEvent.setup();
    render(
      <Wrap>
        <DetailPage />
      </Wrap>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("knowledge-detail-version")).toHaveTextContent("v2");
    });

    const textarea = screen.getByTestId("knowledge-editor-textarea") as HTMLTextAreaElement;
    await waitFor(() => {
      expect(textarea.value).toContain("be warm");
    });

    await user.clear(textarea);
    await user.type(textarea, "# Brand voice\n\nupdated body");
    await user.type(screen.getByTestId("knowledge-editor-edit-reason"), "Tighten language");
    await user.click(screen.getByTestId("knowledge-editor-save"));

    await waitFor(() => {
      expect(successToast).toHaveBeenCalled();
    });
    expect(successToast.mock.calls[0][0]).toBe("Saved");
    expect(successToast.mock.calls[0][1].description).toBe("Now at v3.");

    await waitFor(() => {
      expect(screen.getByTestId("knowledge-detail-version")).toHaveTextContent("v3");
    });

    await waitFor(() => {
      expect(screen.getByTestId("knowledge-version-3")).toBeInTheDocument();
    });
  });
});
