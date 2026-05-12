"use client";

import { ChevronRight } from "lucide-react";
import { forwardRef, type ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * Generic breadcrumb primitive following the shadcn composition
 * pattern. The parent shell decides where to mount it (typically
 * the AppShell breadcrumb slot).
 *
 * Usage:
 *   <Breadcrumb>
 *     <BreadcrumbItem href="/team">Team</BreadcrumbItem>
 *     <BreadcrumbSeparator />
 *     <BreadcrumbItem current>Mariah</BreadcrumbItem>
 *   </Breadcrumb>
 */
export interface BreadcrumbProps {
  children: ReactNode;
  className?: string;
}

export function Breadcrumb({ children, className }: BreadcrumbProps) {
  return (
    <nav aria-label="Breadcrumb" className={cn("flex items-center gap-1.5 text-sm", className)}>
      <ol className="flex flex-wrap items-center gap-1.5">{children}</ol>
    </nav>
  );
}

export interface BreadcrumbItemProps {
  /** Link target; omit for non-link items (e.g., the current page). */
  href?: string;
  /** Marks the current page; renders unlinked and with aria-current. */
  current?: boolean;
  children: ReactNode;
  className?: string;
}

export const BreadcrumbItem = forwardRef<HTMLLIElement, BreadcrumbItemProps>(
  ({ href, current, children, className }, ref) => {
    return (
      <li
        ref={ref}
        className={cn(
          "inline-flex items-center text-[var(--muted-foreground)]",
          current && "text-[var(--foreground)] font-medium",
          className,
        )}
      >
        {href && !current ? (
          <a
            href={href}
            className="rounded px-1 -mx-1 hover:text-[var(--foreground)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
          >
            {children}
          </a>
        ) : (
          <span aria-current={current ? "page" : undefined}>{children}</span>
        )}
      </li>
    );
  },
);
BreadcrumbItem.displayName = "BreadcrumbItem";

export function BreadcrumbSeparator({ className }: { className?: string }) {
  return (
    <li aria-hidden className={cn("text-[var(--muted-foreground)]/60", className)}>
      <ChevronRight className="size-3.5" />
    </li>
  );
}
