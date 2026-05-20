# web-settings-members-invite-real-wire — wire member invite to gateway + magic link

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-settings-members-invite-real-wire`.
**Priority**: P1 — `/settings/members` page has a fully-styled invite UI
that ONLY mutates client-side `useState`. Operator invites someone,
modal closes, the invitee never gets an email and no backend row is
created. Demo and procurement both expose this.
**Sizing**: ~600-800 LOC across new `workspace_member_invitation` table +
2 gateway endpoints (issue / accept) + magic-link reuse + 4 PATCH endpoint
(edit member role/scope) + web wire-up + 8 tests.

---

## Why this brief exists

The `/settings/members` page (`web/app/(app)/settings/members/page.tsx`)
has TODO comments admitting:

```
// V1 ships with seed members + stubbed PATCH; the gateway
// `/api/v1/members/<id>` PATCH wires in a follow-up brief.

const handleInvite = (payload: InvitePayload) => {
  const newMember = { ...payload, status: "invited" };
  setMembers((prev) => [...prev, newMember]);
  // Follow-up brief wires the real POST to /api/v1/workspace/members.
};
```

The operator clicks "Invite", types an email, clicks Send — the modal
closes with a "Invitation sent" toast — but no email is sent, no DB row
created. The invitee never hears anything; if asked "did you send the
invite?", the operator believes they did.

This brief wires the page to real backend.

---

## Pre-flight verification

```bash
# 1. Confirm the TODO comments are still on origin/main.
rg "Follow-up brief wires the real" web/app/\(app\)/settings/members/page.tsx

# 2. Confirm magic-link substrate exists (the invite token will reuse it).
ls crates/argo-cli/src/handlers/gateway/auth_magic_link*.rs

# 3. Confirm no workspace_member_invitation table exists yet.
rg "workspace_member_invitation\|member_invite" crates/argo-store/migrations/

# 4. Confirm 4-role RBAC structure.
rg "owner.*admin.*user.*guest" crates/argo-capability/src/ -l
```

---

## Architecture — invite flow

```
1. Operator (owner/admin) hits Invite modal on /settings/members.
2. Frontend POST /api/v1/workspace/members/invite
   Body: { email, role, cohort_scope }
3. Gateway:
   a. RBAC gate: caller must be owner OR admin.
   b. INSERT into workspace_member_invitation
      (id, tenant_id, email, role, cohort_scope, invited_by,
       expires_at, accepted_at NULL).
   c. Issue magic-link token (reuse the existing
      auth_magic_link_token::issue helper) with payload
      `{ kind: "invite", invitation_id }`.
   d. Send email via the magic_link.html template with link
      pointing to /accept-invite?token=<token>.
   e. Emit AuditKind::WorkspaceMemberInvited.
   f. Return { invitation_id, expires_at }.
4. Frontend shows "Invitation sent to <email>. Expires in 7 days."
5. Invitee clicks the email link → /accept-invite page.
6. Page POSTs /api/v1/workspace/members/accept
   Body: { token }
7. Gateway:
   a. Consume token (atomic same as existing magic-link verify).
   b. Look up invitation by id from token payload.
   c. Find or create user_account row for the invitee's email.
   d. INSERT into workspace_member
      (tenant_id, user_id, role, cohort_scope, joined_at).
   e. Mark invitation accepted_at = now().
   f. Emit AuditKind::WorkspaceMemberJoined.
   g. Sign the user in (mint session) — they land on /workers.
```

---

## Phase 1 — Storage

### 1a. Migration

**File**: `crates/argo-store/migrations/NNN__workspace_member_invitation.sql`

```sql
-- Stream `web-settings-members-invite-real-wire`. One row per
-- outstanding invitation. Accepted rows stay forever as audit trail.

CREATE TABLE IF NOT EXISTS workspace_member_invitation (
    id                   TEXT      NOT NULL PRIMARY KEY,
    tenant_id            TEXT      NOT NULL,
    email                TEXT      NOT NULL,
    role                 TEXT      NOT NULL
                              CHECK(role IN ('owner','admin','user','guest')),
    cohort_scope_json    TEXT      NOT NULL DEFAULT '[]',
    invited_by           TEXT      NOT NULL,   -- principal that sent the invite
    invited_at           INTEGER   NOT NULL,
    expires_at           INTEGER   NOT NULL,
    accepted_at          INTEGER,              -- NULL until accept
    accepted_by_user_id  TEXT                  -- NULL until accept
);

CREATE INDEX IF NOT EXISTS idx_invitation_tenant_email
    ON workspace_member_invitation(tenant_id, email);
CREATE INDEX IF NOT EXISTS idx_invitation_expires
    ON workspace_member_invitation(expires_at);
```

### 1b. workspace_member table — verify it exists or add

```bash
rg "CREATE TABLE.*workspace_member" crates/argo-store/migrations/
```

If absent (likely — invite-flow is being built fresh):

```sql
CREATE TABLE IF NOT EXISTS workspace_member (
    tenant_id           TEXT      NOT NULL,
    user_id             TEXT      NOT NULL,
    role                TEXT      NOT NULL
                              CHECK(role IN ('owner','admin','user','guest')),
    cohort_scope_json   TEXT      NOT NULL DEFAULT '[]',
    joined_at           INTEGER   NOT NULL,
    PRIMARY KEY (tenant_id, user_id)
);
```

(Coordinate with `web-rbac-jwt-role-claim-and-gateway-enforce` — that brief
also touches `workspace_member.role` and may have already added it.)

### 1c. Store ops

**File**: `crates/argo-store/src/workspace_member_invitation.rs` (new)

```rust
pub struct Invitation {
    pub id: String,
    pub tenant_id: TenantId,
    pub email: String,
    pub role: String,
    pub cohort_scope: Vec<String>,
    pub invited_by: String,
    pub invited_at_micros: i64,
    pub expires_at_micros: i64,
    pub accepted_at_micros: Option<i64>,
    pub accepted_by_user_id: Option<String>,
}

pub async fn create_invitation(
    store: &SessionStore, tenant: &TenantId, email: &str, role: &str,
    cohort_scope: &[String], invited_by: &str,
) -> ArgoResult<Invitation> { /* INSERT, return row */ }

pub async fn lookup_invitation(
    store: &SessionStore, id: &str,
) -> ArgoResult<Option<Invitation>> { /* SELECT */ }

pub async fn accept_invitation(
    store: &SessionStore, id: &str, user_id: &str,
) -> ArgoResult<()> {
    // UPDATE accepted_at + accepted_by_user_id, then INSERT workspace_member
    // (both inside one tx).
}

pub async fn list_pending_for_tenant(
    store: &SessionStore, tenant: &TenantId,
) -> ArgoResult<Vec<Invitation>> { /* SELECT WHERE accepted_at IS NULL */ }
```

---

## Phase 2 — Magic-link extension

The existing `auth_magic_link_token::issue` issues tokens for **login**.
Invite tokens need a slightly different shape — embed an invitation id.

**File**: `crates/argo-cli/src/handlers/gateway/auth_magic_link_token.rs`

Extend the token-payload struct to include an optional `invitation_id`:

```diff
  struct TokenPayload {
      email: String,
      issued_at: i64,
+     invitation_id: Option<String>,   // None for login, Some for invite-accept
  }
```

Update `issue()` to accept an optional `invitation_id` and embed it.

`consume()` returns the payload — callers branch on `invitation_id.is_some()`.

---

## Phase 3 — Gateway endpoints

### 3a. POST /api/v1/workspace/members/invite

**File**: `crates/argo-cli/src/handlers/gateway/routes_workspace_members_invite.rs` (new)

```rust
async fn invite_response(
    body: &str, req: &str, store: ...
) -> (&'static str, String) {
    // 1. RBAC gate — depends on web-rbac-jwt-role-claim-and-gateway-enforce.
    //    For now: principal-only auth + role-from-header.
    let role = resolve_caller_role(req)?;
    if !matches!(role.as_str(), "owner" | "admin") {
        return forbidden("Only owners and admins can invite members.");
    }

    // 2. Parse body.
    let opts: InviteBody = serde_json::from_str(body)?;
    if !valid_email(&opts.email) {
        return bad_request("Invalid email address.");
    }
    if !["owner", "admin", "user", "guest"].contains(&opts.role.as_str()) {
        return bad_request("Role must be one of: owner, admin, user, guest.");
    }

    // 3. Check no existing pending invitation for this email.
    let existing = workspace_member_invitation::lookup_pending_by_email(
        store, &tenant, &opts.email
    ).await?;
    if existing.is_some() {
        return ("409 Conflict",
            json!({ "error": "An invitation for this email is already pending." }).to_string());
    }

    // 4. Create invitation row.
    let invitation = workspace_member_invitation::create_invitation(
        store, &tenant, &opts.email, &opts.role, &opts.cohort_scope, &principal,
    ).await?;

    // 5. Issue magic-link token with embedded invitation_id.
    let token = auth_magic_link_token::issue(
        store, &opts.email, Some(invitation.id.clone())
    ).await?;

    // 6. Send email.
    let accept_url = format!(
        "{base_url}/accept-invite?token={token}",
        base_url = env::var("ARGO_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into()),
    );
    let email_body = render_invite_email(
        &principal, &opts.email, &workspace.name, &opts.role, &accept_url
    )?;
    send_email(&opts.email, "You've been invited to a SKYCOT workspace", &email_body).await?;

    // 7. Emit audit.
    let _ = sink.emit(AuditKind::WorkspaceMemberInvited, json!({
        "actor": principal,
        "tenant_id": tenant.to_string(),
        "invitee_email": opts.email,
        "role": opts.role,
        "cohort_scope": opts.cohort_scope,
        "invitation_id": invitation.id,
        "expires_at": invitation.expires_at_micros,
    })).await;

    ("200 OK", json!({
        "invitation_id": invitation.id,
        "expires_at": invitation.expires_at_micros,
        "email_sent_to": opts.email,
    }).to_string())
}
```

### 3b. POST /api/v1/workspace/members/accept

```rust
async fn accept_response(body: &str, req: &str, store: ...) -> (&'static str, String) {
    let opts: AcceptBody = serde_json::from_str(body)?;
    let token_payload = auth_magic_link_token::consume(store, &opts.token).await?;
    let invitation_id = token_payload.invitation_id
        .ok_or_else(|| bad_request("Token is not an invitation token."))?;

    let invitation = workspace_member_invitation::lookup_invitation(store, &invitation_id)
        .await?
        .ok_or_else(|| not_found("Invitation not found."))?;

    if invitation.accepted_at_micros.is_some() {
        return ("409 Conflict",
            json!({ "error": "This invitation was already accepted." }).to_string());
    }
    if invitation.expires_at_micros < chrono::Utc::now().timestamp_micros() {
        return ("410 Gone",
            json!({ "error": "This invitation has expired." }).to_string());
    }

    // Find or create the user account for this email.
    let user = user_account::find_or_create_by_email(store, &invitation.email).await?;

    // Insert workspace_member row.
    workspace_member_invitation::accept_invitation(store, &invitation_id, &user.id).await?;

    // Mint a session for the new user.
    let session = session::issue_session_for_user(store, &user.id).await?;

    // Audit.
    let _ = sink.emit(AuditKind::WorkspaceMemberJoined, json!({
        "actor": user.id,
        "tenant_id": invitation.tenant_id.to_string(),
        "user_id": user.id,
        "role": invitation.role,
        "invited_by": invitation.invited_by,
    })).await;

    ("200 OK", json!({
        "user_id": user.id,
        "tenant_id": invitation.tenant_id.to_string(),
        "role": invitation.role,
        "session_cookie": session.cookie_value,   // returned + set as Set-Cookie header
    }).to_string())
}
```

### 3c. PATCH /api/v1/workspace/members/<member_id>

For editing existing member role / scope (referenced from the TODO).

```rust
async fn patch_member_response(member_id: &str, body: &str, ...) -> (&'static str, String) {
    // RBAC: owner/admin only.
    // Body: { role?, cohort_scope? }
    // UPDATE workspace_member ... WHERE tenant_id = ? AND user_id = ?
    // Audit AuditKind::WorkspaceMemberRoleChanged.
}
```

### 3d. DELETE /api/v1/workspace/members/<member_id>

For removing a member.

```rust
async fn delete_member_response(member_id: &str, ...) -> (&'static str, String) {
    // RBAC: owner only (admins can't remove other admins / owners).
    // Forbid removing the LAST owner — return 409.
    // DELETE FROM workspace_member WHERE ...
    // Audit AuditKind::WorkspaceMemberRemoved.
}
```

### 3e. GET /api/v1/workspace/members

Returns list of members for the tenant, including pending invitations.

---

## Phase 4 — Audit kinds

Add 4 new variants (4-site recipe each):
- `WorkspaceMemberInvited` — label `"workspace.member.invited"`
- `WorkspaceMemberJoined` — label `"workspace.member.joined"`
- `WorkspaceMemberRoleChanged` — label `"workspace.member.role_changed"`
- `WorkspaceMemberRemoved` — label `"workspace.member.removed"`

Coordinate with sibling brief `web-audit-kinds-for-journey-events`.

---

## Phase 5 — Email template

**File**: `crates/argo-email-templates/templates/workspace_invite.html` (new)

Mirror the `magic_link.html` template structure. Subject: "You've been
invited to {workspace_name} on SKYCOT". Body:
- "{inviter} invited you to join {workspace_name} as {role}."
- Click the button to accept.
- Expires in 7 days.

Inline CSS for email compatibility; use the brand gradient + Audiowide
font name (with fallback) per the brand style guide.

---

## Phase 6 — Web wire-up

### 6a. API client

**File**: `web/lib/api/workspace-members.ts` (new)

```ts
export interface InvitePayload {
  email: string;
  role: "owner" | "admin" | "user" | "guest";
  cohort_scope: string[];
}
export async function inviteMember(p: InvitePayload) {
  return apiPost<{ invitation_id: string; expires_at: number; email_sent_to: string }>(
    "/api/v1/workspace/members/invite", p
  );
}
export async function listMembers() {
  return apiGet<{ members: Member[]; pending_invitations: Invitation[] }>(
    "/api/v1/workspace/members"
  );
}
export async function patchMember(id: string, patch: { role?: string; cohort_scope?: string[] }) {
  return apiPatch(`/api/v1/workspace/members/${id}`, patch);
}
export async function removeMember(id: string) {
  return apiDelete(`/api/v1/workspace/members/${id}`);
}
export async function acceptInvitation(token: string) {
  return apiPost("/api/v1/workspace/members/accept", { token });
}
```

### 6b. Page rewrite

**File**: `web/app/(app)/settings/members/page.tsx`

Replace seed-data + setState stubs with TanStack Query:

```diff
- const [members, setMembers] = useState<SeedMember[]>(SEED_MEMBERS);
+ const membersQuery = useQuery({
+   queryKey: ["workspace", "members"],
+   queryFn: listMembers,
+ });
+ const members = membersQuery.data?.members ?? [];
+ const pending = membersQuery.data?.pending_invitations ?? [];

  const handleInvite = async (payload: InvitePayload) => {
-   const newMember: SeedMember = { ...payload, status: "invited" };
-   setMembers((prev) => [...prev, newMember]);
-   // Follow-up brief wires the real POST to /api/v1/workspace/members.
+   try {
+     const result = await inviteMember(payload);
+     toast.success(`Invitation sent to ${result.email_sent_to}`);
+     membersQuery.refetch();
+   } catch (e) {
+     toast.error(`Couldn't send invitation: ${(e as Error).message}`);
+   }
  };

  const handleEditSave = async (id: string, patch: EditMemberPatch) => {
-   setMembers((prev) => prev.map(...));
-   // Follow-up brief wires the real PATCH ...
+   try {
+     await patchMember(id, patch);
+     toast.success("Member updated");
+     membersQuery.refetch();
+   } catch (e) {
+     toast.error(`Update failed: ${(e as Error).message}`);
+   }
  };
```

### 6c. New `/accept-invite` page

**File**: `web/app/(setup)/accept-invite/page.tsx` (new)

```tsx
"use client";
import { useEffect, useState } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import { acceptInvitation } from "@/lib/api/workspace-members";

export default function AcceptInvitePage() {
  const params = useSearchParams();
  const router = useRouter();
  const token = params?.get("token") ?? "";
  const [state, setState] = useState<"loading" | "ok" | "error">("loading");
  const [errMsg, setErrMsg] = useState<string>("");

  useEffect(() => {
    if (!token) { setState("error"); setErrMsg("Missing token in URL."); return; }
    acceptInvitation(token).then((res) => {
      setState("ok");
      // Session cookie is set by the response — redirect to landing.
      setTimeout(() => router.push("/workers"), 1200);
    }).catch((e) => {
      setState("error");
      setErrMsg(e.message);
    });
  }, [token, router]);

  /* render loading / ok / error UI matching the wizard center-editorial style */
}
```

---

## Phase 7 — Tests

### Backend
`crates/argo-cli/src/handlers/gateway/routes_workspace_members_tests.rs`:
1. `invite_creates_row_and_emits_email_audit`
2. `invite_rejects_invalid_email`
3. `invite_rejects_unknown_role`
4. `invite_rejects_duplicate_pending`
5. `invite_rejected_for_user_role_caller` (RBAC)
6. `accept_token_consumes_invitation_and_creates_member`
7. `accept_expired_token_returns_410`
8. `accept_already_consumed_returns_409`
9. `patch_member_role_updates_and_audits`
10. `delete_last_owner_returns_409`

### Frontend
- Invite modal happy path E2E.
- Accept invite page happy path E2E (with stub token).

---

## may-touch

- `crates/argo-store/migrations/NNN__workspace_member_invitation.sql` (new).
- `crates/argo-store/src/migrations/vNN.rs` (new).
- `crates/argo-store/src/workspace_member_invitation.rs` (new) + tests.
- `crates/argo-cli/src/handlers/gateway/auth_magic_link_token.rs` — extend `TokenPayload`.
- `crates/argo-cli/src/handlers/gateway/routes_workspace_members_invite.rs` (new).
- `crates/argo-cli/src/handlers/gateway/routes_workspace_members_accept.rs` (new).
- `crates/argo-cli/src/handlers/gateway/routes_workspace_members.rs` (new, GET + PATCH + DELETE).
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — wire 5 new arms.
- `crates/argo-audit/src/event.rs` — add 4 variants.
- `crates/argo-email-templates/templates/workspace_invite.html` (new).
- `web/lib/api/workspace-members.ts` (new).
- `web/app/(app)/settings/members/page.tsx` — rewrite to use real API.
- `web/app/(setup)/accept-invite/page.tsx` (new).

## must-NOT-touch

- Existing magic-link login flow — extend the payload, don't change verify.
- Other settings tabs (api-keys, data-export, security) — separate concerns.
- The locked design lock — preserve the visual identity.

---

## Acceptance criteria

- [ ] `POST /api/v1/workspace/members/invite` creates a row AND sends an email.
- [ ] Email contains a working magic-link to `/accept-invite?token=...`.
- [ ] Token expires in 7 days (configurable but default 7).
- [ ] Accepting creates a `workspace_member` row + signs the user in.
- [ ] Re-accepting returns 409.
- [ ] Expired token returns 410.
- [ ] PATCH allows owner/admin to change a member's role (but not their own to non-owner if they're the last owner).
- [ ] DELETE removes a member; refuses to remove the last owner with 409.
- [ ] All 4 new AuditKind variants emit correctly.
- [ ] Web `/settings/members` shows real members from the gateway.
- [ ] `/accept-invite` page works end-to-end.
- [ ] 10+ tests pass.
- [ ] `cargo clippy -p argo-store -p argo-cli -p argo-audit --all-targets -- -D warnings` clean.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

---

## Gotchas

1. **The "last owner" rule**: a workspace must have ≥1 owner. DELETE +
   PATCH-to-non-owner must guard against removing the last one. Return
   409 with clear error message: "Cannot remove the last owner. Promote
   another member to owner first."
2. **Email sending in dev**: there's no SMTP in dev — the brief should
   log the magic-link URL to stdout in dev mode so the operator can
   click-through. Production path uses the existing email provider
   (sendgrid / postmark / whichever is configured).
3. **Race condition on simultaneous accepts**: two browser tabs both
   posting accept with the same token. The `consume` op in
   `auth_magic_link_token` is already atomic (single UPDATE WHERE used_at
   IS NULL); the second consume returns AlreadyUsed → 409.
4. **Email enumeration**: the invite endpoint should NOT reveal whether
   the invitee email already has a user_account. Always return
   `email_sent_to` regardless. Account creation happens lazily on
   accept.
5. **Cohort scope validation**: when role is owner/admin, cohort_scope is
   ignored (those tiers always see everything). When role is user/guest,
   cohort_scope can be empty (means "no cohorts visible — minimal
   default"). Surface this in the invite modal copy.
6. **Magic-link token payload growth**: embedding `invitation_id`
   increases token size slightly. Keep token under 2KB total so it fits
   in URL params without truncation by mailer / reverse-proxy.
7. **Email template render**: the `crates/argo-email-templates` crate has
   a helper for inlining CSS. Use it — Gmail / Outlook strip `<style>` blocks.

---

## References

- Existing magic-link substrate: `crates/argo-cli/src/handlers/gateway/auth_magic_link_*.rs`
- Magic-link token migration: `crates/argo-store/migrations/115__magic_link_tokens.sql`
- Members page: `web/app/(app)/settings/members/page.tsx`
- RBAC sibling: `web-rbac-jwt-role-claim-and-gateway-enforce`
- Audit kind sibling: `web-audit-kinds-for-journey-events`
- Audit: `docs/skycot/audit/api-wiring/2026-05-20-wave-3-api-wiring.md` (member-PATCH TODO)
