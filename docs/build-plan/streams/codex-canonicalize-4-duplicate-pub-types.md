# codex-canonicalize-4-duplicate-pub-types — close CI parallel-pub-types lint (P0 audit-finding-1a)

**Owner**: codex. **Sizing hint**: fast-mode candidate — mechanical rename + allowlist edits.
**Depends on**: none.
**Authored against**: origin/main `6b432ec4` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-canonicalize-4-duplicate-pub-types`.
**Source**: 2026-05-17 read-only audit finding #1a (`docs/skycot/audit/2026-05-17-readonly-codebase-audit.md` lines 33-37). Plus 1 NEW collision since audit: `BundleFile` introduced by #1778 replay-export.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — CI invariant `parallel-pub-types-enforce` is currently RED on main. The lint runs as a blocking step in `ci.yml:30` but admin-merge bypasses it. This brief closes the gap.
- **Repeatability** (#3) — canonical-name discipline; one type, one home.

## Scope

P0. Resolve 4 unallowed parallel public type names. For each, pick canonical-home + rename or allowlist the deliberate-mirror cases.

### Cluster 1: `BundleFile`

- `crates/argo-cli/src/handlers/genesis_export.rs:34` (struct) — genesis export bundle file metadata
- `crates/argo-replay/src/bundle.rs:38` (struct) — replay export bundle file metadata

**Resolution:** Rename `argo-cli` one to `GenesisBundleFile`. Replay's `BundleFile` stays canonical (it's the generic bundle primitive in the replay crate; argo-cli's is a feature-specific view). ~10 modified LOC.

### Cluster 2: `Decision`

- `crates/argo-cli/src/handlers/gateway/routes_channels.rs:27` (enum) — channel-side notification decision
- `crates/argo-telemetry/src/cardinality.rs:34` (enum) — cardinality cap admin decision

**Resolution:** Rename `argo-telemetry` one to `CardinalityDecision`. Channel one stays as `Decision` (it's the older surface). ~8 modified LOC.

### Cluster 3: `Severity`

- `crates/argo-skill/src/pack_lint.rs:93` (enum) — pack-lint severity (error/warn)
- `crates/argo-types/src/anomaly.rs:82` (enum) — anomaly severity (info/low/medium/high/critical)

**Resolution:** Rename `argo-skill` one to `PackLintSeverity`. `argo-types::Severity` is canonical (used by audit + anomaly + observability surfaces). ~12 modified LOC across argo-skill callsites.

### Cluster 4: `StripeClient`

- `crates/argo-billing/src/stripe_client.rs:63` (struct) — internal stripe HTTP client
- `crates/argo-mcp-connector-stripe/src/client.rs:7` (struct) + `lib.rs:20` (type alias) — MCP-connector stripe client

**Resolution:** Add to `scripts/lint-parallel-pub-types.allowlist` as a deliberate mirror — both are stripe clients but serve genuinely different layers (billing webhook-driven vs MCP connector tool-driven). Allowlist entry with one-line justification. ~3 LOC in allowlist.

LOC: ~200 net (renames + allowlist + tests + import path updates).

## Files you may touch

**Renames + callsites:**
- `crates/argo-cli/src/handlers/genesis_export.rs` — rename + update self-references. ≤20 modified LOC.
- `crates/argo-cli/src/handlers/genesis_export_tests.rs` (if exists) — test updates. ≤15 modified LOC.
- `crates/argo-telemetry/src/cardinality.rs` — rename + update self-references. ≤15 modified LOC.
- `crates/argo-telemetry/src/cardinality_tests.rs` — test updates. ≤10 modified LOC.
- `crates/argo-cli/src/handlers/gateway/routes_metrics_cardinality.rs` — consume CardinalityDecision. ≤10 modified LOC.
- `crates/argo-skill/src/pack_lint.rs` + `pack_lint_checks.rs` — rename + propagate. ≤25 modified LOC.
- `crates/argo-skill/src/lib.rs` — re-export update. ≤4 modified LOC.
- `crates/argo-cli/src/handlers/skill_pack_lint.rs` — callsite. ≤8 modified LOC.

**Allowlist:**
- `scripts/lint-parallel-pub-types.allowlist` — add `StripeClient` entry with comment. ≤6 modified LOC.

## Files you must NOT touch

- The lint script itself (`scripts/lint-parallel-pub-types.py`) — consume only.
- Audit chain — read-only.
- Anything outside the 4 cluster surfaces.

## Acceptance criteria

- [ ] `python3 scripts/lint-parallel-pub-types.py` exits 0 (no unallowed duplicates).
- [ ] `python3 scripts/lint-parallel-pub-types.py --enforce` exits 0.
- [ ] All 4 clusters resolved per the resolution map above.
- [ ] `StripeClient` allowlist entry has a comment explaining "deliberate mirror — billing webhook vs MCP tool".
- [ ] `cargo test -p argo-cli -p argo-telemetry -p argo-skill -p argo-replay -p argo-billing` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` clean for touched crates.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-canonicalize-4-duplicate-pub-types`
- Title: `refactor: canonicalize 4 duplicate pub types — close parallel-pub-types-enforce gate (codex-canonicalize-4-duplicate-pub-types)`
- Worker verification: crate-scoped tests + clippy + workspace clippy + lint-parallel-pub-types --enforce per SWARM_PROTOCOL.md.

## Hidden constraints

- **Allowlist sparingly.** Default to rename; allowlist only when both surfaces are genuinely different concepts with their own audit/test lifecycle (StripeClient is that case).
- **Don't touch the canonical name's callsites.** Only the renamed surface needs updates — the canonical name stays the canonical name everywhere it's used.
- **Use `rg` to verify zero stragglers** post-rename: `rg 'BundleFile\b' crates/argo-cli/` etc.
- **Audit closes 1a.** When this merges, the audit's finding #1a is resolved on main.
