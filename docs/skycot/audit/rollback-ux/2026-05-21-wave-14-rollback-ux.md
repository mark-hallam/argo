# Wave-14 Audit — Rollback + Version-History UX

**Audit window**: 2026-05-21
**Auditor**: claude (Wave-14 audit agent #4)
**Scope**: rollback UI / version-switcher UX backing the landing-page promise *"Rollback shipping next"*
**Promise audited**: `web/app/landing/page.tsx:46` — *"Every workflow change versioned like code. Side-by-side diff. Rollback shipping next."*
**Wave-13 baseline**: `docs/skycot/audit/diff-versioning/2026-05-21-wave-13-diff-versioning.md` — found rollback 0/10 (DB ready, no UI).

---

## TL;DR

| Dimension | Status | Score |
| --- | --- | --- |
| DB rollback operation | **MISSING** — `argo-store::workflow_definition` has `create` / `set_head` / `archive` but no `rollback_to(version)` | 3 / 10 (set_head composes) |
| Gateway endpoint | **MISSING** — `POST /workflows/<id>/rollback` not in `routes_workflows.rs` route table | 0 / 10 |
| Web UI button | **MISSING** — no rollback CTA on `/workflows/<v>/<n>` viewer or version-list rows | 0 / 10 |
| Confirmation modal | **MISSING** — no `<RollbackConfirmModal>` component exists | 0 / 10 |
| Audit row on rollback | **MISSING KIND** — `AuditKind::WorkflowRolledBack` does not exist; would emit via existing `WorkflowHeadChanged` if rollback piggy-backs on `set_head`, but no `rolled_back: true` discriminator | 2 / 10 |
| Permission gate | **READY** — `Action::WorkflowEdit` → `UserTenantRole::Admin` floor already in `role_gate.rs:135`; rollback would inherit this gate | 9 / 10 |
| Marketing copy posture | **HONEST** — "Rollback shipping next" softened by wave-5; not a lie until shipped | 7 / 10 |
| Brief quality | **STRONG** — `web-workflow-rollback-feature.md` is clear, sized 500-800 LOC, names files, has acceptance criteria | 9 / 10 |
| Brief claim status | **ZOMBIE-DEMOTED 2026-05-21** — was CLAIMED 2026-05-20, demoted by conductor because 5-worker fleet ≠ 10 CLAIMED rows; available for re-claim | needs queue replenishment |

**Overall rollback-UX score: 1.5 / 10** — substrate composes from `set_head` but the dedicated rollback surface (op, endpoint, UI, modal, audit kind, tests) is all unshipped.

---

## File inventory

### Existing substrate (composable)
- `crates/argo-store/src/workflow_definition.rs` — types + module split.
- `crates/argo-store/src/workflow_definition_write.rs` — `create` / `set_head` / `archive`. **`set_head` can re-point head to an OLDER version today**; this is a head-pointer swap, not an additive new-version rollback.
- `crates/argo-store/src/workflow_definition_read.rs` — `get` / `get_head` / `list_versions`.
- `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — 7 routes mounted: List / Head / Versions / Version / Create / SetHead / Archive. Auth via `X-Argo-Principal` + `X-Argo-Role` headers; `WorkflowEdit` capability gate already in place.
- `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs` — emit-audit helper exists; reused on POST / PUT head / DELETE.
- `crates/argo-cli/src/handlers/gateway/role_gate.rs:135` — `Action::WorkflowEdit` → `UserTenantRole::Admin` floor.
- `crates/argo-audit/src/event.rs:237-251` — 3 workflow audit kinds: `WorkflowDefinitionCreated` / `WorkflowHeadChanged` / `WorkflowVersionArchived`.

### Web pages today
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` — 946 LOC visual viewer (buyer demo path). Static seed `version: "v6"` text on line 749. **No rollback affordance; no versions dropdown; no version-switching.**
- `web/app/(app)/verticals/[verticalId]/workflows/[name]/page.tsx` — 161 LOC list view at the orphan URL. Renders newest-first version list with `is_head` badge + `created_at` + "Compare with head" link per row. **No rollback button.**
- `web/app/(app)/verticals/[verticalId]/workflows/[name]/diff/page.tsx` — side-by-side diff route. **No rollback button on the diff page either**, which is the natural "rollback to the older side of the diff" CTA.

### Briefs status
- `docs/build-plan/streams/web-workflow-rollback-feature.md` — TODO **ZOMBIE-DEMOTED 2026-05-21** (BUILD_STATUS.md row 920). Brief is well-formed: DB op + gateway endpoint + modal + audit row + tests; 500-800 LOC. Ready to re-claim.
- `docs/build-plan/streams/mission-pm-workflow-rollback-cli.md` — CLAIMED by codex 2026-05-19 (row 848); covers CLI surface (`argo workflow rollback <v> <n> --to <id>`), NOT the web surface. Different deliverable.
- `docs/build-plan/streams/claude-skill-rollback-cli.md` — orthogonal (skill rollback, not workflow rollback).

### Audit-row coverage
- Existing emissions in `routes_workflows_handlers.rs:152, 199, 244` cover create / head-changed / archived.
- **A rollback piggy-backed on `set_head` would already emit `WorkflowHeadChanged`**. But the audit row would not say "this was a rollback" vs. "this was a forward-promotion of a draft" — both currently look identical in the audit feed. A dedicated `WorkflowRolledBack` kind (or `is_rollback: true` payload field) is the right call for forensic clarity. Brief is silent on this — flag as omission.

---

## The 5 dimensions

### 1. Rollback button — where would it live?
Three candidate sites:

**A. Versions list row (`/verticals/<v>/workflows/<n>` table row)** — natural "I see the version I want, give me the action right there" pattern. Currently each row has "Compare with head"; would add "Rollback to v<N>" as a second button. **Best for power-users.**

**B. Viewer header (`/workflows/<v>/<n>` actions bar)** — alongside "Open builder", "Version history", "Test run". Operator currently has 3 action buttons here; adding "Rollback…" opens a chooser modal listing prior versions. **Best for the buyer-demo path** because they don't have to leave the visible viewer.

**C. Diff page (`/verticals/<v>/workflows/<n>/diff?from=X&to=Y`) "Rollback to from" button** — most contextual; the user has already seen the diff between the older "from" and current "to", so the rollback button on this page reads as "make `from` the new head, including the diff you just looked at." **Best for safety** because the operator has just visually confirmed what they're committing to.

Recommendation: **ship all three**, with the *diff page* button as the primary CTA (because it forces the operator to look at the diff before rollback). The version-list row button is a one-tap shortcut. The viewer-header dropdown is the marketing-promise-satisfier ("right there on the viewer where I demo from"). The brief's plan ("every prior version row has a Rollback to v<N> button") covers A but is silent on B and C.

### 2. Confirmation modal — shows diff vs current before confirming?
The brief specifies: *"confirmation modal showing diff vs current → confirm → API call"*. This is the right design — reuse `<WorkflowDiff>` (`web/components/skycot/WorkflowDiff.tsx`) inside the modal. The brief calls this out: *"reuse if a diff component exists, else add a simple side-by-side."* Today the diff component is already shipped (Wave-13 confirmed).

Concerns:
- **The modal's confirm copy must be unambiguous**: "This will create a NEW version (v8) that exactly matches v3. The current v7 stays in history." Anything weaker risks users believing v7 has been destroyed. (This connects to dimension 3 below.)
- **The modal must show a single-line summary above the diff** for buyers who don't want to read 80 lines of TOML before confirming. Example header: *"You're about to rollback `triage_ticket` from v7 to v3. The diff below shows what will change."*
- **Esc to cancel + Enter to confirm** — the brief calls for focus trap + Esc + Enter; this matches a11y standard.

### 3. New version after rollback — additive or head-pointer swap?
The brief is unambiguous: **additive** — *"rollback creates a new version that's an exact copy of the named older version (additive, never destructive)"*. This is the correct call:
- **No version row is ever destroyed**, so the audit chain stays intact.
- **`set_head` could technically just point to v3 again** (the existing primitive supports this), but doing so creates a confusing version history (head jumps backward in the timeline, monotonic version numbers no longer match the head sequence). A new v8 that is a content-copy of v3 keeps the timeline linear and the head always points to the highest version number.
- **Trade-off**: storage cost — every rollback duplicates a TOML blob. For workflows up to 100KB and v1.0 traffic patterns, this is irrelevant. Compression / dedup is a v1.x problem.

Brief is correct. The DB op signature should be: `WorkflowStore::rollback_to(tenant, vertical, name, target_version_id) -> WorkflowDefinitionId` — computes next version number, INSERTs row with `definition_toml` copied from target + `parent_id = target_version_id`, then calls `set_head` on the new id atomically.

### 4. Audit row — emitted on rollback?
The brief calls for `kind: "workflow_rolled_back"` to be written to the chain. **Today no such `AuditKind` exists.** Three options:

**(a) Add a new `AuditKind::WorkflowRolledBack` variant** to `crates/argo-audit/src/event.rs:237-251`. Brief implies this but doesn't state it. **Recommended** — gives forensic clarity ("this head change was a rollback, not a forward promotion"). 30 LOC in event.rs + 2 tests.

**(b) Reuse `AuditKind::WorkflowHeadChanged` + extra payload field** `"is_rollback": true, "rolled_back_from_version_id": "..."`. Cheaper. Risk: payload extension means audit-feed UI must parse the field to badge rows correctly, and the existing payload contract is documented as 5 fields exactly.

**(c) Emit TWO rows: `WorkflowDefinitionCreated` (for the new copy-version row) + `WorkflowHeadChanged` (for the head flip).** This is what the brief's design naturally produces if rollback just calls `create` + `set_head`. Most truthful but audit-feed-noisy.

Recommendation: **(a) plus (c)** — emit the natural pair PLUS a third `WorkflowRolledBack` event that wraps both with the cross-version delta. Brief should be updated to specify the audit-kind addition.

### 5. Permission gate — admin-only?
The existing `Action::WorkflowEdit` floor is `UserTenantRole::Admin` (`role_gate.rs:135`). Rollback would inherit this — owner + admin can rollback, user + guest cannot. **This is correct for v1.0** — rollback is a destructive-feeling op (even though it's additive), and matches the "members can read, admins can edit" mental model the rest of the SPA enforces.

Brief is silent on the permission gate explicitly — it doesn't need to be, because the rollback handler will call `enforce_role(req, Action::WorkflowEdit)` exactly like `create_version` / `set_head` / `archive_version` do. Confirm in implementation: the rollback handler must call `enforce_role(req, Action::WorkflowEdit)` before any DB work.

---

## Risks of NOT shipping rollback

### Risk 1 (CRITICAL) — Buyer-catchable broken promise
The landing-page copy currently reads *"Rollback shipping next"* (softened from "One-tap rollback" by wave-5 close-the-loop). This is honest *today* but every week it stays unshipped degrades trust. A procurement-stage buyer reading "shipping next" expects shipped within weeks, not months. By the time the v1.0 demo recording lands, "Rollback shipping next" reads as either (a) a stale promise the team forgot, or (b) a roadmap item that didn't survive the cut. Either reading is a deal-killer for risk-averse buyers (regulated industries, finance, healthcare) who are evaluating SKYCOT against competitors that *do* ship workflow rollback today (n8n, Zapier paid plans, Pipedream).

### Risk 2 (HIGH) — Safety-net for the edit-api is missing
The edit-api shipped (mission-pm-workflow-edit-api MERGED, PR #1860). Operators can now edit workflows via the gateway and `POST` new versions. **There is no recovery path from a bad edit today** other than uploading the TOML by hand or running `argo workflow rollback` (CLI, still CLAIMED by codex per row 848). For a web-first surface that markets "version like code", the absence of an in-product undo is the load-bearing gap. The first time a customer ships a bad edit to production and the only recovery is a CLI call to a database that they can't access, the support ticket writes itself: "we trusted your versioning promise, where's the rollback button?"

### Risk 3 (MEDIUM) — Existing audit-row coverage becomes misleading
Today the audit feed shows `WorkflowHeadChanged` rows uniformly — every head change looks the same. Once rollback ships via a separate kind, forensic clarity improves. **Until rollback ships, the audit feed has a blind spot**: every "this workflow was rolled back" event in the future will need to be backfill-tagged from `WorkflowHeadChanged` rows with no discriminator. The longer rollback stays unshipped, the more rows accumulate in the "head changed but for unknown reason" bucket, and the harder it becomes to retroactively label them when the discriminator finally lands. Audit-trail debt compounds.

### Risk 4 (MEDIUM, secondary) — `mission-pm-workflow-rollback-cli` lands first without UI parity
Codex has the CLI brief CLAIMED at row 848. If the CLI ships before the web rollback, operators (admins running `argo workflow rollback`) will write to `WorkflowHeadChanged` audit rows with no UI surface to render the rollback context. The audit page (Etherscan-style, brand-locked 2026-05-18) will need to special-case rollback rows. Sequencing: ship the audit-kind addition (option (a) above) **before** either CLI or web rollback to avoid the "audit row exists but no label" interim state.

---

## Recommended sequencing

1. **NOW**: Author a tiny brief `audit-kind-workflow-rolled-back` — add `AuditKind::WorkflowRolledBack` variant + label + parse + tests. 80-120 LOC, owner=claude. Blocks both CLI and web rollback so the rollback signal is in the chain on day one.

2. **NEXT (parallel)**:
   - **Codex** finishes `mission-pm-workflow-rollback-cli` (CLAIMED row 848). Brief is solid; deps both MERGED 2026-05-20.
   - **Claude** re-claims `web-workflow-rollback-feature.md` (zombie-demoted 2026-05-21, available). Brief is excellent; only update needed is to depend on the audit-kind brief from step 1 and emit the new kind.

3. **THEN (Wave-14 sweep)**: Patch the viewer page (`web/app/(app)/workflows/[verticalId]/[name]/page.tsx`) to add the versions dropdown (Wave-13 F1) so rollback's "Rollback…" CTA lives on the buyer-demo path, not just the orphan version-list URL.

4. **FINALLY**: Revert landing copy `web/app/landing/page.tsx:46` from *"Rollback shipping next"* back to *"One-tap rollback"* in the merge commit of the web feature PR. (Brief calls this out as a sub-bullet of acceptance criteria.)

---

## 250-word executive summary

Rollback is the highest-leverage unshipped P1 of Wave-14. The DB substrate composes from existing primitives (`set_head` can technically re-point head; brief specifies the cleaner additive `rollback_to` op that copies the target version's TOML into a new version row, preserving the linear timeline). The gateway endpoint, the modal, the UI button, and the dedicated `WorkflowRolledBack` audit kind are all unshipped. The brief (`web-workflow-rollback-feature.md`) is well-formed: 500-800 LOC, clear file plan, acceptance criteria, audit-row plan — only gap is silence on adding the new audit-kind variant. The brief was zombie-demoted 2026-05-21 (5-worker fleet vs 10 CLAIMED rows) and is available to re-claim today. Permission gating is ready: `Action::WorkflowEdit → Admin floor` already enforced; rollback inherits without new wiring. Best button placement is on the diff page (forces eyes-on-diff before confirm), the version-list row (power-user shortcut), AND the viewer header (the buyer demo path). Sequence: ship audit-kind first (80-120 LOC), then web rollback + CLI rollback in parallel, then wire the versions dropdown into the buyer-facing viewer so rollback is reachable from the demo path, then revert landing copy from "Rollback shipping next" to "One-tap rollback." Top 3 risks of NOT shipping: (1) buyer-catchable broken promise in regulated-industry demos, (2) edit-api shipped without an in-product recovery path so a single bad edit needs CLI access, (3) audit-feed accumulates indistinguishable head-changed rows that can't be retroactively labeled as rollback once the discriminator finally lands.
