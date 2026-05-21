"use client";
import { ChevronDown, ChevronRight, PanelLeftClose, PanelLeftOpen, Search } from "lucide-react";
import { useMemo, useState } from "react";

import "./BuilderSidebarPalette.css";
import {
  PALETTE_DRAG_MIME,
  PALETTE_ITEM_ICON,
  SIDEBAR_PALETTE_GROUPS,
  STEP_KIND_META,
  type PaletteItem,
  type SidebarPaletteGroup,
} from "./types";

interface Props {
  onSelect: (item: PaletteItem) => void;
}

/**
 * Left-side drag-to-add palette for the workflow builder.
 *
 * Closes the n8n / UIpath "power-user power-tool" gap: a permanent
 * grouped catalog of step kinds + skill variants that the user can
 * either click (append-to-canvas) or drag onto the canvas (drop at
 * coordinate, auto-wire from nearest predecessor).
 *
 * Layout:
 *   • Whole-palette collapse to a 38px rail (header click)
 *   • Per-group expand/collapse for browsing density
 *   • Sticky search filter at the top
 *
 * Drag uses an HTML5 `dataTransfer` payload on the custom
 * `application/x-skycot-palette-item` MIME — avoids collision with
 * React Flow's pointer-event-based node drag handles.
 */
export function BuilderSidebarPalette({ onSelect }: Props) {
  const [collapsed, setCollapsed] = useState(false);
  const [search, setSearch] = useState("");
  const [openGroups, setOpenGroups] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(SIDEBAR_PALETTE_GROUPS.map((g) => [g.label, true])),
  );

  const filtered: SidebarPaletteGroup[] = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return SIDEBAR_PALETTE_GROUPS;
    return SIDEBAR_PALETTE_GROUPS
      .map((g) => ({
        ...g,
        items: g.items.filter(
          (i) =>
            i.label.toLowerCase().includes(q)
            || i.description.toLowerCase().includes(q)
            || g.label.toLowerCase().includes(q),
        ),
      }))
      .filter((g) => g.items.length > 0);
  }, [search]);

  if (collapsed) {
    return (
      <aside
        className="builder-palette-rail"
        data-testid="builder-palette-rail"
        aria-label="Step palette (collapsed)"
      >
        <button
          type="button"
          className="builder-palette-rail-toggle"
          onClick={() => setCollapsed(false)}
          aria-label="Expand palette"
          aria-expanded={false}
          title="Expand palette"
        >
          <PanelLeftOpen size={14} />
        </button>
      </aside>
    );
  }

  return (
    <aside
      className="builder-palette"
      data-testid="builder-palette"
      aria-label="Step palette"
    >
      <button
        type="button"
        className="builder-palette-head"
        onClick={() => setCollapsed(true)}
        aria-label="Collapse palette"
        aria-expanded
        title="Collapse palette"
        data-testid="builder-palette-collapse"
      >
        <span className="builder-palette-eyebrow">Palette</span>
        <PanelLeftClose size={14} />
      </button>
      <div className="builder-palette-search-wrap">
        <Search size={12} className="builder-palette-search-icon" aria-hidden />
        <input
          type="search"
          className="builder-palette-search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search steps…"
          aria-label="Filter palette"
          data-testid="builder-palette-search"
        />
      </div>
      <div className="builder-palette-list">
        {filtered.length === 0 ? (
          <div className="builder-palette-empty" data-testid="builder-palette-empty">
            No matches.
          </div>
        ) : (
          filtered.map((group) => {
            const open = openGroups[group.label] !== false;
            return (
              <section key={group.label} className="builder-palette-group">
                <button
                  type="button"
                  className="builder-palette-group-head"
                  onClick={() =>
                    setOpenGroups((s) => ({ ...s, [group.label]: !open }))
                  }
                  aria-expanded={open}
                  data-testid={`builder-palette-group-${slug(group.label)}`}
                >
                  {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                  <span>{group.label}</span>
                  <span className="builder-palette-group-count">{group.items.length}</span>
                </button>
                {open
                  ? (
                    <div className="builder-palette-items">
                      {group.items.map((item) => {
                        const meta = STEP_KIND_META[item.kind];
                        const icon = PALETTE_ITEM_ICON[item.id] ?? meta.icon;
                        return (
                          <div
                            key={item.id}
                            draggable
                            role="button"
                            tabIndex={0}
                            className="builder-palette-item"
                            data-testid={`builder-palette-item-${item.id}`}
                            title={item.description}
                            onDragStart={(e) => {
                              e.dataTransfer.setData(
                                PALETTE_DRAG_MIME,
                                JSON.stringify(item),
                              );
                              e.dataTransfer.effectAllowed = "copy";
                              e.currentTarget.classList.add("is-dragging");
                            }}
                            onDragEnd={(e) => {
                              e.currentTarget.classList.remove("is-dragging");
                            }}
                            onClick={() => onSelect(item)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter" || e.key === " ") {
                                e.preventDefault();
                                onSelect(item);
                              }
                            }}
                          >
                            <span
                              className="builder-palette-item-icon"
                              style={{ background: meta.color }}
                              aria-hidden
                            >
                              {icon}
                            </span>
                            <span className="builder-palette-item-text">
                              <span className="builder-palette-item-label">
                                {item.label}
                              </span>
                              <span className="builder-palette-item-desc">
                                {item.description}
                              </span>
                            </span>
                          </div>
                        );
                      })}
                    </div>
                  )
                  : null}
              </section>
            );
          })
        )}
      </div>
      <div className="builder-palette-foot">
        Drag onto canvas — drop wires it after the nearest step.
      </div>
    </aside>
  );
}

function slug(label: string): string {
  return label.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}
