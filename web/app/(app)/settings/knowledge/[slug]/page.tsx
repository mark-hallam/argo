"use client";

import { useEffect, useState } from "react";
import { useRouter, useParams } from "next/navigation";
import Link from "next/link";
import { ArrowLeft, Trash2 } from "lucide-react";
import { KnowledgeStyles } from "@/components/skycot/knowledge/styles";
import { KnowledgeEditor } from "@/components/skycot/knowledge/KnowledgeEditor";
import { VersionHistorySidebar } from "@/components/skycot/knowledge/VersionHistorySidebar";
import { categoryLabel } from "@/components/skycot/knowledge/category-meta";
import {
  useDeleteKnowledgeDoc,
  useKnowledgeDoc,
  useKnowledgeRevisions,
  useKnowledgeViewerPolicy,
  useUpdateKnowledgeDoc,
} from "@/lib/hooks/use-workplace-knowledge";
import { useToast } from "@/lib/ui/use-toast";

/**
 * `/settings/knowledge/<slug>` — single-doc editor + version history.
 *
 * RBAC:
 * - owner / admin: full edit, version chip increments on Save
 * - owner only: Delete doc button visible
 * - user / guest: read-only view, Save / Delete hidden
 */
export default function WorkplaceKnowledgeDetailPage() {
  const params = useParams();
  const router = useRouter();
  const slug = typeof params.slug === "string" ? params.slug : Array.isArray(params.slug) ? params.slug[0] : undefined;
  const { canWrite, canDelete } = useKnowledgeViewerPolicy();
  const { success, error: toastError } = useToast();

  const doc = useKnowledgeDoc(slug);
  const revisions = useKnowledgeRevisions(slug);
  const update = useUpdateKnowledgeDoc();
  const del = useDeleteKnowledgeDoc();

  const [draft, setDraft] = useState<string>("");
  const [confirmDelete, setConfirmDelete] = useState(false);

  useEffect(() => {
    if (doc.data) setDraft(doc.data.body_markdown);
  }, [doc.data]);

  if (!slug) {
    return (
      <div className="knowledge-stage">
        <p>Invalid doc URL.</p>
      </div>
    );
  }

  return (
    <>
      <KnowledgeStyles />
      <div className="knowledge-stage" data-testid={`knowledge-detail-${slug}`}>
        <div style={{ marginBottom: 16 }}>
          <Link
            href="/settings/knowledge"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              color: "#6d28d9",
              fontSize: 12,
              fontWeight: 600,
              textDecoration: "none",
            }}
          >
            <ArrowLeft size={12} /> Workplace Knowledge
          </Link>
        </div>

        <div className="knowledge-header">
          <div>
            <div className="knowledge-eyebrow">
              ⚙ Workspace settings · Knowledge
            </div>
            <h1 className="knowledge-title" data-testid="knowledge-detail-title">
              {doc.data?.title ?? slug}
            </h1>
            {doc.data ? (
              <p className="knowledge-sub">
                <span className="knowledge-pill">
                  {categoryLabel(doc.data.category)}
                </span>{" "}
                <span
                  className="knowledge-pill knowledge-pill-version"
                  data-testid="knowledge-detail-version"
                >
                  v{doc.data.version}
                </span>
              </p>
            ) : null}
          </div>
        </div>

        {doc.error ? (
          <div className="knowledge-error-banner" role="alert">
            Couldn&apos;t load doc:{" "}
            {String(doc.error.message ?? doc.error)}
          </div>
        ) : null}

        <div className="knowledge-detail-layout">
          <div>
            <KnowledgeEditor
              body={draft}
              onBodyChange={setDraft}
              isSaving={update.isPending}
              readOnly={!canWrite}
              disabled={!doc.data}
              onSave={({ body, editReason }) => {
                if (!slug) return;
                update.mutate(
                  { slug, body_markdown: body, edit_reason: editReason || undefined },
                  {
                    onSuccess: (d) => {
                      success("Saved", {
                        description: `Now at v${d.version}.`,
                      });
                    },
                    onError: (err) => {
                      toastError("Couldn't save", { description: err.message });
                    },
                  },
                );
              }}
            />

            {canDelete && doc.data ? (
              <div>
                {confirmDelete ? (
                  <div
                    className="knowledge-error-banner"
                    role="alert"
                    data-testid="knowledge-delete-confirm"
                    style={{ marginTop: 24 }}
                  >
                    <div style={{ marginBottom: 8 }}>
                      Delete <strong>{doc.data.title}</strong>? Workers will
                      stop receiving this doc on their next turn.
                    </div>
                    <div style={{ display: "flex", gap: 8 }}>
                      <button
                        type="button"
                        className="knowledge-modal-cancel"
                        onClick={() => setConfirmDelete(false)}
                      >
                        Cancel
                      </button>
                      <button
                        type="button"
                        className="knowledge-delete-btn"
                        style={{ marginTop: 0 }}
                        disabled={del.isPending}
                        onClick={() => {
                          if (!slug) return;
                          del.mutate(slug, {
                            onSuccess: () => {
                              success(`${doc.data?.title ?? slug} deleted`);
                              router.push("/settings/knowledge");
                            },
                            onError: (err) => {
                              toastError("Couldn't delete", {
                                description: err.message,
                              });
                              setConfirmDelete(false);
                            },
                          });
                        }}
                      >
                        <Trash2 size={12} /> Confirm delete
                      </button>
                    </div>
                  </div>
                ) : (
                  <button
                    type="button"
                    className="knowledge-delete-btn"
                    onClick={() => setConfirmDelete(true)}
                    data-testid="knowledge-delete-btn"
                  >
                    <Trash2 size={12} /> Delete doc
                  </button>
                )}
              </div>
            ) : null}
          </div>

          <VersionHistorySidebar
            slug={slug}
            revisions={revisions.data?.revisions ?? []}
            isLoading={revisions.isLoading}
          />
        </div>
      </div>
    </>
  );
}
