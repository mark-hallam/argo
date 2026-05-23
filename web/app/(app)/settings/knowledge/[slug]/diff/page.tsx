"use client";

import Link from "next/link";
import { useMemo } from "react";
import { useParams, useSearchParams } from "next/navigation";
import { ArrowLeft } from "lucide-react";
import { KnowledgeStyles } from "@/components/skycot/knowledge/styles";
import { useKnowledgeRevisions } from "@/lib/hooks/use-workplace-knowledge";

/**
 * `/settings/knowledge/<slug>/diff?from=<v>&to=<v>` — lightweight diff
 * viewer. Renders the stored `diff_from_prior` unified-diff string of
 * the target revision in a plain `<pre>`.
 *
 * Wave-E brief 22 (`web-knowledge-diff-viewer`) ships the rich
 * semantic-diff surface; this brief keeps the viewer minimal so the
 * version-history sidebar's click target doesn't 404.
 */
export default function WorkplaceKnowledgeDiffPage() {
  const params = useParams();
  const search = useSearchParams();
  const slug =
    typeof params.slug === "string"
      ? params.slug
      : Array.isArray(params.slug)
        ? params.slug[0]
        : undefined;
  const fromVersion = parseVersion(search.get("from"));
  const toVersion = parseVersion(search.get("to"));
  const revisions = useKnowledgeRevisions(slug);

  const target = useMemo(() => {
    if (!revisions.data) return undefined;
    return revisions.data.revisions.find((r) => r.version === toVersion);
  }, [revisions.data, toVersion]);

  return (
    <>
      <KnowledgeStyles />
      <div className="knowledge-stage" data-testid={`knowledge-diff-${slug}`}>
        <div style={{ marginBottom: 16 }}>
          <Link
            href={`/settings/knowledge/${encodeURIComponent(slug ?? "")}`}
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
            <ArrowLeft size={12} /> Back to doc
          </Link>
        </div>

        <div className="knowledge-header">
          <div>
            <div className="knowledge-eyebrow">⚙ Workspace settings · Knowledge · Diff</div>
            <h1 className="knowledge-title">Diff</h1>
            <p className="knowledge-sub">
              {fromVersion !== undefined ? (
                <>
                  v{fromVersion} → v{toVersion}
                </>
              ) : (
                <>v{toVersion} (initial revision)</>
              )}
            </p>
          </div>
        </div>

        {revisions.isLoading ? (
          <div className="knowledge-empty">Loading diff…</div>
        ) : !target ? (
          <div className="knowledge-empty" data-testid="knowledge-diff-empty">
            Revision v{toVersion} not found.
          </div>
        ) : !target.diff_from_prior ? (
          <div className="knowledge-empty" data-testid="knowledge-diff-no-prior">
            This is the initial revision — no diff to show.
          </div>
        ) : (
          <pre className="knowledge-diff-pre" data-testid="knowledge-diff-pre">
            {target.diff_from_prior}
          </pre>
        )}
      </div>
    </>
  );
}

function parseVersion(raw: string | null): number | undefined {
  if (raw === null) return undefined;
  const n = Number.parseInt(raw, 10);
  return Number.isFinite(n) ? n : undefined;
}
