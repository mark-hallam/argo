"use client";

import type { KnowledgeCategory, KnowledgeDoc } from "@/lib/api/workplace-knowledge";
import { CATEGORY_META } from "./category-meta";

interface Props {
  docs: KnowledgeDoc[];
  active: KnowledgeCategory | "all";
  onSelect: (category: KnowledgeCategory | "all") => void;
}

/** Left-rail category filter. Counts come from the unfiltered doc set
 *  so the rail labels remain stable across filter clicks. */
export function CategoryNav({ docs, active, onSelect }: Props) {
  const totalCount = docs.length;
  return (
    <nav className="knowledge-nav" aria-label="Knowledge categories">
      <button
        type="button"
        className={
          "knowledge-nav-item" +
          (active === "all" ? " knowledge-nav-item-active" : "")
        }
        data-testid="knowledge-nav-all"
        onClick={() => onSelect("all")}
      >
        <span>All</span>
        <span className="knowledge-nav-count">{totalCount}</span>
      </button>
      {CATEGORY_META.map((cat) => {
        const count = docs.filter((d) => d.category === cat.id).length;
        return (
          <button
            key={cat.id}
            type="button"
            className={
              "knowledge-nav-item" +
              (active === cat.id ? " knowledge-nav-item-active" : "")
            }
            data-testid={`knowledge-nav-${cat.id}`}
            onClick={() => onSelect(cat.id)}
          >
            <span>{cat.label}</span>
            <span className="knowledge-nav-count">{count}</span>
          </button>
        );
      })}
    </nav>
  );
}
