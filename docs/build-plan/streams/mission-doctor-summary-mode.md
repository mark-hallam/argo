# mission-doctor-summary-mode — argo doctor --summary one-line operator health output

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `66d07a69` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-doctor-summary-mode`.

## Scope

Operators frequently run `argo doctor` as a quick health check. The current output is verbose (multi-section JSON or text). For dashboard scraping or quick CLI checks, an operator wants a single-line summary: `OK 12/12 checks` or `DEGRADED 11/12 checks (1 warn: gateway_smoke)`.

This brief adds `argo doctor --summary` flag that:
1. Runs all existing doctor checks (don't change check logic).
2. Emits exactly one line to stdout.
3. Exit code reflects overall state (0=OK, 1=DEGRADED with warnings, 2=FAIL with errors).
4. Optionally with `--json` for `{"state":"OK","passed":12,"warned":0,"failed":0}` shape.

LOC: ~150.

## Files you may touch

- `crates/argo-cli/src/handlers/doctor.rs` — narrowly: add `--summary` flag parsing + one-line formatter at end of existing check pipeline. ≤80 modified lines.
- `crates/argo-cli/src/handlers/doctor_tests.rs` (if exists, else sibling) — 4 unit tests covering OK/DEGRADED/FAIL/JSON paths. ≤80 modified lines.
- `docs/skycot/runbook/doctor.md` (if exists, else NEW) — narrowly: document `--summary` usage. ≤30 modified lines.

## Files you must NOT touch

- The individual doctor check implementations.
- argo-doctor library crate (if separate) — only the CLI surface.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `argo doctor --summary` emits a single line on stdout.
- [ ] Exit codes: 0=OK, 1=DEGRADED (warns), 2=FAIL (errors).
- [ ] `argo doctor --summary --json` emits structured envelope.
- [ ] 4 unit tests passing.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-mission-doctor-summary-mode`
- PR title: `feat(doctor): --summary one-line operator health output (mission-doctor-summary-mode)`

## Notes for the implementer

- Mission-claim alignment: production hardening (operator UX; supports operability across all claims).
- Drift-prevention: extends existing `argo doctor` surface; no parallel command.
