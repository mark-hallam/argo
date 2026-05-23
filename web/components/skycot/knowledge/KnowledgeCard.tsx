"use client";

import Link from "next/link";
import type { KnowledgeDoc } from "@/lib/api/workplace-knowledge";
import { categoryLabel } from "./category-meta";

interface Props {
  doc: KnowledgeDoc;
}

/** Doc card on the `/settings/knowledge` index grid. */
export function KnowledgeCard({ doc }: Props) {
  return (
    <Link
      href={`/settings/knowledge/${encodeURIComponent(doc.slug)}`}
      className="knowledge-card"
      data-testid={`knowledge-card-${doc.slug}`}
    >
      <h2 className="knowledge-card-title">{doc.title}</h2>
      <div className="knowledge-card-meta">
        <span className="knowledge-pill">{categoryLabel(doc.category)}</span>
        <span className="knowledge-pill knowledge-pill-version">v{doc.version}</span>
        <span>·</span>
        <span data-testid={`knowledge-card-updated-${doc.slug}`}>
          {relativeFromMicros(doc.updated_at_micros)} by {doc.updated_by}
        </span>
      </div>
    </Link>
  );
}

export function relativeFromMicros(micros: number): string {
  const then = Math.floor(micros / 1000);
  const diff = Math.max(0, Date.now() - then);
  const m = 60_000;
  const h = 60 * m;
  const d = 24 * h;
  if (diff < m) return "just now";
  if (diff < h) return `${Math.floor(diff / m)} min ago`;
  if (diff < d) return `${Math.floor(diff / h)} h ago`;
  if (diff < 30 * d) return `${Math.floor(diff / d)} d ago`;
  return new Date(then).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}
