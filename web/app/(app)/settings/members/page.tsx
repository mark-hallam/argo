"use client";

import { useState } from "react";
import { UserPlus, Crown, Shield, User, Eye, MoreVertical, Mail } from "lucide-react";

/**
 * Team & roles management — `/settings/members`.
 *
 * Surfaces the 4 backend role tiers (owner / admin / user / guest)
 * defined in argo-capability::default_roles_for_workspace, and
 * provides the missing invite-teammate UX.
 *
 * V1 ships a brand-aligned surface with seed members + an invite
 * dialog stub. Live data + actual invite send wires in a follow-up
 * brief once the gateway exposes `/api/v1/workspace/members` CRUD.
 */

interface SeedMember {
  name: string;
  email: string;
  role: "owner" | "admin" | "user" | "guest";
  joinedAt: string;
  lastSeen: string;
  status: "active" | "invited" | "suspended";
}

const SEED_MEMBERS: SeedMember[] = [
  {
    name: "Sam Harlow",
    email: "sam@acmeco.com",
    role: "owner",
    joinedAt: "2026-04-12",
    lastSeen: "online now",
    status: "active",
  },
  {
    name: "Olivia Chen",
    email: "olivia@acmeco.com",
    role: "admin",
    joinedAt: "2026-04-15",
    lastSeen: "12 min ago",
    status: "active",
  },
  {
    name: "Diego Alvarez",
    email: "diego@acmeco.com",
    role: "user",
    joinedAt: "2026-04-20",
    lastSeen: "2 hours ago",
    status: "active",
  },
  {
    name: "Theo Park",
    email: "theo@acmeco.com",
    role: "user",
    joinedAt: "2026-04-22",
    lastSeen: "yesterday",
    status: "active",
  },
  {
    name: "Naomi Wright",
    email: "naomi@auditor.gov",
    role: "guest",
    joinedAt: "2026-05-01",
    lastSeen: "3 days ago",
    status: "active",
  },
  {
    name: "Aroha Patel",
    email: "aroha@acmeco.com",
    role: "user",
    joinedAt: "2026-05-18",
    lastSeen: "never",
    status: "invited",
  },
];

const ROLE_META = {
  owner: { icon: <Crown size={13} />, color: "#8b5cf6", label: "Owner" },
  admin: { icon: <Shield size={13} />, color: "#ec4899", label: "Admin" },
  user: { icon: <User size={13} />, color: "#0891b2", label: "User" },
  guest: { icon: <Eye size={13} />, color: "#71717a", label: "Guest" },
} as const;

const ROLE_DESCRIPTIONS = {
  owner: {
    title: "Owner",
    blurb: "Full access. Manages billing, settings, members, all workers.",
    can: ["Everything an Admin can do", "Manage billing + payment method", "Delete the workspace", "Transfer ownership"],
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
} as const;

export default function MembersPage() {
  const [showInvite, setShowInvite] = useState(false);
  const [inviteEmail, setInviteEmail] = useState("");
  const [inviteRole, setInviteRole] = useState<keyof typeof ROLE_META>("user");

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .members-stage {
          padding: 48px 32px 96px;
          max-width: 1100px;
          margin: 0 auto;
        }
        .members-header {
          display: flex;
          align-items: flex-end;
          justify-content: space-between;
          margin-bottom: 36px;
          gap: 16px;
          flex-wrap: wrap;
        }
        .members-eyebrow {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 6px;
        }
        .members-title {
          font-family: "Inter", sans-serif;
          font-size: 30px;
          font-weight: 600;
          color: var(--foreground, #1e1b4b);
          margin: 0 0 4px;
          letter-spacing: -0.01em;
        }
        .members-sub {
          font-size: 14.5px;
          color: var(--muted-foreground, #3f3f46);
          margin: 0;
        }
        .members-invite-btn {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 11px 18px;
          border-radius: 10px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-weight: 600;
          font-size: 14px;
          border: none;
          cursor: pointer;
          box-shadow: 0 6px 16px -4px rgba(139, 92, 246, 0.4);
        }
        .members-role-tiers {
          display: grid;
          grid-template-columns: repeat(4, 1fr);
          gap: 12px;
          margin-bottom: 32px;
        }
        @media (max-width: 800px) {
          .members-role-tiers {
            grid-template-columns: repeat(2, 1fr);
          }
        }
        .role-tier {
          border: 1px solid var(--border, #ece9f5);
          border-radius: 12px;
          padding: 16px;
          background: white;
        }
        .role-tier-head {
          display: flex;
          align-items: center;
          gap: 8px;
          margin-bottom: 8px;
        }
        .role-tier-icon {
          width: 26px;
          height: 26px;
          border-radius: 7px;
          display: grid;
          place-items: center;
          color: white;
        }
        .role-tier-title {
          font-weight: 700;
          font-size: 14.5px;
          color: var(--foreground, #1e1b4b);
        }
        .role-tier-blurb {
          font-size: 12.5px;
          color: var(--muted-foreground, #3f3f46);
          line-height: 1.5;
          margin: 0 0 10px;
        }
        .role-tier-can {
          font-size: 12px;
          color: var(--muted-foreground, #71717a);
          line-height: 1.5;
          margin: 0;
          padding-left: 14px;
          list-style: square;
        }
        .members-list {
          background: white;
          border: 1px solid var(--border, #ece9f5);
          border-radius: 12px;
          overflow: hidden;
        }
        .members-list-head,
        .member-row {
          display: grid;
          grid-template-columns: 2.2fr 1fr 1fr 1fr 40px;
          gap: 12px;
          padding: 13px 18px;
          align-items: center;
        }
        .members-list-head {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          color: var(--muted-foreground, #71717a);
          background: var(--background, #fafaff);
          border-bottom: 1px solid var(--border, #ece9f5);
        }
        .member-row {
          font-size: 14px;
          border-bottom: 1px solid var(--border, #ece9f5);
        }
        .member-row:last-child {
          border-bottom: none;
        }
        .member-name-block {
          display: flex;
          align-items: center;
          gap: 12px;
        }
        .member-avatar {
          width: 36px;
          height: 36px;
          border-radius: 50%;
          background: linear-gradient(135deg, #c4b5fd, #fbcfe8);
          color: #1e1b4b;
          display: grid;
          place-items: center;
          font-weight: 600;
          font-size: 13px;
        }
        .member-name {
          font-weight: 600;
          color: var(--foreground, #1e1b4b);
        }
        .member-email {
          font-size: 12.5px;
          color: var(--muted-foreground, #71717a);
        }
        .member-role-pill {
          display: inline-flex;
          align-items: center;
          gap: 5px;
          padding: 4px 10px;
          border-radius: 999px;
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          font-weight: 700;
          color: white;
        }
        .member-status-pill {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.1em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 3px 9px;
          border-radius: 999px;
        }
        .member-status-active {
          background: rgba(20, 184, 166, 0.15);
          color: #0f766e;
        }
        .member-status-invited {
          background: rgba(245, 158, 11, 0.15);
          color: #b45309;
        }
        .member-row-actions {
          background: transparent;
          border: none;
          color: var(--muted-foreground, #71717a);
          cursor: pointer;
          padding: 6px;
          border-radius: 6px;
          display: grid;
          place-items: center;
        }
        .member-row-actions:hover {
          background: rgba(0, 0, 0, 0.04);
          color: var(--foreground, #1e1b4b);
        }
        .invite-overlay {
          position: fixed;
          inset: 0;
          background: rgba(30, 27, 75, 0.5);
          backdrop-filter: blur(4px);
          display: grid;
          place-items: center;
          z-index: 100;
        }
        .invite-modal {
          background: white;
          border-radius: 16px;
          padding: 28px;
          width: 100%;
          max-width: 460px;
          box-shadow: 0 30px 80px -20px rgba(0, 0, 0, 0.4);
        }
        .invite-modal h2 {
          font-family: "Inter", sans-serif;
          font-size: 22px;
          font-weight: 600;
          margin: 0 0 6px;
          color: var(--foreground, #1e1b4b);
        }
        .invite-modal p {
          font-size: 14px;
          color: var(--muted-foreground, #3f3f46);
          margin: 0 0 22px;
        }
        .invite-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 8px;
          display: block;
        }
        .invite-input {
          width: 100%;
          padding: 12px 16px;
          border-radius: 10px;
          border: 1px solid var(--border, #ece9f5);
          font-family: "Inter", sans-serif;
          font-size: 15px;
          margin-bottom: 16px;
        }
        .invite-input:focus {
          outline: none;
          border-color: #8b5cf6;
          box-shadow: 0 0 0 4px rgba(139, 92, 246, 0.1);
        }
        .invite-role-grid {
          display: grid;
          grid-template-columns: repeat(4, 1fr);
          gap: 8px;
          margin-bottom: 22px;
        }
        .invite-role-pill {
          padding: 9px;
          border-radius: 9px;
          border: 1px solid var(--border, #ece9f5);
          background: white;
          font-size: 12px;
          font-weight: 600;
          cursor: pointer;
          color: var(--muted-foreground, #71717a);
        }
        .invite-role-pill.selected {
          border-color: #8b5cf6;
          background: rgba(139, 92, 246, 0.08);
          color: #6d28d9;
        }
        .invite-actions {
          display: flex;
          gap: 10px;
          justify-content: flex-end;
        }
        .invite-cancel {
          padding: 11px 18px;
          border-radius: 10px;
          background: transparent;
          color: var(--foreground, #1e1b4b);
          border: 1px solid var(--border, #ece9f5);
          font-weight: 600;
          font-size: 14px;
          cursor: pointer;
        }
        .invite-send {
          padding: 11px 22px;
          border-radius: 10px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          border: none;
          font-weight: 700;
          font-size: 14px;
          cursor: pointer;
        }
        .invite-send:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
      `}</style>
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
          {(Object.keys(ROLE_DESCRIPTIONS) as Array<keyof typeof ROLE_DESCRIPTIONS>).map((key) => {
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

        <div className="members-list">
          <div className="members-list-head">
            <span>Member</span>
            <span>Role</span>
            <span>Status</span>
            <span>Last seen</span>
            <span />
          </div>
          {SEED_MEMBERS.map((m) => {
            const roleMeta = ROLE_META[m.role];
            return (
              <div className="member-row" key={m.email}>
                <div className="member-name-block">
                  <div className="member-avatar">
                    {m.name
                      .split(" ")
                      .map((n) => n[0])
                      .join("")
                      .slice(0, 2)}
                  </div>
                  <div>
                    <div className="member-name">{m.name}</div>
                    <div className="member-email">{m.email}</div>
                  </div>
                </div>
                <div>
                  <span className="member-role-pill" style={{ background: roleMeta.color }}>
                    {roleMeta.icon}
                    {roleMeta.label}
                  </span>
                </div>
                <div>
                  <span
                    className={`member-status-pill member-status-${m.status === "invited" ? "invited" : "active"}`}
                  >
                    {m.status === "invited" ? "Invite sent" : m.status === "suspended" ? "Suspended" : "Active"}
                  </span>
                </div>
                <div style={{ fontSize: 12.5, color: "var(--muted-foreground, #71717a)" }}>
                  {m.lastSeen}
                </div>
                <button type="button" className="member-row-actions" aria-label={`Manage ${m.name}`}>
                  <MoreVertical size={15} />
                </button>
              </div>
            );
          })}
        </div>
      </div>

      {showInvite ? (
        <div
          className="invite-overlay"
          onClick={(e) => {
            if (e.target === e.currentTarget) setShowInvite(false);
          }}
        >
          <div className="invite-modal" role="dialog" aria-labelledby="invite-h2">
            <h2 id="invite-h2">Invite a teammate</h2>
            <p>They&apos;ll get a magic-link email and land directly on your workspace.</p>

            <label className="invite-label">Work email</label>
            <input
              type="email"
              className="invite-input"
              placeholder="teammate@yourcompany.com"
              value={inviteEmail}
              onChange={(e) => setInviteEmail(e.target.value)}
              autoFocus
            />

            <label className="invite-label">Role</label>
            <div className="invite-role-grid">
              {(Object.keys(ROLE_META) as Array<keyof typeof ROLE_META>).map((r) => (
                <button
                  key={r}
                  type="button"
                  className={`invite-role-pill ${inviteRole === r ? "selected" : ""}`}
                  onClick={() => setInviteRole(r)}
                >
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                    {ROLE_META[r].icon}
                    {ROLE_META[r].label}
                  </span>
                </button>
              ))}
            </div>

            <div className="invite-actions">
              <button type="button" className="invite-cancel" onClick={() => setShowInvite(false)}>
                Cancel
              </button>
              <button
                type="button"
                className="invite-send"
                disabled={!inviteEmail.includes("@")}
                onClick={() => {
                  // Stub: real wire-up POSTs to gateway /api/v1/workspace/members
                  setShowInvite(false);
                  setInviteEmail("");
                }}
              >
                <Mail size={13} style={{ display: "inline", marginRight: 4 }} /> Send invite
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
