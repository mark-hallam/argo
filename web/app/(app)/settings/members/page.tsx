"use client";

import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { UserPlus, Loader2 } from "lucide-react";
import { EditMemberModal, type EditMemberPatch } from "./EditMemberModal";
import { InviteMemberModal, type InvitePayload } from "./InviteMemberModal";
import { MemberRow } from "./MemberRow";
import { MEMBERS_PAGE_STYLES } from "./members-styles";
import {
  ROLE_DESCRIPTIONS,
  ROLE_META,
  SEED_MEMBERS,
  type MemberRole,
  type SeedMember,
} from "./members-data";
import {
  inviteWorkspaceMember,
  listWorkspaceMembers,
  type PendingInvitation,
  type WorkspaceMember,
} from "@/lib/api/workspace-members";

/**
 * Team & roles management — `/settings/members`.
 *
 * Surfaces the 4 backend role tiers (owner / admin / user / guest)
 * defined in argo-capability::default_roles_for_workspace, plus the
 * cohort-scope dimension introduced by brief
 * `web-cohort-scoped-access`. Owner + admin always see everything;
 * user + guest can be limited to a subset of the workspace cohorts.
 *
 * Live data comes from `GET /api/v1/workspace/members` once gateway
 * is reachable; if the call fails (offline / pre-auth flow) the seed
 * roster keeps the demo rendering. Invite POSTs hit the real
 * `/api/v1/workspace/members/invite` endpoint — magic-link email is
 * dispatched server-side (`web-settings-members-invite-real-wire`).
 */
export default function MembersPage() {
  const queryClient = useQueryClient();
  const membersQuery = useQuery({
    queryKey: ["workspace", "members"],
    queryFn: listWorkspaceMembers,
    retry: false,
  });

  const [showInvite, setShowInvite] = useState(false);
  const [editing, setEditing] = useState<SeedMember | null>(null);
  // Stub edit-PATCH still in flight — `web-settings-members-edit-real-wire`
  // sibling brief will replace this with a real PATCH call. Local list
  // overrides keep the modal usable in the meantime.
  const [editOverrides, setEditOverrides] = useState<
    Record<string, { role: MemberRole; cohortScope: string[] | null }>
  >({});

  const liveMembers: SeedMember[] | null = useMemo(() => {
    const data = membersQuery.data;
    if (!data) return null;
    const merged = data.members.map((m) => projectMember(m));
    const pending = data.pending_invitations.map(projectPending);
    return [...merged, ...pending];
  }, [membersQuery.data]);

  const members: SeedMember[] = useMemo(() => {
    const base = liveMembers ?? SEED_MEMBERS;
    return base.map((m) => {
      const override = editOverrides[m.id];
      return override ? { ...m, role: override.role, cohortScope: override.cohortScope } : m;
    });
  }, [liveMembers, editOverrides]);

  const handleEditSave = (memberId: string, patch: EditMemberPatch) => {
    setEditOverrides((prev) => ({
      ...prev,
      [memberId]: { role: patch.role, cohortScope: patch.cohortScope },
    }));
  };

  const handleInvite = async (payload: InvitePayload) => {
    await inviteWorkspaceMember({
      email: payload.email,
      role: payload.role,
      cohort_scope: payload.cohortScope ?? [],
    });
    await queryClient.invalidateQueries({ queryKey: ["workspace", "members"] });
  };

  return (
    <>
      <style jsx global>{MEMBERS_PAGE_STYLES}</style>

      <div className="members-stage">
        <div className="members-header">
          <div>
            <div className="members-eyebrow">⚙ WORKSPACE SETTINGS</div>
            <h1 className="members-title">Team & roles</h1>
            <p className="members-sub">
              Invite humans onto your workspace. Workers (AI teammates) live on the Team page.
            </p>
          </div>
          <button type="button" className="members-invite-btn" onClick={() => setShowInvite(true)}>
            <UserPlus size={15} /> Invite teammate
          </button>
        </div>

        <div className="members-role-tiers">
          {(Object.keys(ROLE_DESCRIPTIONS) as MemberRole[]).map((key) => {
            const meta = ROLE_META[key];
            const desc = ROLE_DESCRIPTIONS[key];
            return (
              <div key={key} className="role-tier">
                <div className="role-tier-head">
                  <div className="role-tier-icon" style={{ background: meta.color }}>
                    {meta.icon}
                  </div>
                  <div className="role-tier-title">{desc.title}</div>
                </div>
                <p className="role-tier-blurb">{desc.blurb}</p>
                <ul className="role-tier-can">
                  {desc.can.slice(0, 3).map((c, i) => (
                    <li key={i}>{c}</li>
                  ))}
                </ul>
              </div>
            );
          })}
        </div>

        <div className="members-list" data-testid="members-list">
          <div className="members-list-head">
            <span>Member</span>
            <span>Role</span>
            <span>Cohort scope</span>
            <span>Status</span>
            <span>Last seen</span>
            <span />
          </div>
          {membersQuery.isPending ? (
            <div
              data-testid="members-loading"
              style={{ padding: "24px", textAlign: "center" }}
            >
              <Loader2 size={20} className="members-spinner" aria-hidden />
            </div>
          ) : null}
          {members.map((m) => (
            <MemberRow key={m.id} member={m} onEdit={setEditing} />
          ))}
        </div>
      </div>

      <InviteMemberModal
        open={showInvite}
        onClose={() => setShowInvite(false)}
        onSend={handleInvite}
      />
      <EditMemberModal
        member={editing}
        onClose={() => setEditing(null)}
        onSave={handleEditSave}
      />
    </>
  );
}

const ROLE_FALLBACK: MemberRole = "user";

function projectMember(row: WorkspaceMember): SeedMember {
  return {
    id: row.user_id,
    name: row.display_name ?? row.email.split("@")[0] ?? row.email,
    email: row.email,
    role: coerceRole(row.role),
    joinedAt: row.joined_at ? toIsoDate(row.joined_at) : "",
    lastSeen: row.last_login_at ? toLastSeen(row.last_login_at) : "never",
    status: "active",
    cohortScope: null,
  };
}

function projectPending(row: PendingInvitation): SeedMember {
  return {
    id: `invite_${row.invitation_id}`,
    name: row.email.split("@")[0] ?? row.email,
    email: row.email,
    role: coerceRole(row.role),
    joinedAt: toIsoDate(row.invited_at),
    lastSeen: "pending",
    status: "invited",
    cohortScope: row.cohort_scope.length ? row.cohort_scope : null,
  };
}

function coerceRole(role: string): MemberRole {
  if (role === "owner" || role === "admin" || role === "user" || role === "guest") return role;
  return ROLE_FALLBACK;
}

function toIsoDate(epochSecs: number): string {
  const ms = epochSecs > 1e12 ? epochSecs : epochSecs * 1000;
  return new Date(ms).toISOString().slice(0, 10);
}

function toLastSeen(epochSecs: number): string {
  const ms = epochSecs > 1e12 ? epochSecs : epochSecs * 1000;
  const minutes = Math.max(0, Math.round((Date.now() - ms) / 60_000));
  if (minutes < 1) return "online now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} hr ago`;
  return `${Math.round(hours / 24)} d ago`;
}
