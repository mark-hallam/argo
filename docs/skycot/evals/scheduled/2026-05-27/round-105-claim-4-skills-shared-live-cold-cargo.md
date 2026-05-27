# Round 105 — Claim 4 (Skills shared + learned) LIVE cold-cargo rerun

- **Date**: 2026-05-27 (UTC)
- **Round**: 105
- **Pillar/Claim**: Claim 4 — Skills shared + learned (MISSION.md operator-locked LIVE criteria)
- **Verdict**: 🟢 PASS (LIVE re-confirmed)

## Why this claim, this round

Round 102 hit Claim 2, R103 Claim 6, R104 Claim 1. Claim 4 hadn't been touched in the last 3 rounds and the prompt's "axis 2 — mission claim coverage" rotation calls for it. The load-bearing test is `claim_4_live_skill_install_invoke_and_audit_with_provenance` at `crates/argo-skill/tests/claim_4_live_install_invoke.rs`, the same test that gated the CLAUDE.md Claim 4 🟡 LIKELY-LIVE → 🟢 LIVE flip via PR #2285 on 2026-05-25.

## Setup

```bash
CARGO_TARGET_DIR=/tmp/cold-claim-4-target \
  cargo test --package argo-skill --test claim_4_live_install_invoke claim_4_live \
    -- --nocapture
```

Isolated target dir per `memory_pressure_safeguards` (workers share `/tmp/argo-shared-target`; this dir is independent so the run doesn't conflict).

## Observed

```
Compiling argo-skill v0.0.1 (~/code/argo/crates/argo-skill)
Finished `test` profile [optimized + debuginfo] target(s) in 1m 41s
 Running tests/claim_4_live_install_invoke.rs

running 1 test
test claim_4_live_skill_install_invoke_and_audit_with_provenance ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
finished in 0.10s
```

Cargo cold compile 1m41s, test runtime 0.10s. Matches the round-of-PR-#2285 timing within seconds (R104 referenced "1m53s cargo, 0.10s test runtime").

## Anti-theater check

The test is 459 LOC across 8 phases (`wc -l crates/argo-skill/tests/claim_4_live_install_invoke.rs`):

1. Schema preflight (assert relevant tables present).
2. Skill install via real `argo-skill` install path.
3. `skill_provenance` row written with source/version/author triple.
4. Worker `list_for_action` (low + high cardinality) — exercises overlay filter.
5. `attach_to_session` returns the installed skill.
6. `WorkflowStepCompleted` audit row written with full production payload shape (skill_name + status + final_text_sha).
7. Read the audit row back and assert payload['skill_name'] / ['status'] / ['final_text_sha'] match.
8. Forensic join: audit row → `skill_provenance` returns the same triple asserted at install.

The phase-8 forensic join is the dispositive anti-theater pin: it walks audit → provenance through production code paths and would fail if either side drifted away from the other. The test isn't asserting an in-memory mirror; it's asserting that audit and provenance survive the round-trip through `SessionStore` + `AuditSink`.

`assert` count: 17 assertions on the 4 fixture constants (`FIXTURE_SKILL_NAME`, `FIXTURE_SKILL_VERSION`, `FIXTURE_SOURCE_PACK`, plus computed `final_text_sha`). Each fixture is referenced ≥3 times, so changing one would surface as multi-line test failure — single-line edits cannot rewrite this test green.

The "round-29 prior coverage" partial-coverage flag in BUILD_STATUS.md (`crates/argo-skill/tests/claim_4_mvp_row_4_novel_sequence_detection.rs`) is a different test (threshold-gate boundary + candidate lifecycle). The claim-4-LIVE pin specifically exercises install→invoke→audit→provenance — that's the surface the MISSION.md operator-locked criterion names, and it's still load-bearing.

## Gap / follow-up brief candidates

None this round. Claim 4 cold cargo PASS, anti-theater dispositive, last-2-round-untouched rotation closed. The next claim to rotate to (R106 or R107) will be Claim 3 (Platform-builds-workers) or Claim 5 (Any-PC-execution) since both were last touched ≥3 rounds back.

116th anti-theater verification this week.
