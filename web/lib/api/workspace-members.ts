/**
 * Workspace-members API client.
 *
 * Routes through the typed `lib/api/client.ts` wrapper. Backs the
 * `/settings/members` page (`web-settings-members-invite-real-wire`):
 * - `GET /api/v1/workspace/members` — current + pending list.
 * - `POST /api/v1/workspace/members/invite` — issue invite.
 * - `POST /api/v1/workspace/members/accept` — invitee accept.
 *
 * The companion `/accept-invite` page consumes `acceptInvitation`.
 */

import { apiGet, apiPost } from "@/lib/api/client";

export type WorkspaceMemberRole = "owner" | "admin" | "user" | "guest";

export interface WorkspaceMember {
  user_id: string;
  email: string;
  display_name: string | null;
  role: WorkspaceMemberRole | string;
  joined_at: number;
  last_login_at: number | null;
  /** ISO-8601 access expiry. Null for permanent staff. The gateway
   *  nightly auto-revoke job (out-of-scope sibling brief) flips
   *  `revoked_at = expires_at` and invalidates session cookies. */
  expires_at?: string | null;
}

export interface PendingInvitation {
  invitation_id: string;
  email: string;
  role: WorkspaceMemberRole | string;
  cohort_scope: string[];
  invited_by: string;
  invited_at: number;
  /** Invitation token expiry (24h-ish lifetime — distinct from
   *  member-record access expiry below). */
  expires_at: number;
  /** ISO-8601 access expiry baked into the invite. When the invitee
   *  accepts, this carries onto the member record. Null → permanent. */
  member_expires_at?: string | null;
}

export interface WorkspaceMembersResponse {
  tenant_id: string;
  members: WorkspaceMember[];
  pending_invitations: PendingInvitation[];
}

export interface InvitePayload {
  email: string;
  role: WorkspaceMemberRole;
  cohort_scope: string[];
  /** Time-bound access — ISO-8601 expiry that travels with the
   *  invite token onto the eventual member record. Null/omitted →
   *  permanent (default for staff). */
  expires_at?: string | null;
}

export interface InviteResult {
  invitation_id: string;
  expires_at: number;
  email_sent_to: string;
}

export interface AcceptResult {
  user_id: string;
  tenant_id: string;
  role: WorkspaceMemberRole | string;
  email: string;
  next_step: string;
}

export async function listWorkspaceMembers(): Promise<WorkspaceMembersResponse> {
  return apiGet<WorkspaceMembersResponse>("/api/v1/workspace/members");
}

export async function inviteWorkspaceMember(payload: InvitePayload): Promise<InviteResult> {
  return apiPost<InviteResult, InvitePayload>(
    "/api/v1/workspace/members/invite",
    payload,
  );
}

export async function acceptInvitation(token: string): Promise<AcceptResult> {
  return apiPost<AcceptResult, { token: string }>(
    "/api/v1/workspace/members/accept",
    { token },
  );
}
