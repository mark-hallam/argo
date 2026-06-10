# fix-manual-run-audit-row-kind-status-and-principal

**Owner**: claude or codex.
**Depends on**: none.
**Authored against**: origin/main `02837c942` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-manual-run-audit-row-kind-status-and-principal

## Goal

A "Run now" on a worker must land in the audit trail as what it IS — a
worker run, attributed to the signed-in operator, with the run's REAL
outcome — not as an `http.request · Failed` row by "anonymous" while the
engine logged `ok:true`.

## Why now (live e2e, observed BOTH 2026-06-10 and 2026-06-11)

Two successful manual runs (Casey #2798-era, Chase post-DEC-048; e.g. action
`019eb32cc8747713923b53726e04b1c7`, engine log `{"ok":true,"steps_completed":1}`)
surfaced in /audit as **kind `http.request`, badge `Failed`, decision chain
"Run triggered by anonymous via manual_run_now"** — three honesty defects on
the platform's flagship surface:

1. **Kind**: the row is emitted as generic `AuditKind::HttpRequest`
   (`routes_worker_run.rs:288`) — `/audit` even meta-filters `http.request`
   rows by default (#2661), so the flagship "watch your teammate work" row is
   both mislabeled AND hidden from the default feed.
2. **Status**: payload `status` is set from the outcome (`routes_worker_run.rs:147,254,271`)
   but the rendered badge shows Failed for a successful run — the feed reads
   `outcome_status(p)` with a `status` fallback (`routes_audit_feed.rs:331-332`)
   and the detail normalizer defaults to `'failed'`
   (`web/lib/api/audit-detail.ts:171`); localize where the success gets lost
   (payload shape vs reader) and fix at the emit so readers need no guessing.
3. **Actor**: `principal_id_for_audit` returns `None` without an
   `X-Argo-Principal` header (`routes_auth_principal.rs:81-83`) → actor
   "anonymous" (`routes_worker_run.rs:115`) even though the operator is
   signed in with a `__Host-session` JWT. Attribution must come from the
   SESSION (server-resolved, same precedence discipline as
   `resolve_tenant_for_request` — never trust a client header over the JWT).

## Fix

1. Emit the manual-run action with a run-specific identity: keep the
   existing kind enum surface (extend `AuditKind` only if a variant like
   `WorkerRun` doesn't already exist — grep first) OR stamp
   `action_type: "manual_run"` + endpoint so the feed/detail render "Worker
   run" not "http.request", and the default /audit meta-filter does NOT hide
   it.
2. Stamp the payload's `outcome_status` explicitly from the engine outcome
   (completed/failed) so the two-tier reader is unambiguous; the detail
   page badge must match the engine result (no `'failed'` default winning on
   a successful run).
3. Resolve the actor server-side from the session JWT principal (mirror the
   WS1 JWT-first pattern; `X-Argo-Principal` stays as the no-session
   fallback only). "anonymous" remains ONLY for genuinely anonymous calls.

## May touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_run.rs` (367 LOC) +
  `_tests`
- `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs` (180 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` (reader only
  if emit-side fix leaves ambiguity)
- web render layers only if labels need a case added
  (`AuditTableRow.tsx:189-194`, `ActionSummaryHero.tsx:224-230`)

## Acceptance / verifier

1. Test: successful mocked run → audit row `outcome_status == "completed"`,
   rendered tone NOT failed; failed run → failed (both directions pinned,
   `routes_worker_run_tests.rs`).
2. Test: signed-in session POST /run → actor == the session principal (not
   "anonymous"); no-session POST → "anonymous" preserved; a forged
   `X-Argo-Principal` header does NOT override the JWT (negative pin).
3. The run row appears in the DEFAULT /audit feed (not meta-filtered) titled
   as a worker run.
4. `cargo test -p argo-cli` (touched modules) + clippy green; no new pub
   types without grep-first.

## Size / owner / priority

~80-150 LOC incl. tests. **🟠 P2 — audit-trail honesty on the flagship
surface** (DEC-029/NO-THEATER class: a TRUE "it worked" shown as Failed by
"anonymous" is the same credibility failure mode inverted). No collisions:
#2661 touched query-side filtering only; no in-flight briefs on this emit
path (verified 2026-06-11).
