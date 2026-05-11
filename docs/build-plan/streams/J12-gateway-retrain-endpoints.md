# Stream J12 — Gateway retrain endpoints (classify / commit / undo / reverse-test)

**Owner**: claude. **Depends on**: J7 MERGED; J10 MERGED (memory CRUD); J4 MERGED (Genesis-lite classify path reused for retrain classification).
**Authored against**: origin/main `eab8a50b` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J12-gateway-retrain-endpoints`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 12 of 18 — unblocks frontend E4-g.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Multi-approval | Single delegator commits | Department-level approval workflow for retrains in regulated tenants | `mission-enterprise-retrain-multi-approval` (named in E4-g) |
| Conflict detection scope | Worker-level only | Cross-worker / cross-team detection | `mission-enterprise-retrain-cross-worker-conflicts` (named) |
| Regression test scope | 50-action sample from `mission-skill-regression-detector` MERGED | + curated golden examples per vertical | `mission-enterprise-retrain-golden-examples` (named) |
| Undo TTL | 14 days client + server | Same | n/a |
| Reverse-test cost | Free in v1 | Per-tenant LLM cost cap on reverse-test | `mission-enterprise-retrain-reverse-test-cost-cap` |
| Audit | Single retrain row per commit + linked rollback row on undo | Full retrain-event bundle for compliance | `mission-enterprise-retrain-audit-bundle` (named) |

v1 is acceptable: retrain mutations all run through
existing classifier + regression-detector + memory
infrastructure.

## Why

E4-g's RetrainDialog calls four endpoints under
`/api/v1/workers/<id>/retrain/*`. Until they exist,
E4-g falls back to mock mutations. This brief lands
them in one batch.

## Scope

4 JSON endpoints, all under
`/api/v1/workers/<id>/retrain/`:

- `POST .../classify` body `{ input: string }`
  → `{ classification, plain_english, conflicts?,
  needs_generalization?, regression_preview?,
  structural_change }`.
- `POST .../commit` body
  `{ classification, structural_change, accepted_conflict_resolution? }`
  → `201 { retrain_id, applied_at }`.
- `POST .../undo` body `{ retrain_id }`
  → `200 { ok: true, undone_at }`.
- `POST .../reverse-test` body
  `{ retrain_id, sample_size }`
  → `{ samples: ReverseTestSample[] }` where
  each sample is `{ inbound_context, before_output,
  after_output, outcome }`.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `routes_dispatch_post.rs`
  (narrowly add 4 arms).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_retrain.rs`
  — NEW. ≤ 480 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_retrain_tests.rs`
  — NEW. ≥ 14 tests. ≤ 500 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 1 line.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`
  — narrowly: 4 arms. ≤ 380 LOC after.

## Files you must NOT touch

- `crates/argo-genesis/**` — read-only.
- `crates/argo-memory/**` — read-only.
- `crates/argo-skill/**` (regression detector) —
  read-only.
- Other Track-J handler files.

## Acceptance criteria

- [ ] `POST .../classify`:
      - Calls Genesis-lite to classify the input
        into one of `memory_fact / policy_rule /
        skill_correction / vocabulary_patch /
        action_ban`.
      - Returns plain-English description.
      - Detects conflicts against existing memory
        + policy chain; returns `conflicts` array.
      - When ambiguous, returns
        `needs_generalization` with 1 clarifying
        question + 2 options.
      - Runs regression-detector against last 50
        actions; returns `regression_preview` array
        if any regressions detected.
- [ ] `POST .../commit`:
      - Persists the structural change (memory
        write / policy add / voice profile edit /
        lexicon add / action ban).
      - Atomic transaction across whichever stores
        are affected.
      - Records a retrain_audit row with:
        - actor + worker_id + input text
        - classification + structural_change
        - conflict resolution choice (if any)
        - regression-check verdict
        - applied_at timestamp
      - Resets trust-ratchet for affected
        action_type if classification = policy_rule.
      - 201 + `{ retrain_id, applied_at }`.
- [ ] `POST .../undo`:
      - Looks up the retrain_audit row.
      - If older than 14 days → 410 Gone.
      - Reverses the structural change via the
        snapshot stored at commit time.
      - Audit row stamped as "undone".
      - Resets trust-ratchet again (so subsequent
        autonomy proposals fire fresh).
- [ ] `POST .../reverse-test`:
      - Loads `sample_size` (1-10; default 3)
        recent audit rows where the action_type
        matches the retrain's scope.
      - For each: runs the worker's response
        generator with BOTH the pre-retrain state
        AND the post-retrain state.
      - Returns `before_output` + `after_output` +
        outcome tag (`safe` / `review` / `blocked`).
      - SIDE-EFFECT-FREE (no real channel sends).
      - Cost limited to 10 LLM calls per request.
- [ ] All endpoints require bearer auth.
- [ ] Cross-tenant access → 403.
- [ ] Unknown worker id → 404.
- [ ] 14 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file ≤ 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J12-gateway-retrain-endpoints`
- PR title: `gateway: retrain classify/commit/undo/reverse-test endpoints (J12)`

## Notes for the implementer

- **Classifier reuse.** Call Genesis-lite via the
  same path J4 wired (`argo_genesis::classify`).
  Pass the input + worker context; the classifier
  decides which structural change applies.

- **Structural change shape.** A tagged union:
  ```rust
  enum StructuralChange {
      MemoryFact { text: String },
      PolicyRule { predicate_json: serde_json::Value, action_type: String },
      VoiceProfileEdit { tone: Option<Tone>, formality: Option<u8>, signature_phrases: Option<Vec<String>> },
      VocabularyPatch { from: String, to: String },
      ActionBan { action_type: String },
  }
  ```

- **Snapshot at commit.** Before applying the
  change, capture the BEFORE state of whatever
  store is being mutated:
  - MemoryFact: capture the namespace state
    (or just record the new fact for inverse-on-
    undo).
  - PolicyRule: capture the policy chain.
  - VoiceProfileEdit: capture the worker.persona
    JSON.
  - VocabularyPatch: capture the lexicon row.
  - ActionBan: capture the allowed-actions set.
  Store as `retrain_audit.before_snapshot`.

- **Undo logic.** Inverse-apply the snapshot:
  - MemoryFact: delete the fact added.
  - PolicyRule: remove the rule.
  - VoiceProfileEdit: restore the prior persona
    JSON.
  - VocabularyPatch: delete the lexicon row.
  - ActionBan: re-add the action_type to allowed
    set.
  Then mark the original retrain_audit row's
  `undone_at`.

- **Regression-detector integration.** Reuse the
  existing `mission-skill-regression-detector`
  MERGED API: `detect_regressions(worker_id,
  proposed_change, sample_window=50)`. Returns
  ≥ 0 sample-mismatches with before/after pairs.

- **Reverse-test execution.** Runs the worker's
  response generator twice per sample (before +
  after the retrain). For non-LLM actions (e.g.
  policy gates), the comparison is structural;
  for LLM actions, the LLM is called with the
  appropriate prompt context. Hard-cap at 10
  LLM calls per request to prevent runaway cost.

- **No channel side-effects.** Reverse-test runs
  in the trajectory-replay sandbox (same
  mechanism as E5-d's audit-replay). MCP calls
  → mock; outbound channels → log-only.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-retrain-multi-approval`
  (named).
- `mission-enterprise-retrain-cross-worker-conflicts`
  (named).
- `mission-enterprise-retrain-golden-examples`
  (named).
- `mission-enterprise-retrain-reverse-test-cost-cap`
  (named).
- `mission-enterprise-retrain-audit-bundle`
  (named).
