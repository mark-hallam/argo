"use client";

import { useState } from "react";
import { UserPlus } from "lucide-react";
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

/**
 * Team & roles management — `/settings/members`.
 *
 * Surfaces the 4 backend role tiers (owner / admin / user / guest)
 * defined in argo-capability::default_roles_for_workspace, plus the
 * cohort-scope dimension introduced by brief
 * `web-cohort-scoped-access`. Owner + admin always see everything;
 * user + guest can be limited to a subset of the workspace cohorts.
 *
 * V1 ships with seed members + stubbed PATCH; the gateway
 * `/api/v1/members/<id>` PATCH wires in a follow-up brief.
 */
export default function MembersPage() {
  const [members, setMembers] = useState<SeedMember[]>(SEED_MEMBERS);
  const [showInvite, setShowInvite] = useState(false);
  const [editing, setEditing] = useState<SeedMember | null>(null);

  const handleEditSave = (memberId: string, patch: EditMemberPatch) => {
    setMembers((prev) =>
      prev.map((m) =>
        m.id === memberId
          ? { ...m, role: patch.role, cohortScope: patch.cohortScope }
          : m,
      ),
    );
    // Follow-up brief wires the real PATCH to /api/v1/members/<id>.
  };

  const handleInvite = (payload: InvitePayload) => {
    const newMember: SeedMember = {
      id: `mbr_invite_${Date.now()}`,
      name: payload.email.split("@")[0],
      email: payload.email,
      role: payload.role,
      joinedAt: new Date().toISOString().slice(0, 10),
      lastSeen: "never",
      status: "invited",
      cohortScope: payload.cohortScope,
    };
    setMembers((prev) => [...prev, newMember]);
    // Follow-up brief wires the real POST to /api/v1/workspace/members.
  };

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
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
