# eval-argo-skill-sdk-author-flow

**Priority**: P2 (claim-4 substrate audit — verify the SDK operators use to author custom skills)
**Owner**: claude
**Type**: Read-only audit + regression smoke test
**Size**: ~150-300 LOC: one Markdown audit report + one integration smoke test
**Authored against**: origin/main `01bbe9b58` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-skill-sdk-author-flow`

## Why

The gap manifest `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` flags `argo-skill-sdk` as **🔴 NOT AUDITED** in the Claim 4 row:

> The crate operators use to AUTHOR custom skills. Unknown surface size + test coverage.

MISSION.md claim-4 is "Skills shared + learned" — the learning loop is well-pinned (rounds 28/29/31b/31c landed 4 regression contracts), but the **authoring** half is structurally invisible to the audit chain. If an operator pulls `argo-skill-sdk` into their own crate to write a custom skill, what's the contract? Which traits do they implement? Where do they declare capabilities, RBAC tags, audit hooks? If the SDK is paper-thin or stale, claim-4's promise ("operators author skills") is materially weaker than the rest of the substrate.

This brief audits the surface and authors a smoke test that exercises the operator-author flow end-to-end: write a 3-skill smoke through the SDK, verify the skills lint clean + canary green + audit row emitted.

## What

### Step 1 — audit report

New file `docs/skycot/audit/2026-05-24-argo-skill-sdk-author-flow/audit.md` (~80-150 LOC). Sections:

1. **Crate surface** — `rg 'pub (fn|struct|enum|trait)' crates/argo-skill-sdk/src/ -n` enumerate every public symbol. Group by purpose: traits an operator implements (e.g. `Skill`, `Capability`), helpers (e.g. `audit_emit`), test fixtures.
2. **Test coverage** — count `cargo test -p argo-skill-sdk` tests; list each test file with a one-line summary of what it pins.
3. **Documentation** — does every public symbol have rustdoc per CLAUDE.md core-platform invariant? Count: `pub_symbols`, `rustdoc'd`, `with_example`. If gaps, list them.
4. **Workspace consumers** — `rg 'argo-skill-sdk = ' crates/*/Cargo.toml` to see which crates depend on it. If zero workspace consumers, flag as drift (mirrors the round-2 17-zero-consumer drift finding from earlier audits).
5. **Author-flow verdict** — can a third-party operator pull `argo-skill-sdk` into their crate today, implement a `Skill`, and ship it? List the gaps that block this.

### Step 2 — author-flow smoke test

New file `crates/argo-skill-sdk/tests/author_flow_smoke_test.rs` (~80-150 LOC, ~3 tests):

1. **Minimal skill** — implement the simplest possible `Skill` (e.g. echo: takes a string, returns it; one capability declared, no RBAC). Assert:
   - the skill compiles (the test compiling IS the assertion),
   - `Skill::name()` returns the declared name,
   - `Skill::invoke(input)` returns the expected echo output.
2. **With capabilities** — implement a skill that declares a capability (e.g. `Capability::FileRead { path: "/tmp/test.txt" }`). Assert the capability is enumerable via `Skill::capabilities()` and matches the declaration.
3. **With audit hook** — implement a skill that invokes the SDK's audit-emit helper (if it exists). Assert the audit row is produced with the expected `kind`/`payload` shape. If no helper exists, document the gap in the audit report §5 and skip this test.

Each test cites the audit report verbatim in a top-of-file doc-comment.

## Acceptance

- [ ] `docs/skycot/audit/2026-05-24-argo-skill-sdk-author-flow/audit.md` exists with the 5 sections.
- [ ] `crates/argo-skill-sdk/tests/author_flow_smoke_test.rs` exists with 2-3 tests (test 3 conditional on SDK surface).
- [ ] `cargo test -p argo-skill-sdk --test author_flow_smoke_test` shows green for every authored test.
- [ ] `cargo clippy -p argo-skill-sdk --all-targets -- -D warnings` clean.
- [ ] If §5 verdict is "operator cannot ship a skill today," report names follow-up fix-briefs (one per gap) — but does NOT author them.
- [ ] No production `.rs` file in `crates/argo-skill-sdk/src/` is touched. Audit-only.

## May-touch

- `docs/skycot/audit/2026-05-24-argo-skill-sdk-author-flow/audit.md` (NEW)
- `crates/argo-skill-sdk/tests/author_flow_smoke_test.rs` (NEW)
- `crates/argo-skill-sdk/Cargo.toml` (if `[dev-dependencies]` needs a test helper)

**Out of may-touch**: every `.rs` file under `crates/argo-skill-sdk/src/`. Audit is read-only across production code.

## Parallel surfaces

- `rg 'author_flow' crates/` returns zero hits — net-new test file.
- `crates/argo-skill/tests/end_to_end_claim4_loop.rs` is the **library-consumer** test (`argo-skill` API). This brief's test is the **SDK-author** test (consumer of `argo-skill-sdk`). Different layer; do not duplicate.
- `crates/argo-skill-sdk` may itself depend on `argo-skill` — that's fine, but the smoke must hit the SDK's public surface, not the underlying crate.

## Sensible deviations

- If `argo-skill-sdk` is mostly empty (e.g. just a few re-exports of `argo-skill` types), the audit report says so plainly, and the smoke test reduces to test 1 only (minimal compile). The audit IS the deliverable — a thin SDK is a finding, not a failure.
- If the SDK declares a `Skill` trait but no concrete invocation harness exists (so the test can't actually call the skill), document the missing harness as a follow-up fix-brief and skip test 1's invoke assertion. The compile-passing assertion still proves the trait shape is implementable.
- If the SDK is structurally incompatible with author-flow goals (e.g. requires runtime DI that's only available inside `argo-cli`), recommend a `mission-skill-sdk-extract-operator-author-shim` fix-brief in the audit report and ship the brief at audit-only completion.
