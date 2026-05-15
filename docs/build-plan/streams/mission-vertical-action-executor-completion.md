# mission-vertical-action-executor-completion — Action executor + instruction_template + retry policies for stub verticals

**Owner**: claude. **Depends on**: L1 MERGED (for IR-based executor wiring).
**Authored against**: origin/main `7598d5d1` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-action-executor-completion`.

## Scope

MISSION_BACKLOG item 138. The ~20 vertical-pack skill-family briefs (customer-support / sales-sdr / marketing-intern / finance-ar / etc.) shipped skills but their action executors, instruction_template hooks, and retry policies remain stubbed. This brief completes those for the 5 v1 priority verticals (Recruiter / AP Specialist / Vendor Manager / Support Specialist / Deal Desk Analyst).

LOC: ~1500 spread across verticals.

## Files you may touch

- For each of the 5 v1 priority verticals:
  - `verticals/<vertical-slug>/vertical.toml` — TOUCH. Fill in `action_executor`, `instruction_template`, `retry_policy` sections.
  - `crates/argo-vertical/src/registry.rs` — narrowly: confirm registration. ≤5 modified lines.
- NEW `crates/argo-vertical-action-executor/src/lib.rs` if a shared registry-wiring layer is needed; otherwise inline per vertical.

## Files you must NOT touch

- Existing vertical-pack skill files (those are L+M-track work; this is the executor/template layer above them); the M15 vertical-pack-loader (separate brief).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] 5 priority verticals each have a non-stub action_executor + instruction_template + retry_policy in vertical.toml.
- [ ] Each vertical's eval suite (>=7 fixtures, already SHIPPED via #1485) still green.
- [ ] Smoke test: `argo work serve --vertical <name> --once --max-steps 3` runs without falling back to default executor.
- [ ] 5 unit tests (one per vertical).
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-vertical-action-executor-completion`
- PR title: `Action executor + instruction_template + retry policies for stub verticals (mission-vertical-action-executor-completion)`

## Notes for the implementer

- Mission-claim alignment: claim 5 (real-world PC execution per vertical, not generic fallback).
- Authored from MISSION_BACKLOG.md [NEW] item — no prior brief existed; check `rg 'pub (struct|enum|fn) <Name>'` for any new names introduced before declaring (drift prevention rule).


## Files you may touch — AUTHORITATIVE 2026-05-15 evening (supersedes original list)

Codex noted brief targets `verticals/<slug>/vertical.toml` paths but the 5 priority v1 packs now live under `packs/{recruiter,ap-specialist,vendor-manager,support-tier1,deal-desk}/`. Correcting:

- For each of the 5 packs: `packs/<pack-slug>/pack.toml` — fill in action_executor, instruction_template, retry_policy sections. ≤30 modified lines per pack.
- For each pack: `packs/<pack-slug>/worker.toml` (if exists, else NEW) — set executor binding. ≤15 modified lines.
- `crates/argo-vertical-packs/src/loader.rs` — narrowly: confirm registration; no parallel registry surface. ≤10 modified lines.
