# claude-delete-orphan-shape-b-connectors — pure delete: 16 connector crates with zero consumers (~6,300 LOC)

**Owner**: claude. **Depends on**: nothing (pure delete).
**Authored against**: origin/main `bf8236e3` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-delete-orphan-shape-b-connectors`.

> **PURE DELETE 2026-05-14 by conductor (CEO-authorized Phase 1 cleanup).** Audit cluster 4 found 16 `argo-mcp-connector-*` "shape-B" crates with **zero workspace consumers** — no `[[bin]]`, no path/version dep from any other Cargo.toml. Pure dead code. ~6,300 LOC.

## Scope

Delete the following 16 connector crates entirely:

- `crates/argo-mcp-connector-apollo`
- `crates/argo-mcp-connector-box`
- `crates/argo-mcp-connector-clearbit`
- `crates/argo-mcp-connector-dropbox`
- `crates/argo-mcp-connector-greenhouse`
- `crates/argo-mcp-connector-lever`
- `crates/argo-mcp-connector-mailgun`
- `crates/argo-mcp-connector-microsoft-teams`
- `crates/argo-mcp-connector-quickbooks`
- `crates/argo-mcp-connector-replicate`
- `crates/argo-mcp-connector-resend`
- `crates/argo-mcp-connector-signal`
- `crates/argo-mcp-connector-telegram`
- `crates/argo-mcp-connector-whatsapp-cloud`
- `crates/argo-mcp-connector-xero`
- `crates/argo-mcp-connector-zoom`
- `crates/argo-mcp-connector-zoominfo`

**That's 17 if you count zoominfo as separate from zoom**; audit-4 reported 16. **Implementer verifies count at claim time** — claim brief refers to the SET, not the cardinality.

Vertical-side cleanup: 4 vertical.toml files reference these connectors as `transport.command = "..."` but the connectors have no `[[bin]]` (so subprocess spawn would fail at runtime). Remove the broken references:
- `verticals/lead-research/vertical.toml` — remove `argo-mcp-connector-clearbit`, `argo-mcp-connector-zoominfo`, `argo-mcp-connector-dropbox` entries
- `verticals/bookkeeping-assistant/vertical.toml` — remove `argo-mcp-connector-quickbooks` entry

Workspace-level: remove the 17 entries from root `Cargo.toml` `[workspace.members]` list.

**Not in scope**: deleting shape-A connectors (those have bins + actual consumers); replacing with stubs; building real implementations.

## Files you may touch

- `crates/argo-mcp-connector-*/` (17 directories) — `rm -rf` each via `git rm -r`.
- `Cargo.toml` (workspace root) — remove the 17 `[workspace.members]` entries.
- `verticals/lead-research/vertical.toml` — remove 3 transport.command entries pointing at deleted connectors.
- `verticals/bookkeeping-assistant/vertical.toml` — remove 1 transport.command entry.
- `Cargo.lock` — regenerate via `cargo check`.

## Files you must NOT touch

- Any shape-A connector (the ones with `[[bin]]` and active consumers — gmail, github-issues, slack, hubspot, salesforce, etc.).
- `argo-mcp-connector-shared` — utility crate; keep.
- `argo-mcp-image-gen` — Argo-shipped image-gen server; keep (audit 4 confirmed it's wired).
- Any vertical.toml that doesn't reference one of the 17 deleted connectors.

## Acceptance criteria

- [ ] 17 connector directories removed via `git rm -r`.
- [ ] Workspace `Cargo.toml` no longer lists the 17 entries.
- [ ] 4 vertical.toml files cleaned of broken references.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes (no regressions; the deleted crates' tests go with them).
- [ ] Net LOC reduction: ~6,300 (verify via `git diff --stat`).
- [ ] No breakage in `verticals/lead-research/eval/*.jsonl` or `verticals/bookkeeping-assistant/eval/*.jsonl` if those reference the deleted connectors — if they do, flag for follow-up cleanup brief.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-delete-orphan-shape-b-connectors`
- PR title: `cleanup: delete 17 orphan shape-B connector crates (~6,300 LOC) (claude-delete-orphan-shape-b-connectors)`

## Notes for the implementer

- This is the highest-LOC-per-effort cleanup brief in the queue. Pure delete; zero behavior change.
- If any of the 17 connectors has a sneaky consumer (e.g. a docs example, a test fixture, a CI workflow), `cargo build --workspace` will fail. Inspect the failure and either remove the reference too OR keep that one connector (file follow-up note).
- Per CLAUDE.md "subtract before adding" — this is the most-leverage subtract pass in the queue.
- The shape-B crates that survive (because they have hidden consumers): document each in the PR body.
- Operator-visible: the 4 verticals that referenced these connectors were already broken (paper-only). This brief just stops pretending.
