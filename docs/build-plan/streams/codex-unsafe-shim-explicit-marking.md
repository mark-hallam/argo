# codex-unsafe-shim-explicit-marking — Mark OS FFI shims with SAFETY comments + CI enforcement (F-14 P2)

**Owner**: codex. **Depends on**: `codex-ci-workflows-reactivate` MERGED (so the unsafe-lint actually fires).
**Authored against**: origin/main `58e853b6` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-unsafe-shim-explicit-marking`.
**Source**: F-14 P2 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Mission alignment

- **CLAUDE.md:59** — CI must reject `unsafe` outside marked FFI shims.
- **Security invariant** — every `unsafe` block in the codebase should have an explicit `SAFETY:` justification, audit-trail-style.

## Scope

P2 hygiene. Unix signal lifecycle code uses `unsafe { libc::kill(...) }` in 5 places:

**Production (no SAFETY comment):**
- `crates/argo-cli/src/handlers/gateway/lifecycle.rs:20` — `libc::kill(pid, 0)` (process-alive check)
- `crates/argo-cli/src/handlers/gateway/lifecycle.rs:31` — `libc::kill(pid, libc::SIGTERM)` (graceful shutdown)
- `crates/argo-cli/src/handlers/self_update_install.rs:174` — self-update signal path

**Test files (no SAFETY comment):**
- `crates/argo-cli/tests/work_serve_shutdown.rs:80`, `:160`
- `crates/argo-cli/tests/shutdown_e2e.rs:81`

**Working pattern exists** in `crates/argo-tool/src/process_registry_tail.rs:94` — has explicit `SAFETY:` comment.

Scope:
- Add `// SAFETY: <reason>` comment to each `unsafe { libc::kill(...) }` site (5 production + 3 test).
- The SAFETY comment should state the precondition that makes the unsafe-block safe: pid is non-negative i32, signal is valid libc constant, no aliasing concerns.
- Update `scripts/check-fn-loc.sh` (or add `scripts/check-unsafe-safety.sh` if cleaner) — fail on any `unsafe {` not immediately preceded by `// SAFETY:` comment.
- Wire the lint into the F-01-reactivated CI workflow (in `ci.yml` once renamed from `.disabled`).

LOC: ~80.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/lifecycle.rs` — add SAFETY comments at `:20` and `:31`. ≤10 modified lines.
- `crates/argo-cli/src/handlers/self_update_install.rs` — SAFETY comment at `:174`. ≤5 modified lines.
- `crates/argo-cli/tests/work_serve_shutdown.rs` — SAFETY at `:80` and `:160`. ≤10 modified lines.
- `crates/argo-cli/tests/shutdown_e2e.rs` — SAFETY at `:81`. ≤5 modified lines.
- `scripts/check-unsafe-safety.sh` (NEW) — bash/grep lint requiring `// SAFETY:` before every `unsafe {`. ≤40 LOC.
- `scripts/local-ci-check.sh` — invoke the new lint. ≤10 modified lines.
- `.github/workflows/ci.yml` (post-F-01) — invoke the new lint. ≤5 modified lines.

## Files you must NOT touch

- The `unsafe` blocks themselves — no behavior change, only comments.
- Other `unsafe` blocks not in the audited list (process_registry_tail.rs already correct).
- Don't refactor the signal-handling logic — out of scope.

## Acceptance criteria

- [ ] Every `unsafe {` block in `crates/argo-cli/src/handlers/gateway/lifecycle.rs`, `self_update_install.rs`, `tests/work_serve_shutdown.rs`, `tests/shutdown_e2e.rs` has a preceding `// SAFETY:` comment with justification.
- [ ] `scripts/check-unsafe-safety.sh` exists, executable, and reports zero violations on current main.
- [ ] Local CI invokes the lint.
- [ ] (Post-F-01 CI re-enable): GitHub CI invokes the lint.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-unsafe-shim-explicit-marking`
- PR title: `chore: SAFETY: comments on libc::kill shims + CI lint (codex-unsafe-shim-explicit-marking) — F-14 P2`

## Notes for the implementer

- SAFETY comment format: `// SAFETY: <preconditions met>`. Example: `// SAFETY: pid is a non-negative i32 returned by std::process::Child::id(); SIGTERM is a valid libc::c_int.`
- The lint regex should be permissive about comment placement (single-line `// SAFETY:` or block `/* SAFETY: ... */`) but strict about presence.
- After this brief, every `unsafe` shim is audit-trailed; CI structurally prevents new unsafe blocks from landing without justification.
