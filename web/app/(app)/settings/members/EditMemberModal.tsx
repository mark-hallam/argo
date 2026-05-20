"use client";

import { useEffect, useState } from "react";
import { CohortScopePicker } from "./CohortScopePicker";
import { ROLE_META, type MemberRole, type SeedMember } from "./members-data";

export interface EditMemberPatch {
  role: MemberRole;
  cohortScope: string[] | null;
}

interface EditMemberModalProps {
  member: SeedMember | null;
  onClose: () => void;
  onSave: (memberId: string, patch: EditMemberPatch) => void;
}

/**
 * Edit-access modal. Mirrors the documented contract for
 * `PATCH /api/v1/members/<id>` with body
 * `{ role, cohort_scope: [<cohort-id>] | null }`. The gateway endpoint
 * is wired in a follow-up brief; the caller stubs the actual PATCH +
 * toast and updates local state in the interim.
 */
export function EditMemberModal({ member, onClose, onSave }: EditMemberModalProps) {
  const [role, setRole] = useState<MemberRole>("user");
  const [cohortScope, setCohortScope] = useState<string[] | null>(null);

  useEffect(() => {
    if (member !== null) {
      setRole(member.role);
      setCohortScope(member.cohortScope);
    }
  }, [member]);

  if (member === null) return null;

  const handleSave = () => {
    onSave(member.id, { role, cohortScope });
    onClose();
  };

  return (
    <div
      className="invite-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="invite-modal"
        role="dialog"
        aria-labelledby="edit-member-h2"
        data-testid="edit-member-modal"
      >
        <h2 id="edit-member-h2">Edit access for {member.name}</h2>
        <p>
          {member.email} · joined {member.joinedAt}
        </p>

        <label className="invite-label">Role</label>
        <div className="invite-role-grid">
          {(Object.keys(ROLE_META) as MemberRole[]).map((r) => (
            <button
              key={r}
              type="button"
              className={`invite-role-pill ${role === r ? "selected" : ""}`}
              onClick={() => setRole(r)}
              data-testid={`edit-role-${r}`}
            >
              <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                {ROLE_META[r].icon}
                {ROLE_META[r].label}
              </span>
            </button>
          ))}
        </div>

        <label className="invite-label">Cohort scope</label>
        <CohortScopePicker role={role} value={cohortScope} onChange={setCohortScope} />

        <div className="invite-actions">
          <button type="button" className="invite-cancel" onClick={onClose}>
            Cancel
          </button>
          <button
            type="button"
            className="invite-send"
            onClick={handleSave}
            data-testid="edit-save-btn"
          >
            Save changes
          </button>
        </div>
      </div>
    </div>
  );
}
