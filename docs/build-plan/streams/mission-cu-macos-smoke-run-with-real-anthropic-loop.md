# mission-cu-macos-smoke-run-with-real-anthropic-loop — one-time macOS computer-use end-to-end smoke

**Owner**: claude or codex. **Depends on**: operator macOS Accessibility + Screen Recording perms granted on `argo` binary (carry from CLAUDE.md "Remaining for v1.0 launch" item (c)).
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-macos-smoke-run-with-real-anthropic-loop`.

## Why this is P2 (claim-5 LIVE flip prerequisite)

Mission gap manifest row `argo-executor-computer-use-macos` is 🟢 audited (37/37 lib tests PASS) but the smoke test (`notes_smoke.rs`) is gated on `ARGO_NATIVE_MACOS_INTEGRATION=1` and has never been run end-to-end against a real Anthropic API loop. CLAUDE.md status block lists claim-5 as 🟡 LIKELY-LIVE-pending-perms — this brief closes that gap.

## Scope

1. **Prerequisite check**: confirm operator has granted Accessibility + Screen Recording perms on the built `argo` binary. If not, log the missing perm in the report and stop — do NOT attempt to bypass.
2. **Build the binary**: `cargo build --release -p argo-cli` (one-time cold build is fine; document wall time).
3. **Run the Notes smoke**: `ARGO_NATIVE_MACOS_INTEGRATION=1 cargo test -p argo-executor-computer-use-macos --test notes_smoke -- --nocapture` with a live Anthropic API key in env.
4. **Document**: paste the test output (with API key redacted) into a report at `docs/skycot/evals/2026-05-25-mission-cu-macos-smoke-real-anthropic-loop.md`. Include: wall time, screenshot count taken, action count, final assertion result.
5. **If the test PASSes**: propose a follow-up brief `mission-claim-5-flip-live-in-claude-md` to flip the CLAUDE.md status block from 🟡 to 🟢.
6. **If the test FAILs**: file a follow-up brief per failure mode.

## Files you may touch

- `docs/skycot/evals/2026-05-25-mission-cu-macos-smoke-real-anthropic-loop.md` — new report.

## Files you must NOT touch

- `crates/argo-executor-computer-use-macos/**` — read-only.
- The smoke test source.
- The Anthropic API key in any committed form.

## Acceptance criteria

- [ ] Report file exists with a clear PASS/FAIL verdict + the output (key redacted).
- [ ] Test wall time documented.
- [ ] Follow-up brief proposed in the report.
- [ ] Operator perm state documented (granted or not).

## Branch + PR

- Branch: `claude-mission-cu-macos-smoke-run-with-real-anthropic-loop` or `codex-…`.
- PR title: `eval(cu-macos): end-to-end smoke against real Anthropic loop (mission-cu-macos-smoke-run-with-real-anthropic-loop)`.

## Notes

- Operator-only prerequisite: macOS perms. Confirm those are granted before claiming.
- This is the last LIVE-gate piece for claim-5 before the v1.0 demo MP4 records.
