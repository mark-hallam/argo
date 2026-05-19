# claude-rebase-pr-1799 — recover #1799 claude-rebase-pr-1749 (F-12 P2 recovery, second hop)

**Owner**: claude. **Recovery target**: PR [#1799](https://github.com/mark-hallam/Argo-Agent/pull/1799). **Original brief**: `streams/claude-rebase-pr-1749.md`.
**Authored against**: origin/main `da578d55` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1799`.
**Priority**: P2 — F-12 audit closure; on merge close #1749 SUPERSEDED + flip `codex-code-executor-sandbox-failclose` MERGED.

## Why this brief exists

PR #1799 is itself a recovery (Path B fresh-start) of closed #1749 — the original `codex-code-executor-sandbox-failclose` brief (fail-closed Local executor backend + LocalCodeExecution capability). Opened 2026-05-17 by claude worker; verified clean at open time per-crate clippy + 16/16 executor_code tests + workspace-clippy-fast-fail clean (117s).

Conductor attempted admin-merge in tick-2 (2026-05-19); `gh api` reports `mergeable=false`, `state=dirty`. 2 days of upstream traffic since 2026-05-17 have introduced conflicts — likely in `crates/argo-config/src/lib.rs` (the prior rebase already had to merge-resolve a `pub use sections::{...}` conflict; new sections re-exports may have landed).

This is the **second recovery hop** for this F-12 closure (1749 → 1799 → 1799-rebase). Avoid a third hop: this rebase should merge promptly once pushed.

## Acceptance: pick ONE recovery path

### Path A — rebase (strongly preferred — second-hop fatigue)

1. `git fetch origin && git checkout claude-claude-rebase-pr-1749`
2. `git rebase origin/main`. Expected conflicts:
   - `crates/argo-config/src/lib.rs` — `pub use sections::{...}` is a known hotspot; merge in any new exports landed since 2026-05-17 while preserving `CodeExecutorConfig` + `DEFAULT_CODE_EXECUTOR_PROFILE`.
   - Possibly `crates/argo-cli/src/handlers/worker_actions_executor_code.rs` if upstream touched the executor dispatcher.
   - Possibly `crates/argo-capability/src/lib.rs` if a new `Capability::*` variant landed and shifted the enum.
3. Resolution strategy: this PR's executor-related additions take precedence in the executor crate; for `argo-config/src/lib.rs` and `argo-capability/src/lib.rs`, ADD this PR's contributions alongside any upstream additions — these crates are accretion-only.
4. Verification per original brief: per-crate clippy (-p argo-cli -p argo-capability -p argo-types -p argo-config) clean; `cargo test -p argo-cli --bin argo worker_actions_executor_code` 16/16 pass; `bash scripts/workspace-clippy-fast-fail.sh` clean.
5. `git push --force-with-lease origin claude-claude-rebase-pr-1749`.
6. Comment on #1799: "✅ second-hop rebased + force-pushed onto origin/main `da578d55`. Worker on-call: please prioritise merging promptly to avoid a third hop."

### Path B — fresh start (LAST RESORT)

Only if Path A surfaces conflicts in >3 files outside the may-touch list. Close #1799 SUPERSEDED and open `claude-rebase-pr-1799-fresh` against today's main, re-applying the F-12 fail-closed pattern. Note: this becomes the THIRD recovery for the same intent; flag the operator for review before going this path.

## Acceptance criteria

- [ ] PR #1799 ends MERGEABLE / non-DIRTY against current main.
- [ ] `Capability::LocalCodeExecution` present in argo-capability.
- [ ] `[code_executor]` config section parses; `ProductionGate` enforces fail-closed on Local backend in production profile.
- [ ] `cargo clippy -p argo-cli -p argo-capability -p argo-types -p argo-config --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli --bin argo worker_actions_executor_code` 16/16 pass (3 gate tests included).
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] On merge, conductor flips `codex-code-executor-sandbox-failclose` to MERGED and closes #1749 with SUPERSEDED comment.
