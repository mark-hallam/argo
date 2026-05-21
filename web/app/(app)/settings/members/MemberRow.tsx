"use client";

import { MoreVertical } from "lucide-react";
import { CohortScopeCell } from "./CohortScopeCell";
import { ROLE_META, type SeedMember } from "./members-data";

interface MemberRowProps {
  member: SeedMember;
  onEdit: (member: SeedMember) => void;
}

export function MemberRow({ member, onEdit }: MemberRowProps) {
  const roleMeta = ROLE_META[member.role];
  return (
    <div className="member-row" data-testid={`member-row-${member.id}`}>
      <div className="member-name-block">
        <div className="member-avatar">
          {member.name
            .split(" ")
            .map((n) => n[0])
            .join("")
            .slice(0, 2)}
        </div>
        <div>
          <div className="member-name">{member.name}</div>
          <div className="member-email">{member.email}</div>
        </div>
      </div>
      <div>
        <span className="member-role-pill" style={{ background: roleMeta.color }}>
          {roleMeta.icon}
          {roleMeta.label}
        </span>
      </div>
      <CohortScopeCell role={member.role} cohortScope={member.cohortScope} />
      <div>
        <span
          className={`member-status-pill member-status-${
            member.status === "invited" ? "invited" : "active"
          }`}
        >
          {member.status === "invited"
            ? "Invite sent"
            : member.status === "suspended"
              ? "Suspended"
              : "Active"}
        </span>
      </div>
      <div style={{ fontSize: 12.5, color: "var(--muted-foreground, #52525b)" }}>
        {member.lastSeen}
      </div>
      <div style={{ display: "flex", gap: 6, alignItems: "center", justifyContent: "flex-end" }}>
        <button
          type="button"
          className="member-edit-btn"
          onClick={() => onEdit(member)}
          data-testid={`member-edit-${member.id}`}
        >
          Edit
        </button>
        <button type="button" className="member-row-actions" aria-label={`Manage ${member.name}`}>
          <MoreVertical size={15} />
        </button>
      </div>
    </div>
  );
}
