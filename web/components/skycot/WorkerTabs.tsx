"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";

/**
 * Tab strip rendered below the WorkerHero. Each tab is a real
 * sub-route (`/workers/<id>/<tab>`) so the URL is preserved on
 * reload and shareable. Active state is read from `usePathname()`.
 *
 * The brief mentions "shadcn Tabs primitive" — the SKYCOT design
 * system doesn't yet vendor that primitive, and Radix Tabs are
 * client-state-only (won't sync to URL). Using Next `<Link>` with
 * an aria-current marker gives us the URL-state behavior the brief
 * actually requires.
 */

export const WORKER_TAB_IDS = [
  "overview",
  "activity",
  "memory",
  "skills",
  "settings",
] as const;

export type WorkerTabId = (typeof WORKER_TAB_IDS)[number];

export const WORKER_TAB_LABELS: Record<WorkerTabId, string> = {
  overview: "Overview",
  activity: "Activity",
  memory: "Memory",
  skills: "Skills",
  settings: "Settings",
};

export function isWorkerTabId(value: string): value is WorkerTabId {
  return (WORKER_TAB_IDS as readonly string[]).includes(value);
}

/** Pulls the tab segment out of a worker-profile pathname.
 *  `/workers/<id>` → `"overview"` (default).
 *  `/workers/<id>/<tab>` → `tab`.
 *  Anything else → `"overview"`. */
export function activeWorkerTab(pathname: string | null): WorkerTabId {
  if (!pathname) return "overview";
  const segments = pathname.split("/").filter(Boolean);
  const segment = segments[2];
  if (!segment) return "overview";
  return isWorkerTabId(segment) ? segment : "overview";
}

export interface WorkerTabsProps {
  workerId: string;
  className?: string;
}

export function WorkerTabs({ workerId, className }: WorkerTabsProps) {
  const pathname = usePathname();
  const active = activeWorkerTab(pathname);

  return (
    <nav
      data-testid="worker-tabs"
      aria-label="Worker profile sections"
      className={cn(
        "border-b border-[var(--border)] -mx-4 px-4 sm:-mx-6 sm:px-6 lg:-mx-8 lg:px-8",
        className,
      )}
    >
      <ul role="tablist" className="flex items-center gap-1 overflow-x-auto">
        {WORKER_TAB_IDS.map((tab) => {
          const isActive = tab === active;
          return (
            <li key={tab} role="presentation">
              <Link
                role="tab"
                aria-selected={isActive}
                aria-current={isActive ? "page" : undefined}
                data-testid={`worker-tab-${tab}`}
                data-active={isActive || undefined}
                href={`/workers/${workerId}/${tab}`}
                className={cn(
                  "inline-flex items-center h-10 px-3 -mb-px border-b-2 cursor-pointer",
                  "text-sm transition-colors duration-[var(--duration-fast)]",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--background)]",
                  isActive
                    ? "border-[var(--primary)] text-[var(--foreground)] font-medium"
                    : "border-transparent text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--accent)]/40",
                )}
              >
                {WORKER_TAB_LABELS[tab]}
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
