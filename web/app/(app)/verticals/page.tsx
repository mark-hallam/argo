"use client";
import "./page.css";

import Link from "next/link";
import { useMemo, useState } from "react";
import {
  Briefcase,
  Building2,
  HeartPulse,
  Megaphone,
  PiggyBank,
  Search,
  Sparkles,
  Wrench,
} from "lucide-react";
import type { ReactNode } from "react";
import { useVerticals } from "@/lib/hooks/use-verticals";
import type { VerticalRow } from "@/lib/api/verticals";

/**
 * Vertical-template marketplace landing — brand-loud marketplace
 * register matching `/skills` A1 (per locked design memory
 * `project_skills_page_design_locked.md`). Reuses the hero-search
 * pattern (gradient icon + 18px input + try chips + count pill).
 *
 * Cards are sourced from the live gateway `/verticals` registry
 * (`useVerticals`); seed fallback in dev when the gateway is empty
 * or unreachable.
 */

const TRY_CHIPS = [
  "support",
  "finance",
  "sales",
  "marketing",
  "operations",
  "hr",
];

const ICON_ROTATION: ReactNode[] = [
  <Sparkles size={20} key="sparkles" />,
  <Briefcase size={20} key="briefcase" />,
  <PiggyBank size={20} key="piggy" />,
  <Megaphone size={20} key="mega" />,
  <Wrench size={20} key="wrench" />,
  <Building2 size={20} key="building" />,
  <HeartPulse size={20} key="heart" />,
];

function prettifyTag(tag: string): string {
  const cleaned = tag.replace(/[._-]+/g, " ").trim();
  if (cleaned.length === 0) return tag;
  return cleaned.charAt(0).toUpperCase() + cleaned.slice(1);
}

function cardSubtitle(row: VerticalRow): string {
  if (row.headline && row.headline.trim().length > 0) return row.headline;
  return row.description;
}

export default function VerticalsPage() {
  const { data, isLoading, isError, error } = useVerticals();
  const verticals = useMemo(() => data?.verticals ?? [], [data?.verticals]);
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (q.length === 0) return verticals;
    return verticals.filter(
      (v) =>
        v.name.toLowerCase().includes(q) ||
        v.description.toLowerCase().includes(q) ||
        v.tags.some((t) => t.toLowerCase().includes(q)),
    );
  }, [verticals, query]);

  return (
    <div className="verticals-stage">
      <div className="verticals-eyebrow">
        <span className="verticals-eyebrow-dot" />
        VERTICAL TEMPLATES
      </div>

      <h1 className="verticals-hero">
        Pick the role.
        <br />
        We <strong>ship the rest</strong>.
      </h1>

      <p className="verticals-sub">
        Every vertical is a complete starter package — system prompt, memory schema, approval
        gates, sample workflows, eval cases. Install one and your first AI hire is ready the
        same shift.
      </p>

      <div className="search-row">
        <div className="search-icon">
          <Search size={16} />
        </div>
        <input
          type="text"
          className="search-input"
          placeholder="Search verticals, roles, or tags…"
          aria-label="Search the vertical-template marketplace"
          data-testid="verticals-search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <span className="count-pill" data-testid="verticals-count">
          {verticals.length} verticals · {filtered.length} shown
        </span>
        <span className="search-kbd">⌘K</span>
      </div>

      <div className="try-row">
        <span className="try-label">TRY</span>
        {TRY_CHIPS.map((chip) => (
          <button
            type="button"
            key={chip}
            className="try-chip"
            tabIndex={0}
            onClick={() => setQuery(chip)}
          >
            {chip}
          </button>
        ))}
      </div>

      <div className="section-label">FEATURED VERTICALS</div>

      {isLoading && (
        <div className="state-panel" data-testid="verticals-loading">
          Loading vertical templates from the marketplace…
        </div>
      )}

      {isError && !isLoading && (
        <div
          className="state-panel state-panel-error"
          data-testid="verticals-error"
          role="alert"
        >
          Couldn’t reach the vertical-template registry
          {error instanceof Error ? `: ${error.message}` : "."}
        </div>
      )}

      {!isLoading && !isError && filtered.length === 0 && (
        <div className="state-panel" data-testid="verticals-empty">
          No verticals match “{query}”. Try a different search or clear the filter.
        </div>
      )}

      {!isLoading && !isError && filtered.length > 0 && (
        <div className="featured-grid">
          {filtered.map((vertical, idx) => (
            <Link
              key={vertical.id}
              href={`/verticals/${encodeURIComponent(vertical.id)}`}
              className="vertical-card"
              data-testid="vertical-card"
            >
              <div className="vertical-icon">
                {ICON_ROTATION[idx % ICON_ROTATION.length]}
              </div>
              <h2 className="vertical-title">{vertical.name}</h2>
              <p className="vertical-blurb">{cardSubtitle(vertical)}</p>
              <div className="vertical-meta">
                <span className="vertical-meta-item vertical-role">
                  {vertical.role_label}
                </span>
                <span className="vertical-meta-item">
                  {vertical.action_count} actions
                </span>
                {vertical.workflow_count > 0 ? (
                  <span className="vertical-meta-item">
                    {vertical.workflow_count} workflows
                  </span>
                ) : null}
              </div>
              <div className="vertical-tag-row">
                {vertical.tags.slice(0, 4).map((tag) => (
                  <span key={tag} className="vertical-tag">
                    {prettifyTag(tag)}
                  </span>
                ))}
              </div>
              <div className="vertical-id">v{vertical.version} · {vertical.id}</div>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
