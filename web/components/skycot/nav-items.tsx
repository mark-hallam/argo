"use client";

import { Activity, Inbox, Settings, Sparkles, Users, Workflow } from "lucide-react";
import type { ReactNode } from "react";

/**
 * The canonical nav-item list, used by every surface inside the
 * `(app)` route group. Each entry carries an `href` and the prefix
 * AppShell uses to decide which item is active for the current
 * pathname (a row highlights when `pathname.startsWith(href)`).
 *
 * Adding a top-level surface: append a new entry here, then ship its
 * `(app)/<path>/page.tsx`. Until the route exists, clicking falls
 * through to `not-found.tsx` — that's intentional.
 */
export interface NavItemDescriptor {
  href: string;
  label: string;
  icon: ReactNode;
}

export const NAV_ITEMS: NavItemDescriptor[] = [
  { href: "/", label: "Approvals", icon: <Inbox /> },
  { href: "/team", label: "Team", icon: <Users /> },
  { href: "/workers", label: "Workers", icon: <Workflow /> },
  { href: "/audit", label: "Audit", icon: <Activity /> },
  { href: "/skills", label: "Skills", icon: <Sparkles /> },
];

export const FOOTER_NAV_ITEMS: NavItemDescriptor[] = [
  { href: "/settings", label: "Settings", icon: <Settings /> },
];

export function isNavItemActive(pathname: string, href: string): boolean {
  if (href === "/") return pathname === "/" || pathname === "";
  return pathname === href || pathname.startsWith(`${href}/`);
}
