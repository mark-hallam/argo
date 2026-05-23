"use client";

import Link from "next/link";
import { formatHashForDisplay } from "@/components/skycot/ActivityRow";
import type { KnowledgeRevision } from "@/lib/api/workplace-knowledge";
import { relativeFromMicros } from "./KnowledgeCard";

interface Props {
  slug: string;
  revisions: KnowledgeRevision[];
  isLoading?: boolean;
}

/** Right-rail timeline of revisions. Each row links to the lightweight
 *  diff page with `from` = the prior version and `to` = this revision's
 *  version. Click on the truncated hash chip itself copies the full
 *  revision id (HashCell pattern from ActivityRow.tsx). */
export function VersionHistorySidebar({ slug, revisions, isLoading }: Props) {
  return (
    <aside className="knowledge-version-sidebar" data-testid="knowledge-version-sidebar">
      <h3>Version history</h3>
      {isLoading ? (
        <div style={{ fontSize: 12, color: "#52525b" }}>Loading revisions…</div>
      ) : revisions.length === 0 ? (
        <div style={{ fontSize: 12, color: "#52525b" }}>No revisions yet.</div>
      ) : (
        revisions.map((rev, idx) => {
          const prior = revisions[idx + 1];
          const diffHref = prior
            ? `/settings/knowledge/${encodeURIComponent(slug)}/diff?from=${prior.version}&to=${rev.version}`
            : `/settings/knowledge/${encodeURIComponent(slug)}/diff?to=${rev.version}`;
          return (
            <Link
              key={rev.id}
              href={diffHref}
              className="knowledge-version-row"
              data-testid={`knowledge-version-${rev.version}`}
            >
              <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                <RevisionHashChip hash={rev.id} />
                <span style={{ fontWeight: 600 }}>v{rev.version}</span>
              </div>
              <div style={{ marginTop: 4, color: "#52525b" }}>
                {relativeFromMicros(rev.edited_at_micros)} by {rev.edited_by}
              </div>
              {rev.edit_reason ? (
                <div style={{ marginTop: 4, color: "#3f3f46", fontStyle: "italic" }}>
                  {rev.edit_reason}
                </div>
              ) : null}
            </Link>
          );
        })
      )}
    </aside>
  );
}

function RevisionHashChip({ hash }: { hash: string }) {
  const display = formatHashForDisplay(hash);
  const copy = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      void navigator.clipboard.writeText(hash).catch(() => undefined);
    }
  };
  return (
    <button
      type="button"
      className="knowledge-version-hash"
      data-testid={`knowledge-version-hash-${hash}`}
      data-hash={hash}
      aria-label={`Revision ${hash}, click to copy`}
      title={`${hash} — Click to copy`}
      onClick={copy}
    >
      {display}
    </button>
  );
}
