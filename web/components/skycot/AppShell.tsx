"use client";

import { ChevronDown } from "lucide-react";
import { usePathname } from "next/navigation";
import type { ReactNode } from "react";
import { Avatar } from "@/components/ui/Avatar";
import { NavDrawer, NavItem } from "@/components/ui/NavDrawer";
import {
  FOOTER_NAV_ITEMS,
  NAV_ITEMS,
  isNavItemActive,
} from "@/components/skycot/nav-items";

/**
 * Layout shell for every surface inside the `(app)` route group:
 * a desktop sidebar (mobile bottom-sheet) on the left, the page
 * content in the main pane. Optional breadcrumb slot renders above
 * the page content.
 *
 * Nav items come from the shared `nav-items` module so each surface
 * highlights consistently against the current pathname.
 */
export interface AppShellProps {
  children: ReactNode;
  /** Optional breadcrumb element rendered in a sticky bar above the page. */
  breadcrumb?: ReactNode;
}

export function AppShell({ children, breadcrumb }: AppShellProps) {
  return (
    <div className="min-h-screen bg-[var(--background)]">
      <NavDrawer brand={<Brand />}>
        <AppNav />
      </NavDrawer>
      <main className="md:pl-64">
        {breadcrumb ? (
          <div className="sticky top-0 z-10 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
            <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-3">{breadcrumb}</div>
          </div>
        ) : null}
        {children}
      </main>
    </div>
  );
}

function AppNav() {
  const pathname = usePathname() ?? "/";
  return (
    <>
      {NAV_ITEMS.map((item) => (
        <NavItem key={item.href} href={item.href} active={isNavItemActive(pathname, item.href)}>
          {item.icon}
          {item.label}
        </NavItem>
      ))}
      <div className="mt-auto flex flex-col gap-1 pt-3 border-t border-[var(--border)]">
        {FOOTER_NAV_ITEMS.map((item) => (
          <NavItem key={item.href} href={item.href} active={isNavItemActive(pathname, item.href)}>
            {item.icon}
            {item.label}
          </NavItem>
        ))}
        <WorkspaceFooter />
      </div>
    </>
  );
}

function Brand() {
  return (
    <a href="/" className="flex items-center gap-2.5 group">
      <span
        aria-hidden
        className="grid size-8 place-items-center rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] font-semibold text-sm shadow-sm"
      >
        S
      </span>
      <span className="flex flex-col leading-tight">
        <span className="text-sm font-semibold text-[var(--foreground)]">SKYCOT</span>
        <span className="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">
          AI teammates
        </span>
      </span>
    </a>
  );
}

function WorkspaceFooter() {
  return (
    <button
      type="button"
      className="mt-1 flex items-center gap-2.5 rounded-[var(--radius-md)] px-3 py-2 hover:bg-[var(--accent)] transition-colors duration-[var(--duration-fast)]"
    >
      <Avatar alt="Sam Harlow" size="sm" />
      <span className="flex-1 flex flex-col items-start leading-tight">
        <span className="text-sm font-medium text-[var(--foreground)]">Sam Harlow</span>
        <span className="text-[11px] text-[var(--muted-foreground)]">Acmeco · Owner</span>
      </span>
      <ChevronDown className="size-4 text-[var(--muted-foreground)]" />
    </button>
  );
}
