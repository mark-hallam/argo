"use client";

import { useMemo, useState } from "react";
import { Plus } from "lucide-react";
import { KnowledgeStyles } from "@/components/skycot/knowledge/styles";
import { KnowledgeCard } from "@/components/skycot/knowledge/KnowledgeCard";
import { CategoryNav } from "@/components/skycot/knowledge/CategoryNav";
import { CreateDocModal } from "@/components/skycot/knowledge/CreateDocModal";
import {
  useCreateKnowledgeDoc,
  useKnowledgeDocs,
  useKnowledgeViewerPolicy,
} from "@/lib/hooks/use-workplace-knowledge";
import { useToast } from "@/lib/ui/use-toast";
import type { KnowledgeCategory } from "@/lib/api/workplace-knowledge";

/**
 * `/settings/knowledge` — workplace wiki index. Grid of docs grouped
 * by category, with the left rail filtering. Owner / admin sees the
 * "New doc" pill; everyone else sees the read-only browse view.
 *
 * Brand-restrained register with one Audiowide accent on the hero
 * title (matches the locked /settings page memory).
 */
export default function WorkplaceKnowledgeIndexPage() {
  const { canWrite } = useKnowledgeViewerPolicy();
  const [active, setActive] = useState<KnowledgeCategory | "all">("all");
  const [showCreate, setShowCreate] = useState(false);
  const { success, error: toastError } = useToast();
  const query = useKnowledgeDocs();
  const create = useCreateKnowledgeDoc();

  const docs = query.data?.docs ?? [];
  const visible = useMemo(
    () => (active === "all" ? docs : docs.filter((d) => d.category === active)),
    [docs, active],
  );

  return (
    <>
      <KnowledgeStyles />
      <div className="knowledge-stage" data-testid="knowledge-index">
        <div className="knowledge-header">
          <div>
            <div className="knowledge-eyebrow">⚙ Workspace settings · Knowledge</div>
            <h1 className="knowledge-title">Workplace Knowledge</h1>
            <p className="knowledge-sub">What every worker reads, every turn.</p>
          </div>
          {canWrite ? (
            <button
              type="button"
              className="knowledge-cta"
              onClick={() => setShowCreate(true)}
              data-testid="knowledge-new-doc"
            >
              <Plus size={14} /> New doc
            </button>
          ) : null}
        </div>

        {query.error ? (
          <div className="knowledge-error-banner" role="alert">
            Couldn&apos;t load workplace knowledge: {String(query.error.message ?? query.error)}
          </div>
        ) : null}

        <div className="knowledge-layout">
          <CategoryNav docs={docs} active={active} onSelect={setActive} />

          {query.isLoading ? (
            <div className="knowledge-empty">Loading docs…</div>
          ) : visible.length === 0 ? (
            <div className="knowledge-empty" data-testid="knowledge-empty">
              {active === "all"
                ? "No workplace docs yet. Create the first one to give every worker shared context."
                : `No docs in this category yet.`}
            </div>
          ) : (
            <div className="knowledge-grid">
              {visible.map((doc) => (
                <KnowledgeCard key={doc.id} doc={doc} />
              ))}
            </div>
          )}
        </div>
      </div>

      {showCreate ? (
        <CreateDocModal
          isSubmitting={create.isPending}
          submitError={create.error?.message ?? null}
          onCancel={() => setShowCreate(false)}
          onSubmit={(req) =>
            create.mutate(req, {
              onSuccess: (doc) => {
                setShowCreate(false);
                success(`${doc.title} created`, { description: doc.slug });
              },
              onError: (err) => {
                toastError("Couldn't create doc", { description: err.message });
              },
            })
          }
        />
      ) : null}
    </>
  );
}
