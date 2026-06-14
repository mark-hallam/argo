# fix-approvals-probation-rerun-rebirths-hold-counter-never-converges

**Owner**: claude — **P2 DATA-CONSISTENCY / UX (e2e soak finding).**
**Depends on**: none.
**Authored against**: origin/main `55c215e4a9d4989301146de899302057b1c456c9` on `2026-06-14`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-approvals-probation-rerun-rebirths-hold-counter-never-converges

Source: docs/skycot/audit/2026-06-14-e2e-soak-15x.md — the single substantive
defect the 15x soak surfaced (agent 2/5, reproduced across all 3 of its passes:
E2EJun14-04, -05, -06).

## Problem

The `/approvals` PENDING counter **drifts UP on page reload instead of
converging to 0**, and two of the three reproductions never reached a verified
PENDING 0 at all. The hash-chained audit source-of-truth stays clean (0
pending/refused/failed) and each Approve click still decrements exactly 1 per
settled click at a 100% rate — so the underlying approvals are CORRECT. The bug
is purely the inbox COUNTER not converging. It is a real UX / data-consistency
defect, not a nit: the operator can clear the inbox and watch it re-fill, and
two of three soak passes could not drain it to a trustworthy 0.

Soak repro (PENDING count observed across successive reloads, per the audit
scorecard):

- **P05 (E2EJun14-05)**: `2 → 3 → 5 → 6 → 13` on reloads — NOT drained to a
  verified 0.
- **P06 (E2EJun14-06)**: `2 → 3 → 5 → 10` on reloads — NOT drained to a
  verified 0.
- **P04 (E2EJun14-04)**: first showed 3, then 14 on reload; only reached a true
  0 after ~30 clicks (brute-drained, not converged).

### Root cause (code-grounded — file:line)

The `/approvals` listing unions three arms into one count
(`crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs:122-201`). The
third — the **probation-hold arm** — is where the count is born and where it
fails to converge:

1. `list_response` calls `routes_approvals_probation::fetch_probation_hold_rows`
   (`routes_approvals_json.rs:180-201`).
2. `fetch_probation_hold_rows`
   (`crates/argo-cli/src/handlers/gateway/routes_approvals_probation.rs:60-88`)
   reads every `probation.hold` audit row for the tenant and treats each as
   PENDING **unless** its `(session_id, tool)` key appears in
   `resolved_hold_keys` — the set of `approval.granted`/`approval.declined`
   markers stamped with `probation_hold:true` + the original `session_id` +
   `tool`.
3. Approving a `phold:` row runs `decide_probation_hold`
   (`routes_approvals_probation.rs:239-277`). It writes the resolution marker
   keyed to the **old** `(session_id, tool)` (`write_resolution`, lines
   282-314) **and then re-runs the teammate** via `rerun_with_tool` →
   `routes_worker_run::run_worker_now` (`routes_approvals_probation.rs:266` +
   `338`).

The re-run mints a **fresh session** (`run_worker_now`'s outcome carries a new
`outcome.session_id` — `routes_worker_run.rs:181-188`), and the teammate is
**still inside its 24h probation window** (`PROBATION_HOURS = 24`,
`crates/argo-identity/src/store_hire.rs:137`). So the dispatch gate emits **new
`probation.hold` audit rows on the NEW session_id**
(`argo_agent_runner::turn_helpers_gate::emit_probation_hold`).

The resolution marker the approve wrote is keyed to the **old** session_id, so
the **new** holds are never in `resolved_hold_keys` → they surface as pending.
Net per approve: −1 (old hold resolved) **+N** (new run's holds re-born). The
counter climbs on every reload and never converges to 0 while the teammate
remains on probation.

### Why this is a NEW, still-open defect (not a re-litigation of #2895)

VERIFY-BEFORE-FIX — the closest prior work is **MERGED**, but fixed a
*different half* and explicitly does NOT cover the counter:

- `fix-probation-holds-surface-in-approvals-inbox` — **MERGED #2894**. Added the
  read-side probation-hold arm above (correct that holds surface).
- `fix-probation-rehold-after-approval-or-fix-copy` — **MERGED #2895**. The
  operator-decided fork: probation holds **every** action **by design**
  (DEC-025 #2 — one-shot session grants); the fix was **toast copy only** (2
  files, +15 LOC — NO server code, NO list arm, NO counter).

So re-holding is **by design and must stay** — this brief does NOT try to stop
the gate from re-holding. The bug it fixes is that the **inbox counter never
converges** given by-design re-holding. No other open brief/PR targets the
counter-convergence symptom (greped BUILD_STATUS.md + `git log --oneline -200`).

## Fix (first-principles — pick the convergent semantics)

The inbox is a queue of decisions the operator still owes; a queue the operator
can empty must be able to reach 0. Today an Approve both clears one hold AND
triggers a fresh full run that re-births holds, so clearing the queue re-fills
it. Two convergent options; implement **A** (preferred — keeps re-holding
by-design AND makes the counter honest):

**Option A (preferred) — propagate the resolution to the re-run's new session.**
After the re-run, for the approved `tool`, write a resolution marker keyed to
the re-run's `(new_session_id, tool)` too (not only the old session). Thread the
re-run's `outcome.session_id` back out of `rerun_with_tool` / `run_worker_now`
(already returned in the run body — `routes_worker_run.rs:188`) and have
`decide_probation_hold` write the second marker. An Approve of tool T then nets
exactly −1 for T regardless of how many times the re-run re-emits a T-hold.
Also confirm whether the one-shot grant already suppresses the re-hold for the
approved tool on the re-run (grep the gate path `emit_probation_hold` +
`run_worker_now`'s grant-minting); if it does, the same-tool climb is already
closed and the marker propagation handles the rest.

**Option B (fallback) — scope the hold arm to the teammate's CURRENT session.**
Change `fetch_probation_hold_rows` / `probation_hold_rows` so that per
`(worker_id, tool)` only the hold from the teammate's **latest** session counts
as pending — older sessions' identical holds collapse. Less preferred (changes
read semantics rather than fixing the resolution gap; must not hide a genuinely
distinct held tool).

In BOTH options the audit chain + decrement-per-click are unchanged; only the
convergence of the surfaced PENDING set changes. NO THEATER preserved.

## Acceptance criteria (CHECKABLE verifier)

Add a route test to
`crates/argo-cli/src/handlers/gateway/routes_approvals_probation_tests.rs`
(extend the existing harness — `store_and_tenant`, `write_hold`, `list_items`,
`phold_row`; do NOT add a parallel module). The test MUST encode convergence:

- [ ] **Counter converges, does not climb on reload.** Seed a probation hold for
      tool `T`; assert 1 `phold:` row. Approve via `decide_probation_hold`. Then
      call `list_items` twice more (two reloads) and assert PENDING `phold:`
      count is **0 and stays 0** — the approve does not re-birth an unresolved
      hold for the approved tool. (Drive the re-run path; if the harness cannot
      drive a real worker turn, assert the equivalent invariant: after approve,
      no UNRESOLVED `probation.hold` row for `(any_session, T)` remains visible.)
- [ ] **Reload is idempotent.** Two successive `list_items` with no intervening
      decision return the same PENDING count.
- [ ] **Existing behaviour preserved.** `held_tool_surfaces_in_inbox_and_clears_on_resolution`,
      `repeated_holds_for_same_tool_collapse_to_one_row`,
      `refuse_records_decline_and_clears_the_row` still pass.
- [ ] `cargo test -p argo-cli routes_approvals_probation` green (isolated).
- [ ] Workspace clippy clean on touched files; no new `unwrap`/`expect` outside
      `#[cfg(test)]`.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_approvals_probation.rs`
  (the fix — resolution-marker propagation, or current-session scoping; ~20-40 LOC).
- `crates/argo-cli/src/handlers/gateway/routes_approvals_probation_tests.rs`
  (the convergence verifier — extend the existing module; ~30-50 LOC).
- `crates/argo-cli/src/handlers/gateway/routes_worker_run.rs`
  (read-only confirmation of `outcome.session_id` threading; only touch if
  Option A needs the new session id surfaced — additive return only).
- `docs/build-plan/FEATURE_MATRIX.md` (approvals-inbox / probation-holds row, in-band).

## Parallel-surface declaration

No NEW `pub struct`/`enum`/`type`. The fix reuses `HoldRow`, `write_resolution`,
`resolved_hold_keys`, `fetch_probation_hold_rows` in
`routes_approvals_probation.rs` and extends them in place. Greped
`pub (struct|enum|type)` against `crates/` for the helper names — no parallel
home. Behaviour fix inside one already-canonical module, not a new surface.

## Out of scope

- The probation-window DESIGN (DEC-025 #2 — hold every action 24h, one-shot
  grants); operator-locked, re-affirmed by #2895.
- The toast copy on the worker page (already fixed by #2895).
- The memory-row + seeded `approval_request` arms of the list (not the climb).

## Merge lane

Server-side fix in `argo-cli` (Rust) — the web-only ≤200-LOC vitest lane does
NOT apply (gateway state; vitest cannot reproduce cross-reload server
convergence). Gate normally; the CHECKABLE verifier is the
`cargo test -p argo-cli routes_approvals_probation` convergence test.

~20-40 LOC fix + ~30-50 LOC test. `routes_approvals_probation.rs` is ~361 LOC
today; additions stay well under 500. ~1-2 hours.
