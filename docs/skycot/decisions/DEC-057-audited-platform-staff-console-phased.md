# DEC-057 — Audited platform-staff console (Phase 1 now, Phase 2 later)

- **Status:** active
- **Date:** 2026-06-19
- **Operator call:** "do phase 1 now, do phase 2 later. record decision."
- **Decides:** the staff-access trust model surfaced by the 2026-06-19
  access-control audit (`docs/skycot/audit/2026-06-19-access-control-and-user-types-audit.md`).
- **Relates to:** [DEC-029](DEC-029-truth-bounded-status-reporting.md) (hash-chained
  audit / NO THEATER — the spine this reuses), [DEC-041](DEC-041-execution-posture-risk-tiered-act-by-default.md)
  (least-privilege / no privilege-smear), the `elevated_view_grant` time-bounded
  grant primitive (the Phase-2 model).

## Context

The audit verified the **customer plane** (Owner/Admin/User/Guest within a tenant)
is real and enforced. It found the **platform-operator plane absent**: Skycot's own
staff have **no audited way** to suspend an abusive/non-paying tenant, reconcile a
billing issue, or support a stuck customer — the only path today is raw database
access. For a live, paid, multi-tenant product this is the load-bearing gap.

Four options were considered: (a) audited staff console, (b) zero staff access,
(c) break-glass-DB-only, (d) decide later. The operator asked for the best long-term
direction; the recommendation was (a), phased so the cheap high-value half ships first.

## Decision

Build a **first-class, default-deny, fully-audited platform-staff identity** — a
distinct *kind* of principal that lives **outside** any single tenant and has **no
standing access to anyone**. Ship it in two phases.

### Phase 1 — NOW (account-level only, no workspace entry)

A platform-staff principal that can:

1. **Read account-level status across tenants** — id, name, tier, lifecycle status,
   member/worker counts. **NOT** workspace contents (no messages, audit bodies,
   documents, or teammate internals).
2. **Suspend / reactivate a tenant account** — and the suspend is **actually
   enforced**: a suspended tenant's tenant-scoped requests are rejected
   (`403 account_suspended`), wiring the lockout the existing `tenant.status`
   column only implied. (This also closes the audit's "suspend not enforced" gap.)
3. Every staff action writes a **hash-chained audit row to the affected tenant's
   own chain**, so the customer can see "your account was suspended by Skycot staff
   at <time>." The audit trail IS the trust guarantee.

Authorization rules (non-negotiable, first-principles):
- Staff status is **default-deny** — a normal user is never staff.
- The staff role is resolved from the **verified session token's `sub`** → a
  `platform_staff` lookup; **never** a client-supplied header, never standing
  owner-of-every-tenant.
- Staff are seeded only via a deliberate, audited CLI (`argo platform staff …`).

### Phase 2 — LATER (consent-based in-workspace access)

Add **time-boxed, customer-visible "act inside a workspace"** (impersonation), modelled
on the existing `elevated_view_grant`: a staff member requests a scoped, expiring grant
into **one named tenant**; the customer sees it; every in-workspace action is audited.
Un-defer trigger: the first real customer support engagement that cannot be resolved at
the account level.

## First-principles rationale

1. **Staff access is inevitable — design it or improvise it.** "Zero access" decays
   into unaudited break-glass under the first 2am incident. The only stable end-state
   is a governed path built before it's needed.
2. **The audit trail is the product, not overhead.** SKYCOT already ships hash-chained,
   truth-bounded audit. Pointing it at our own staff converts a liability into a
   stronger enterprise trust story than "we can never help you."
3. **It composes with existing primitives** (capability grants, `elevated_view_grant`,
   role gate, audit chain) — mostly wiring, not new machinery.
4. **A staff principal is a different identity kind**, not "owner of every tenant."
   Modelling it as super-owner would be exactly the privilege-smear DEC-041 forbids.

## Consequences / honest cost

- Phase 1 is real engineering (additive migration + identity type + store + a staff
  gate + platform routes + suspend-lockout choke-point + seeding CLI + tests). It is
  the largest of the audit's gaps, but Phase 1 is bounded and ships abuse-control +
  billing-ops immediately without the hardest part (in-workspace impersonation).
- New externally-reachable surface (`/api/v1/platform/*`) — must be default-deny and
  fail-closed; covered by negative tests (non-staff → 403).
- Phase 2 is explicitly deferred; no in-workspace staff visibility exists until it lands.

## Index line

Added to `docs/skycot/decisions/README.md`.
