# Stream H (follow-up) — `argo doctor` checks for the H-stack

**Owner**: codex. **Depends on**: H-1 (#796) + H-2 (#800) +
H-3 (#799) merged. **Independent** of H-orchestrator-persist
(degrades gracefully if persist tables are missing).

## Scope

Extend `argo doctor` with health checks specific to the
H-auto-approval stack. The doctor command already exists and
runs a battery of checks; this brief adds 4 more:

1. **Auto-approval policy file** — does
   `<cwd>/.skycot/auto_approval.toml` exist? Parse it. If
   present but `enabled = true` AND no caps set, warn (a
   bare-enabled policy is dangerous).
2. **worker_budget table reachable** — open the store, run
   `SELECT 1 FROM worker_budget LIMIT 1`; report row count.
3. **session + handoff tables reachable** — same shape; if
   the persist migration hasn't landed yet, report `MISSING`
   not `ERROR` (the brief is independent of persist).
4. **NotifyChannel manifests** — list every
   `manifests/channels/*.toml`, parse each, report which
   channel ids are configured. Useful for diagnosing
   "approval went nowhere" issues.

Output: extends the existing JSON envelope from `argo
doctor --json` with a new `auto_approval` section.

## Files you may touch
- `crates/argo-cli/src/handlers/doctor.rs` — extend (do NOT
  replace any existing check; ADD the 4 new ones)
- `crates/argo-cli/src/handlers/doctor_h_stack.rs` (NEW —
  put the 4 checks here as a sibling module to keep
  doctor.rs under the 500-LOC budget)

## Files you must NOT touch
- Any other handler file.
- `crates/argo-identity/**` — read-only.
- `crates/argo-orchestrator/**` — read-only.

## Acceptance criteria
- [ ] `argo doctor` emits a new `Auto-approval stack:`
      section with 4 sub-checks, each labelled OK / WARN /
      ERROR / MISSING.
- [ ] `argo doctor --json` includes a new `auto_approval`
      key with structured sub-results.
- [ ] Missing migrations report as `MISSING`, not crash.
- [ ] Bare-enabled policy (no caps) reports a WARN with
      explanation.
- [ ] 5+ unit tests covering each check's OK / WARN / MISSING
      paths.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-doctor-extensions`
- PR title: `doctor: H-stack health checks (H-doctor-extensions)`

## Notes
- Look at the existing `doctor.rs` for the check pattern
  (each check returns `(name, status, detail)`).
- Don't add a Prometheus dep here — that's a separate
  brief.
