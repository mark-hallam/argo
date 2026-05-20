/**
 * Static data for `/settings/members`: seed roster, role metadata,
 * and role-tier descriptions.
 *
 * Extracted from `page.tsx` so the page stays well under the 500-LOC
 * ceiling once the cohort-scope surfaces (chip column, edit modal,
 * scope picker) join it. Live data + invite POST wires in a follow-up
 * brief once the gateway exposes `/api/v1/workspace/members` CRUD.
 */

import { Crown, Shield, User, Eye } from "lucide-react";
import type { ReactNode } from "react";

export type MemberRole = "owner" | "admin" | "user" | "guest";

export interface SeedMember {
  id: string;
  name: string;
  email: string;
  role: MemberRole;
  joinedAt: string;
  lastSeen: string;
  status: "active" | "invited" | "suspended";
  /**
   * Cohort scope. `null` → all cohorts (default for owner / admin).
   * Array of cohort_ids from `SAMPLE_COHORT_SUMMARIES`. Owner + admin
   * always see everything regardless of this value; the row still
   * renders "All cohorts" for those roles.
   */
  cohortScope: string[] | null;
}

export const SEED_MEMBERS: SeedMember[] = [
  {
    id: "mbr_sam",
    name: "Sam Harlow",
    email: "sam@acmeco.com",
    role: "owner",
    joinedAt: "2026-04-12",
    lastSeen: "online now",
    status: "active",
    cohortScope: null,
  },
  {
    id: "mbr_olivia",
    name: "Olivia Chen",
    email: "olivia@acmeco.com",
    role: "admin",
    joinedAt: "2026-04-15",
    lastSeen: "12 min ago",
    status: "active",
    cohortScope: null,
  },
  {
    id: "mbr_diego",
    name: "Diego Alvarez",
    email: "diego@acmeco.com",
    role: "user",
    joinedAt: "2026-04-20",
    lastSeen: "2 hours ago",
    status: "active",
    cohortScope: ["sales::established"],
  },
  {
    id: "mbr_theo",
    name: "Theo Park",
    email: "theo@acmeco.com",
    role: "user",
    joinedAt: "2026-04-22",
    lastSeen: "yesterday",
    status: "active",
    cohortScope: ["support::trusted", "support::developing"],
  },
  {
    id: "mbr_naomi",
    name: "Naomi Wright",
    email: "naomi@auditor.gov",
    role: "guest",
    joinedAt: "2026-05-01",
    lastSeen: "3 days ago",
    status: "active",
    cohortScope: null,
  },
  {
    id: "mbr_aroha",
    name: "Aroha Patel",
    email: "aroha@acmeco.com",
    role: "user",
    joinedAt: "2026-05-18",
    lastSeen: "never",
    status: "invited",
    cohortScope: ["marketing::unknown", "marketplace::developing"],
  },
];

interface RoleMeta {
  icon: ReactNode;
  color: string;
  label: string;
}

export const ROLE_META: Record<MemberRole, RoleMeta> = {
  owner: { icon: <Crown size={13} />, color: "#8b5cf6", label: "Owner" },
  admin: { icon: <Shield size={13} />, color: "#ec4899", label: "Admin" },
  user: { icon: <User size={13} />, color: "#0891b2", label: "User" },
  guest: { icon: <Eye size={13} />, color: "#71717a", label: "Guest" },
};

interface RoleDescription {
  title: string;
  blurb: string;
  can: string[];
}

export const ROLE_DESCRIPTIONS: Record<MemberRole, RoleDescription> = {
  owner: {
    title: "Owner",
    blurb: "Full access. Manages billing, settings, members, all workers.",
    can: [
      "Everything an Admin can do",
      "Manage billing + payment method",
      "Delete the workspace",
      "Transfer ownership",
    ],
  },
  admin: {
    title: "Admin",
    blurb: "Manages workers + workflows + members.",
    can: [
      "Hire / pause / archive workers",
      "Edit workflows + skills",
      "Invite new members",
      "Approve actions on any worker",
      "View full audit log",
    ],
  },
  user: {
    title: "User",
    blurb: "Day-to-day operator. Uses workers + approves their decisions.",
    can: [
      "Approve / refuse actions on assigned workers",
      "View assigned workers + cohorts",
      "Read audit log",
      "Cannot edit workflows or hire workers",
    ],
  },
  guest: {
    title: "Guest",
    blurb: "Read-only. Good for auditors, observers, prospective hires.",
    can: [
      "Read audit log",
      "View workers + their KPIs",
      "Cannot approve, edit, or hire",
      "Cannot see billing or settings",
    ],
  },
};

/**
 * Roles that bypass cohort scoping. Owner + admin always see every
 * worker / workflow / audit entry regardless of any cohortScope value
 * set on their member row. The picker reflects this by disabling the
 * "Specific cohorts" branch when these roles are selected.
 */
export function roleOverridesCohortScope(role: MemberRole): boolean {
  return role === "owner" || role === "admin";
}
