"use client";

import { usePathname } from "next/navigation";
import { type ReactNode } from "react";
import { NavDrawer, NavItem } from "@/components/ui/NavDrawer";
import { HoverHint } from "@/components/ui/Tooltip";
import { DemoBanner } from "@/components/skycot/DemoBanner";
import { ThemeToggle } from "@/components/skycot/ThemeToggle";
import { UserPill } from "@/components/skycot/UserPill";
import {
  FOOTER_NAV_ITEMS,
  NAV_ITEMS,
  isNavItemActive,
} from "@/components/skycot/nav-items";

/**
 * Layout shell for every surface inside the `(app)` route group:
 * a desktop sidebar (mobile bottom-sheet) on the left, the page
 * content in the main pane. Optional breadcrumb slot renders above
 * the page content. When `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is set a
 * dismissible `<DemoBanner />` rides above everything so the viewer
 * knows they're looking at seeded sample data.
 *
 * Nav items come from the shared `nav-items` module so each surface
 * highlights consistently against the current pathname. The sidebar
 * bottom-left user identity comes from `<UserPill />`, which reads
 * the signed-in visitor's email + workspace from NextAuth.
 */
export interface AppShellProps {
  children: ReactNode;
  /** Optional breadcrumb element rendered in a sticky bar above the page. */
  breadcrumb?: ReactNode;
}

export function AppShell({ children, breadcrumb }: AppShellProps) {
  return (
    <div className="min-h-screen bg-[var(--background)]">
      <DemoBanner />
      <NavDrawer brand={<Brand />}>
        <AppNav />
      </NavDrawer>
      <main id="main" tabIndex={-1} className="md:pl-64 focus:outline-none">
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
        <SingleBinaryFooterChip />
        <div className="flex items-end gap-1">
          <div className="flex-1 min-w-0">
            <UserPill />
          </div>
          <ThemeToggle />
        </div>
      </div>
    </>
  );
}

function Brand() {
  return (
    <>
      <a href="/" className="flex items-center gap-2.5 group" style={{ textDecoration: "none" }}>
        <span
          aria-hidden
          style={{
            display: "block",
            width: 28,
            height: 28,
            borderRadius: 7,
            background: "linear-gradient(135deg, #8b5cf6, #ec4899)",
            boxShadow: "0 4px 12px -2px rgba(139,92,246,0.30)",
          }}
        />
        <span className="flex flex-col leading-tight">
          <span
            style={{
              fontFamily: "var(--font-audiowide)",
              fontSize: 14,
              letterSpacing: "0.08em",
              color: "var(--foreground)",
            }}
          >
            SKYCOT
          </span>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 9.5,
              textTransform: "uppercase",
              letterSpacing: "0.16em",
              color: "var(--muted-foreground)",
              fontWeight: 600,
              marginTop: 2,
            }}
          >
            AI teammates
          </span>
        </span>
      </a>
    </>
  );
}

/**
 * Single-binary footer chip — surfaces the mission pillar that's
 * invisible on 9 of 12 in-product surfaces per the wave-6 +
 * wave-10 mission audits. SKYCOT ships as ONE binary you can
 * self-host on a Mac mini or any Linux box — that's the SaaS-vs-
 * self-host moat. The chip is the cheapest possible reminder
 * that the buyer's data never has to leave their infrastructure.
 *
 * Renders as a small JetBrains-Mono pill below the workspace
 * switcher in the sidebar footer. Click → /pricing#self-host.
 */
function SingleBinaryFooterChip() {
  return (
    <HoverHint
      label="SKYCOT ships as one binary you can self-host. Click for details."
      side="right"
    >
      <a
        href="/pricing#self-host"
        className="mt-2 mx-2 flex items-center gap-2 rounded-[999px] border border-[var(--border)] bg-[var(--background)] px-3 py-1.5 text-[10.5px] uppercase tracking-[0.14em] font-semibold text-[var(--muted-foreground)] hover:border-[var(--brand-primary)] hover:text-[var(--foreground)] transition-colors duration-[var(--duration-fast)]"
        style={{ fontFamily: "var(--font-mono)" }}
      >
        <span
          aria-hidden
          className="inline-block size-1.5 rounded-full"
          style={{ background: "linear-gradient(135deg, #8b5cf6, #ec4899)" }}
        />
        Self-hosted · single binary
      </a>
    </HoverHint>
  );
}
