"use client";

import { useState } from "react";
import { Mail } from "lucide-react";
import { CohortScopePicker } from "./CohortScopePicker";
import { ROLE_META, type MemberRole } from "./members-data";

export interface InvitePayload {
  email: string;
  role: MemberRole;
  cohortScope: string[] | null;
}

interface InviteMemberModalProps {
  open: boolean;
  onClose: () => void;
  onSend: (payload: InvitePayload) => void;
}

/**
 * Invite-teammate modal. Posts to gateway
 * `/api/v1/workspace/members` (stubbed: caller handles the actual
 * POST + toast). Carries cohort scope so the invitee lands with their
 * access boundary already set.
 */
export function InviteMemberModal({ open, onClose, onSend }: InviteMemberModalProps) {
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<MemberRole>("user");
  const [cohortScope, setCohortScope] = useState<string[] | null>(null);

  if (!open) return null;

  const reset = () => {
    setEmail("");
    setRole("user");
    setCohortScope(null);
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const handleSend = () => {
    onSend({ email, role, cohortScope });
    reset();
    onClose();
  };

  return (
    <div
      className="invite-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <div
        className="invite-modal"
        role="dialog"
        aria-labelledby="invite-h2"
        data-testid="invite-member-modal"
      >
        <h2 id="invite-h2">Invite a teammate</h2>
        <p>They&apos;ll get a magic-link email and land directly on your workspace.</p>

        <label className="invite-label" htmlFor="invite-email-input">Work email</label>
        <input
          id="invite-email-input"
          type="email"
          className="invite-input"
          placeholder="teammate@yourcompany.com"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          autoFocus
        />

        <label className="invite-label">Role</label>
        <div className="invite-role-grid">
          {(Object.keys(ROLE_META) as MemberRole[]).map((r) => (
            <button
              key={r}
              type="button"
              className={`invite-role-pill ${role === r ? "selected" : ""}`}
              onClick={() => setRole(r)}
            >
              <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                {ROLE_META[r].icon}
                {ROLE_META[r].label}
              </span>
            </button>
          ))}
        </div>

        <label className="invite-label">Cohort scope</label>
        <CohortScopePicker
          role={role}
          value={cohortScope}
          onChange={setCohortScope}
          compactHelper
        />

        <div className="invite-actions">
          <button type="button" className="invite-cancel" onClick={handleClose}>
            Cancel
          </button>
          <button
            type="button"
            className="invite-send"
            disabled={!email.includes("@")}
            onClick={handleSend}
            data-testid="invite-send-btn"
          >
            <Mail size={13} style={{ display: "inline", marginRight: 4 }} /> Send invite
          </button>
        </div>
      </div>
    </div>
  );
}
