# claude-rebase-pr-1778 — recover #1778 mission-replay-cli-bundle-export (P1)

**Owner**: claude. **Recovery target**: PR [#1778](https://github.com/mark-hallam/Argo-Agent/pull/1778). **Original brief**: `streams/mission-replay-cli-bundle-export.md`.
**Authored against**: origin/main `da8e0e0e` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1778`.

## Why this brief exists

PR #1778 (`argo replay export <session_id>` tarball) opened 2026-05-16T13:38 and stayed DIRTY ≥24h. Conflict surface: argo-replay + audit_export.rs (#1769 audit-csv-export landed in admin-wave-2 with overlapping crate scope).

## Acceptance: pick ONE recovery path

### Path A — rebase

1. `git fetch origin && git checkout mission-replay-cli-bundle-export`
2. `git rebase origin/main`. Expect conflicts in:
   - `crates/argo-replay/src/` (audit-csv-export PR didn't touch this directly, but config-show added a sibling)
   - `crates/argo-cli/src/handlers/audit_export.rs` (#1769 landed; check if the replay-export module wants to reuse its sanitize helpers)
3. Verification per brief: `cargo test -p argo-replay -p argo-cli`, `cargo clippy --all-targets -- -D warnings`.
4. `git push --force-with-lease origin mission-replay-cli-bundle-export`.

### Path B — fresh start

1. Re-read `streams/mission-replay-cli-bundle-export.md`.
2. Note that #1769 audit-csv-export has shipped — consume its sanitize/canonical-form helpers instead of re-rolling.
3. Implement on `mission-replay-cli-bundle-export-v2`.
4. Close #1778.

**Path A is fine** unless the prior worker's bundle module is too tightly coupled to the pre-1769 audit world; if so, Path B + reuse #1769's helpers is cleaner.

## Acceptance criteria

- [ ] PR ends MERGEABLE / non-DIRTY.
- [ ] Bundle layout per original brief: `manifest.toml + audit/chain.jsonl + skills/ + memory/ + bundle/ + README.md`.
- [ ] SHA-256 manifest checksum.
- [ ] `--sanitize` flag works; redacts secrets/PII.
- [ ] BUILD_STATUS.md row flipped TODO → IN_REVIEW.

## Hidden constraints

- **`--sanitize` reuses `argo-audit::canonical_payload` (or audit_export.rs's sanitize)** — don't fork the logic.
- **Memory rows export via audit walk** — no separate `session_id` column on memory.
- **Streaming tar writer**, not buffered.
