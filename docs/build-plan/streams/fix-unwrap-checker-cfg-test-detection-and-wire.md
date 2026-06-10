# fix-unwrap-checker-cfg-test-detection-and-wire

Authored against: origin/main ca46b97f1e42d9d093d275763b55a04add3d65c3 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-unwrap-checker-cfg-test-detection-and-wire

## Goal

Fix the unwrap/expect invariant checker so it stops producing false positives,
then wire it into the conductor pre-merge gate so the "no unwrap/expect outside
#[cfg(test)]" invariant is actually enforced.

## Why now (architecture audit 2026-06-10, P2-5)

- `bash scripts/check-no-unwrap-outside-test.sh` exits 1 reporting 10
  "PRODUCTION violations", but **8 are false positives** — inside fn-level
  `#[cfg(test)]` functions the script fails to recognize (verified:
  `crates/argo-cli/src/handlers/gateway/routes_worker_memory_helpers.rs:424-447`
  and `crates/argo-cli/src/handlers/vertical_workflow_step.rs:339-350` are
  fn-level `#[cfg(test)]`).
- The 2 REAL ones are justified crypto cases:
  `crates/argo-credentials/src/master_key.rs:205` and `:316`
  (`.expect("HKDF expand 32 bytes always fits")`).
- The checker is wired to NOTHING on the merge path (verified 2026-06-10:
  `grep -c check-no-unwrap conductor-pre-merge-gate.sh` = 0); only an opt-in
  local hook (`scripts/install-no-unwrap-hook.sh`).

## Fix

1. Teach the script fn-level `#[cfg(test)]` detection (currently it likely only
   recognizes `mod tests`-level gating) so the 8 false positives clear.
2. Resolve the 2 real HKDF expects: convert to `map_err`/`unwrap_or_else` with
   a structural-impossibility comment, OR add an explicit, justified allowlist
   entry — operator-pattern choice; prefer the code fix.
3. Wire the (now-clean) check into `conductor-pre-merge-gate.sh`.

## May touch

- `scripts/check-no-unwrap-outside-test.sh`
- `crates/argo-credentials/src/master_key.rs` (the 2 expects) OR an allowlist
- `scripts/conductor-pre-merge-gate.sh`

## Acceptance / verifier

1. `bash scripts/check-no-unwrap-outside-test.sh` exits 0 on clean main (0 false
   positives, 0 real unjustified).
2. A synthetic `.unwrap()` in a non-test production fn FAILS the check.
3. Conductor gate includes the step.

## Size / owner

~30 LOC script + 2-line crate fix. Owner: claude. 🟡 P2 — a guard that both
lies and guards nothing today.
