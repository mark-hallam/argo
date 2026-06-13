# dec042-p1c-distill-and-proposal-audit-rows

DEC-042 Stage-2 "audited" leg — emit `skill.distilled` + `skill.proposed_for_review` audit rows so the self-improve loop's distill + propose events are tamper-evident, not silent.

**Owner**: claude — audit/compliance surface; conductor / adversarial review BEFORE merge (this is a merge-gate, NOT an authoring blocker).
**Depends on**: dec042-p1b-eval-floor-gates-distilled-proposal — **MERGED** `25734806a` (PR #2857, 2026-06-12). Dependency satisfied; this row should flip BLOCKED→TODO (the ledger note at BUILD_STATUS.md still says "p1b is itself BLOCKED" — that is STALE; p1b merged).
**Authored against**: origin/main `ad9518400` on `2026-06-14`.
**Validate before claim**: bash scripts/check-brief-freshness.sh dec042-p1c-distill-and-proposal-audit-rows

Source: DEC-042 staged roadmap (`docs/skycot/decisions/DEC-042-continuous-self-improvement-loop.md` §5) — Stage-2 (p1a/p1b/p1c) completion audit. This is the observability + security (tamper-evident traceability) leg of the self-improvement product loop. DEC-042's safety spine: every loop ends in a CHECKABLE verifier (here = a hash-chained audit row), never a model's self-report.

## Problem (the distill + proposal events are SILENT — verified on current main)

The self-improve learning loop emits exactly ONE audit row today — the
per-cluster **decision** row (kind `skill.self_improve.decision`, written by
`append_decision_audit`). The two UPSTREAM events that precede it leave no
tamper-evident record:

- **Distill call — no audit.** The cron's gated distill path
  `distill_and_propose_judged_for_tenant_gated`
  (`crates/argo-skill/src/dreams_distillation.rs:334-375`) only does
  `tracing::info!` (`:358`) + `insert_row_with_state` (`:365`) — a `tracing`
  log line is NOT a hash-chained audit row. The REAL-provider CLI path
  (`crates/argo-cli/src/handlers/skill_distill_now.rs`, `run` →
  `try_distill` → `distill_cohort`) hits the Dreams API at real per-cohort
  cost and likewise emits no audit row. **Provider spend, model, and outcome
  are invisible to the audit trail.**
- **Proposal entry — no audit.** A candidate enters the pending-review queue
  via `propose_for_review_judged_with_evals_for_tenant`
  (`crates/argo-skill/src/store_vertical.rs:327-382`), which calls
  `insert_row_with_state` (`:382`) silently. The operator sees a skill queued
  for review with **no record of HOW it arrived** (distilled vs
  judged-memory vs manual).
- **The only emission today** is `self_improve_cron.rs:313`
  (`helpers::append_decision_audit(...)`).

This breaks the "audited" requirement of Stage-2: the loop can spend provider
budget and queue a skill for human review with zero entries in the
hash-chained, tamper-evident audit log between trace-ingest and the decision
row.

## Fix (two additive audit kinds; REUSE the hash-chain helpers, do NOT fork)

Add two **additive** audit kinds and emit them at the two silent points,
reusing the existing chaining primitives:

1. **`skill.distilled`** — emitted when a distillation call returns a
   candidate. Emit from BOTH the cron path
   (`dreams_distillation.rs` gated/non-gated distill) AND the CLI path
   (`skill_distill_now.rs`). Payload: cohort (`vertical`, `action`, `size`),
   `model`, estimated cost (USD micros), candidate skill `name`/`version`,
   `tenant_id`.
2. **`skill.proposed_for_review`** — emitted at review-queue entry
   (the `propose_for_review_judged_with_evals_for_tenant` insert point).
   Payload: skill `name`/`version`, `vertical`, `source`
   (`distilled | judged_memory | manual`), `learned_in` session, `tenant_id`.

**Reuse the hash-chain, do NOT hand-roll it.** Use the BEGIN-IMMEDIATE +
`last_audit_hash` + `chain_hash` pattern already in
`crates/argo-skill/src/self_improve_cron_helpers.rs:18-104`
(`append_decision_audit` is the template: `begin_with("BEGIN IMMEDIATE")` at
`:57`, parent hash at `:60`, `chain_hash(parent, KIND, canonical)` at `:61`).
Generalize / call that path for the two new kinds — a forked chaining
implementation is a P1-class drift bug (DEC-029 NO THEATER: the verifier must
chain through ONE chain).

**LOC ceiling pre-check (verified on `ad9518400`):**
`crates/argo-audit/src/event_kind.rs` = 342/500, `crates/argo-audit/src/event.rs`
= 332/500 — both have headroom for 2 additive enum variants + their
`as_str`/`parse`/`all` mappings. **Re-verify headroom holds AFTER your edit**
(both files MUST stay < 500 LOC). If either file would cross 500, STOP and
queue a split brief rather than force-fit.

## May touch

- `crates/argo-audit/src/event_kind.rs` — add 2 additive `AuditKind` enum
  variants (`SkillDistilled`, `SkillProposedForReview`), additive ONLY.
- `crates/argo-audit/src/event.rs` — add the `as_str` (~`:45`), `parse`
  (~`:143`), and `all` (~`:241`) mappings for the 2 new kinds (a new kind
  needs edits in BOTH audit files — discovery-verified).
- `crates/argo-skill/src/dreams_distillation.rs` — emit `skill.distilled` from
  the distill path.
- `crates/argo-skill/src/self_improve_cron.rs` — emit
  `skill.proposed_for_review` at queue entry (or thread through helpers).
- `crates/argo-skill/src/self_improve_cron_helpers.rs` — generalize the
  existing chain-append helper so the 2 new kinds reuse it (no fork).
- `crates/argo-skill/src/store_vertical.rs` — proposal insert point
  (`propose_for_review_judged_with_evals_for_tenant`).
- `crates/argo-cli/src/handlers/skill_distill_now.rs` — emit `skill.distilled`
  on the CLI/REAL-provider distill path.
- `crates/argo-cli/src/cli_subcommands_skill.rs` — the `distill` subcommand
  definition/dispatch (post-cli-split sibling of `skill_distill_now.rs`,
  defines the `distill` arg at `:254`). Touch ONLY if the audit-emission needs
  a new arg/field threaded; the emission itself lives in the handler above.
- Tests alongside the above crates.
- `docs/build-plan/FEATURE_MATRIX.md` — update the DEC-042 Stage-2 row IN-BAND
  in the SAME commit (see reminder below).

Do NOT touch `store_vertical.rs`/`dreams_distillation.rs`/`skill_distill_now.rs`
signatures in a way that breaks out-of-scope callers — additive constructors /
additive emission only (p1b deliberately added the `*_gated` variant rather
than mutating the shared signature; follow that discipline).

## Parallel-surface declaration

Ran `rg 'pub (struct|enum|type) <Name>' crates/` for every NEW identifier:

- `rg 'pub (struct|enum|type) SkillDistilled' crates/` → **0 hits** — and
  `rg -ni 'skill\.distilled|proposed_for_review|SkillDistilled|ProposedForReview'
  crates/argo-audit/src crates/argo-skill/src` → **0 hits**. NEW, justified:
  these are new enum *variants* on the existing `AuditKind` enum
  (`crates/argo-audit/src/event_kind.rs`) — they EXTEND that enum, they do NOT
  create a parallel type. The closest existing siblings are `SkillApproved` /
  `SkillRejected` / `SkillCandidatePromoted` / `SkillRolledBack` /
  `SkillRegressionDetected` (`event_kind.rs:91-105`); the new variants sit in
  that same family and follow the same `skill.*` `as_str` convention.
- **No new pub struct / pub type / standalone enum is introduced.** Payloads
  are carried as the existing JSON-payload shape used by every other
  `AuditKind` (no new typed struct). If a payload helper is added it is a
  private (`fn` / `pub(super)`) helper in the argo-skill crate, not a new
  public surface.

## Acceptance criteria + CHECKABLE verifier

1. Two additive `AuditKind` variants (`skill.distilled`,
   `skill.proposed_for_review`) added to `event_kind.rs` (enum) AND `event.rs`
   (`as_str`/`parse`/`all`) — round-trip parse/`as_str` covered by a test;
   both files stay < 500 LOC.
2. `skill.distilled` emitted from BOTH the cron distill path
   (`dreams_distillation.rs`) and the CLI path (`skill_distill_now.rs`);
   payload carries cohort (vertical/action/size), model, estimated cost,
   candidate name/version, `tenant_id` (tenant threaded from the cron's
   per-trace scope / CLI session — never `PLACEHOLDER_TENANT_ID`; p1a's
   `*_for_tenant` chain is the tenant source).
3. `skill.proposed_for_review` emitted at review-queue entry; payload carries
   skill name/version, vertical, source (`distilled|judged_memory|manual`),
   `learned_in` session, `tenant_id`.
4. Chaining REUSES `self_improve_cron_helpers.rs` BEGIN-IMMEDIATE +
   `chain_hash`/`last_audit_hash` — NO forked chain. A test asserts the chain
   verifies END-TO-END across all three row kinds.

**CHECKABLE VERIFIER** (the loop ends in a check, not a self-report):
- (a) **Test**: one self-improve tick over a qualifying cohort yields ≥ 3
  chained audit rows in order `skill.distilled` → `skill.proposed_for_review`
  → `skill.self_improve.decision`, and the hash-chain verifies end-to-end
  (each row's `chain_hash` derives from the prior row's hash).
- (b) **Test**: `argo skill distill-now` (mock caller) emits `skill.distilled`
  with `tenant_id` + `model` + cost fields populated (non-placeholder tenant).
- (c) `cargo test -p argo-skill -p argo-cli` targeted suites GREEN +
  `cargo clippy -p argo-audit -p argo-skill -p argo-cli` CLEAN (paste the
  passing tail into the PR body).

Audit/compliance surface → conductor / adversarial review before merge
(merge-gate, NOT an authoring blocker). Do NOT self-merge.

## Constraints (core-platform invariants)

- All touched files stay < 500 LOC; no fn > 80 LOC (decompose into
  `self_improve_cron_helpers.rs` if an emission helper would exceed it).
- No `unwrap`/`expect` outside `#[cfg(test)]` — audit emission is on the
  loop's hot path; propagate `ArgoResult` errors.
- Multi-tenant by construction: `tenant_id` threaded through BOTH new payloads
  from the real per-trace / per-session scope (reuse p1a's `*_for_tenant`
  chain; never the placeholder).
- Additive migration discipline: no new audit-table column is required (reuse
  the existing payload-JSON shape); if any schema touch is needed it is
  additive-only per `docs/skycot/operator/migrations.md`.
- Every new public API has rustdoc with at least one example (the 2 enum
  variants get doc comments matching the existing `Skill*` variants' style).

## Ledger discipline (REQUIRED — same commit)

Update `docs/build-plan/FEATURE_MATRIX.md` IN-BAND in the SAME commit that
ships the code: flip / add the DEC-042 Stage-2 "audited" row with the status
badge + commit SHA + crate/route. The `FEATURE_MATRIX.md` markdown IS the
source-of-truth ledger; do NOT commit the generated
`docs/skycot/feature-matrix.html` (gitignored artifact — DEC-043). If the
ledger lags the code, stop and reconcile before opening the PR.

~90–140 LOC. **🟡 P2 — closes the "audited" leg of DEC-042 Stage-2.**
