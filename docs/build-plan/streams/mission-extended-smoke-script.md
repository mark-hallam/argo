# Mission load-bearing wire — extend smoke-skycot.sh with BrowserExecutor + Slack scenarios

**Owner**: claude. **Depends on**: `scripts/smoke-skycot.sh` SHIPPED (245 lines, 18 numbered checks); `tests/browser_action_executor_smoke.rs` SHIPPED (#1247); `tests/slack_approval_roundtrip_e2e.rs` SHIPPED (#1229).
**Authored against**: origin/main `5e1ef545` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-extended-smoke-script`.

> SOURCE: BUILD_STATUS.md header stop-condition: "extended `./scripts/smoke-skycot.sh` passes including BrowserExecutor + Slack-approval scenarios". Today the script runs 18 mock checks but exercises neither BrowserExecutor (M2) nor Slack roundtrip (M3) — the two Tier M MERGED items that v1 needs to demonstrate. This brief extends the script with 2 new numbered sections to invoke the existing `cargo test` integration smokes.

## Scope

ADD 2 new numbered check sections to `scripts/smoke-skycot.sh`:

- **#19 BrowserExecutor smoke**: invoke `cargo test -p argo-cli --test browser_action_executor_smoke --quiet` after a `command -v python3` guard (the test spawns `python3 tests/fixtures/browser_mock_mcp.py`). Skip-with-warn when python3 absent.
- **#20 Slack approval roundtrip**: invoke `cargo test -p argo-cli --test slack_approval_roundtrip_e2e --quiet`. No external dep (pure-Rust HMAC verification).

Match the existing pattern:
- Numbered header echo (`echo "# 19 BrowserExecutor smoke"`)
- Exit-on-fail with clear error
- Update launch_readiness expected count 18→20 in `docs/skycot/launch_readiness.md` if referenced

This is NOT in scope: making the script run a live Slack workspace (operator-only); driving BrowserExecutor through `argo` CLI from the script (use the `cargo test` integration smoke instead); rewriting any of the existing 18 checks.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `scripts/smoke-skycot.sh` — narrowly scoped: append 2 new check blocks before the final success message. Add a `command -v python3` guard for #19. Approximate +40 LOC.
- `docs/skycot/launch_readiness.md` — narrowly scoped: update pass-count references (search for `18/18` or `18 checks` and bump to `20/20` / `20 checks` where it refers to the smoke output).

## Files you must NOT touch

- Any `crates/` source (existing integration tests already cover the surfaces)
- `tests/fixtures/browser_mock_mcp.py` — Python fixture stays as-is
- Any non-script/non-doc file
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `scripts/smoke-skycot.sh` has new sections `# 19 BrowserExecutor smoke` and `# 20 Slack approval roundtrip`
- [ ] #19 guards python3 availability; warns-and-skips (not fails) when absent
- [ ] #20 unconditionally runs the slack roundtrip smoke
- [ ] All 20 checks pass on a clean repo: `bash scripts/smoke-skycot.sh` exits 0
- [ ] `launch_readiness.md` references 20-check count
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (no code changes — should be unaffected)
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-extended-smoke-script`
- PR title: `scripts: extend smoke-skycot with BrowserExecutor + Slack scenarios (mission-extended-smoke-script)`

## Notes for the implementer

- The python3 guard is critical — operators without python3 should see the smoke pass (with a warn line) rather than fail. Use `if ! command -v python3 >/dev/null 2>&1; then echo "SKIP #19 BrowserExecutor — python3 not in PATH"; else <run test>; fi`.
- Use `--quiet` flag on `cargo test` to keep smoke output readable.
- Match the existing block style exactly — copy a heading + step pattern from one of the existing checks.
- Don't add timing assertions or pass/fail percentages — just exit code propagation.
