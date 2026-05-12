"use client";

import Link from "next/link";
import { ArrowRight } from "lucide-react";
import type { ExampleRole } from "@/lib/hire/examples";
import { cn } from "@/lib/utils";

/**
 * One example-role tile in the team empty-state grid (E3-e).
 *
 * The whole tile is a `next/link` anchor — the entire card body and
 * the bottom-right "Start with this →" affordance navigate to the
 * same destination (`/hire?example=<id>`). The affordance exists as a
 * focusable visual cue for keyboard / screen-reader users; the
 * underlying click handler is the same `<Link>` wrap. Hover + focus
 * styles live on the anchor so both interaction surfaces highlight
 * together.
 *
 * Visual tokens (border, accent, ring) use CSS custom properties so
 * dark mode + tenant theming flow through without component changes.
 */
export interface ExampleRoleTileProps {
  role: ExampleRole;
}

const tileHref = (id: string) => `/hire?example=${encodeURIComponent(id)}`;

export function ExampleRoleTile({ role }: ExampleRoleTileProps) {
  return (
    <Link
      href={tileHref(role.id)}
      data-testid={`team-example-tile-${role.id}`}
      className={cn(
        "group relative flex flex-col gap-3 h-full",
        "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)]",
        "p-5 text-left",
        "transition-[box-shadow,border-color,transform] duration-[var(--duration-fast)] ease-out",
        "hover:border-[var(--primary)] hover:shadow-md",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--background)]",
      )}
    >
      <h3 className="text-base font-semibold tracking-tight text-[var(--foreground)]">
        {role.title}
      </h3>
      <p className="text-sm text-[var(--muted-foreground)] line-clamp-2">
        {role.description}
      </p>
      <div className="mt-auto flex items-center justify-end">
        <span
          data-testid={`team-example-tile-cta-${role.id}`}
          className={cn(
            "inline-flex items-center gap-1.5 text-xs font-medium",
            "text-[var(--primary)] opacity-80 group-hover:opacity-100",
            "transition-opacity duration-[var(--duration-fast)] ease-out",
          )}
        >
          Start with this
          <ArrowRight aria-hidden className="size-3.5" />
        </span>
      </div>
    </Link>
  );
}
