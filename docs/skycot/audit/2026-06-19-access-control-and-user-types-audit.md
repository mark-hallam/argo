# Access-Control & User-Types Audit — 2026-06-19

**Question (operator):** "Does the platform have admin / different user types defined,
and the appropriate access controls implemented? Properly define what the platform
actually needs, and check the current state."

**Method:** three parallel code-grounded investigations (Rust core, web SPA + gateway,
membership + billing). Read the code, not the docs. Every claim below is tagged
**VERIFIED** (read the deny path + a live call-site) or **GAP** (defined-but-not-wired,
or absent). Supersedes the stale 2026-05-20 sweep note "RBAC is copy-not-enforcement"
— that is no longer true.

---

## Part 1 — What the platform ACTUALLY NEEDS (first-principles reference model)

A live, multi-tenant commercial SaaS with a self-serve tier and an enterprise tier needs
**two planes of identity**, not one:

### Plane A — the customer plane (inside a single workspace/tenant)
Roles a paying customer's own org needs:
1. **Owner** — billing authority + can delete the account. The buyer.
2. **Admin** — runs the workspace (hire/manage/configure) but not billing or deletion.
3. **Member / User** — does the work (runs teammates), no admin surfaces.
4. **Viewer / Guest** — read-mostly / shared-link recipient.
5. **Lifecycle on a member**: invite, accept, **change role, remove**.

### Plane B — the platform-operator plane (Skycot's own staff, cross-tenant)
Roles Skycot-the-company needs to run a paid product:
6. **Support / staff** — see (and, with consent + audit, act inside) a customer workspace
   to resolve a ticket. Time-bounded, fully audited, capability-gated. *Not a back door.*
7. **Billing-ops** — reconcile a payment / refund / dunning issue.
8. **Super-admin** — suspend an abusive or non-paying tenant; legal/compliance hold.

### Cross-cutting controls every plane depends on
- **Authentication** (prove who you are).
- **Authorization** (what you may do) — enforced **server-side**, never trusting the UI.
- **Tenant isolation** (workspace A can never see workspace B) — ideally structural.
- **Account lifecycle** (active / suspended / closed) — actually enforced.
- **Audit trail** on every privileged action (especially Plane B staff access).

---

## Part 2 — Current state vs. that model

### ✅ Customer plane — built and ENFORCED (better than expected)

| Need | State | Evidence |
|---|---|---|
| Owner / Admin / User / Guest roles | **VERIFIED** | `UserTenantRole` enum `crates/argo-identity/src/user_tenant.rs:54`; one row per `(user_id, tenant_id)` so a user can hold different roles in different workspaces |
| Role read at login, carried in session | **VERIFIED** | role read from `user_tenant` row, stamped into session token `auth_magic_link_verify.rs:219-249` |
| Server-side authorization (not cosmetic) | **VERIFIED** | `enforce_role(req, Action)` gate, 24-action closed enum, role taken from the **verified token** not a client header, **fails closed to Guest**, ~35–37 live call-sites, returns real `403` — `crates/argo-cli/src/handlers/gateway/role_gate.rs:173,297,328` |
| Capability-based control on the agent runtime | **VERIFIED** | `CapabilityEngine::check` on the tool-dispatch seam — *every* agent tool call is checked before it runs; default-deny — `crates/argo-capability/src/lib.rs:230`, `crates/argo-tool/src/registry.rs:188` |
| AI workers can't self-escalate | **VERIFIED** | DEC-041(D) fence: a worker session binds to a deny-all / least-privilege profile, never `owner` — `crates/argo-agent-runner/src/dispatch.rs:98,146-165`, wired `turn_helpers_run.rs:184-194` |
| Multi-user workspace + invite flow | **VERIFIED** | invite→email→accept→membership link, owner/admin-gated, backend + SPA — `routes_workspace_members_invite.rs`, `routes_workspace_members_accept.rs`, table `migrations/127__workspace_member_invitation.sql` |
| Plan tiers + entitlements enforced | **VERIFIED** | free / pro / enterprise, worker-cap + feature gates fail-closed at runtime — `tier_gate.rs:52,84`; tiers in `docs/skycot/pricing/v1-tiers.toml` |
| Tenant isolation | **VERIFIED (fragile)** | tenant derived from verified session token; ~401 hand-written `WHERE tenant_id = ?` clauses with negative tests — `resolve_tenant.rs:61`. No single central guard. |

### ❌ Gaps — what a live platform needs and does NOT have

| Gap | Severity | Detail |
|---|---|---|
| **No platform / super-admin plane at all** | **HIGH** | Exhaustive grep for `super_admin / platform_admin / staff / impersonat / backoffice` → **zero**. The highest role is tenant-scoped `Owner`. Skycot staff have **no audited way** to help a stuck customer, suspend a fraudster, or handle a billing/legal issue — only raw database access. For a live paid product this is the load-bearing gap. |
| **Can't change a member's role or remove a member** | **HIGH** | PATCH (role change) + DELETE (remove member) explicitly deferred/unbuilt — `routes_workspace_members.rs:14-18`. You can on-board but not off-board or demote. Blocks any real org with staff churn. |
| **Account suspend not enforced** | **MEDIUM** | `tenant.status` column (`active`/`suspended`) exists (`migrations/106__tenant.sql`) but no gateway path enforces a suspended-tenant lockout. The off-switch is wired to nothing. |
| **Tenant isolation is per-query, not structural** | **MEDIUM** | Correctness depends on ~401 query sites each being written right; one forgotten `WHERE` = a cross-tenant leak. Fine today (tested), fragile at scale. First-principles fix: a store-layer choke-point that *requires* a `TenantId` so the unsafe query is unspendable. |
| **Two auth systems not unified** | **MEDIUM** | Gateway role-floor (`enforce_role`) and runtime `CapabilityEngine` are separate; a new sensitive route shipped without an `enforce_role` line would be ungated. Needs a router-level default-deny. |
| **No SSO login (SAML/OIDC)** | **MEDIUM (enterprise)** | SCIM 2.0 user-provisioning exists (`scim_user`, bearer-gated), but the actual single-sign-on *login* path is not built; `sso` is only a tier flag. Enterprise buyers will require it. |
| **SPA is "reach the page, then 403"** | **LOW (UX, not a hole)** | Web route gating is binary authenticated/anonymous; a Guest can load the `/billing` page shell and only gets 403 on the data call. Enforcement is correct; the UX should hide the surface by role. |
| **Members page shows demo names on empty fetch** | **LOW (theater)** | `SEED_MEMBERS` fallback in `web/app/(app)/settings/members/members-data.tsx`. NO-THEATER (DEC-029) — replace with an empty state. |

---

## Part 3 — Recommendation (priority order, first-principles)

1. **Decide the staff-access story deliberately (operator decision).** This is the one
   genuine product/trust call. Options:
   - (a) **Zero staff cross-tenant access** — strongest privacy story, but no support/abuse ability.
   - (b) **Capability-gated, time-bounded, fully-audited staff console** with explicit
     impersonation + customer-visible audit row. *Recommended* — it's the version you
     don't have to rebuild, and the audit trail *is* the trust story.
   - (c) Break-glass DB access only, documented — cheap now, becomes a liability.
2. **Build member off-boarding** (role-change + remove). Small, blocks real orgs.
3. **Enforce `tenant.status` suspend** — wire the lockout the column already implies.
4. **Harden tenant isolation into a structural choke-point** before scale.
5. **Add a router-level default-deny** so no future route ships ungated.
6. **SSO login** when the first enterprise deal needs it (SCIM half already exists).
7. Clean up the two LOW items (role-aware SPA gating, kill `SEED_MEMBERS` fallback).

---

*All findings code-grounded this session; no files edited during the audit.*
