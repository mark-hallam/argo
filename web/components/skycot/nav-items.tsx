"use client";

import { Activity, GitBranch, Inbox, Layers, Mic, Settings, Sparkles, Users, Wand2 } from "lucide-react";
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
 *
 * The roster has exactly ONE sidebar entry: "Team" (`/team`, the
 * canonical rendered roster, matching the "5 teammates, one team"
 * page header + operator vocabulary). The `/workers` route still
 * resolves (deep-links, `/workers/<id>/overview`, `?just-hired=1`)
 * — only the duplicate *sidebar entry* was removed, because two
 * adjacent entries ("Team" + "Workers") for one roster surface read
 * as two features to a buyer.
 */
export interface NavItemDescriptor {
  href: string;
  label: string;
  icon: ReactNode;
}

export const NAV_ITEMS: NavItemDescriptor[] = [
  { href: "/", label: "Approvals", icon: <Inbox /> },
  { href: "/team", label: "Team", icon: <Users /> },
  { href: "/genesis", label: "Genesis", icon: <Wand2 /> },
  { href: "/pitch", label: "Pitch", icon: <Mic /> },
  { href: "/cohorts", label: "Cohorts", icon: <Layers /> },
  { href: "/skills", label: "Skills", icon: <Sparkles /> },
  { href: "/workflows", label: "Workflows", icon: <GitBranch /> },
  { href: "/audit", label: "Audit", icon: <Activity /> },
];

export const FOOTER_NAV_ITEMS: NavItemDescriptor[] = [
  { href: "/settings", label: "Settings", icon: <Settings /> },
];

export function isNavItemActive(pathname: string, href: string): boolean {
  if (href === "/") return pathname === "/" || pathname === "";
  return pathname === href || pathname.startsWith(`${href}/`);
}
