# codex-audit-chain-query-perf-at-scale — audit-chain query perf at 100k/1M rows (P2 operational-audit blind-spot)

**Owner**: codex. **Sizing hint**: fast-mode adjacent — bench scaffold, no architectural decisions.
**Depends on**: none.
**Authored against**: origin/main `37f268a6` on `2026-05-17`.
**Validate before launch**: `bash scripts/check-brief-freshness.sh codex-audit-chain-query-perf-at-scale`.
**Source**: 2026-05-17 audit blind-spot — `audit_record` table grows unbounded. `verify_session()`, `verify_all()`, and `/audit/feed?scope=*` are operator-facing surfaces with unknown perf characteristics at 100k+ rows.

## Mission alignment

- **Observability** (CLAUDE.md priority #2) — audit-chain is the operator's incident-response surface. Slow queries during an incident = unusable.
- **Reliability** (#1) — at 1M rows, naïve scan = minutes-long verify; document the actual numbers.

## Scope

P2. Three benches in `crates/argo-audit/benches/chain_perf.rs`:

1. **`verify_session` at row counts.**
   - Seed a fresh DB with N audit rows in a single session (chained via `parent_hash`).
   - Run `verify_session(session_id)`; measure wall-clock + memory peak.
   - Profiles: N = 1k / 10k / 100k / 1M.

2. **`verify_all` (chain sweep CLI) at session counts.**
   - Seed S sessions × 1k rows each.
   - Run `verify_all()`; measure wall-clock per-session + memory peak.
   - Profiles: S = 10 / 100 / 1000.

3. **`/audit/feed?scope=tenant&since=24h` at row counts.**
   - Seed via HTTP POST to gateway (or direct DB) at scaled counts.
   - Hit `/audit/feed` with the operator's typical query shape.
   - Measure server-side query plan time + first-byte latency.
   - Profiles: 1k / 10k / 100k rows in the requested window.

### Output

`cargo bench -p argo-audit chain_perf > /tmp/audit-chain-perf.txt`. Bench results captured in standard criterion output format. PR body includes a summary table with numbers at each scale point.

### Index recommendations (out of scope but document)

If any bench surfaces > 1s wall-clock at the 100k scale, PR body documents which query would benefit from a covering index. Index creation itself is a follow-up brief (don't change schema in this brief).

LOC: ~250.

## Files you may touch

- `crates/argo-audit/benches/chain_perf.rs` (NEW) — 3 bench groups. ≤200 LOC.
- `crates/argo-audit/Cargo.toml` — narrowly: add `[[bench]]` entry + `criterion` dev-dep if not present. ≤8 modified LOC.
- `crates/argo-audit/benches/fixtures/seed_chain.rs` (NEW) — chain-seeding helper. ≤100 LOC.
- `docs/skycot/operator/audit-chain-perf.md` (NEW) — operator runbook + result interpretation. ≤60 LOC.

## Files you must NOT touch

- `argo-audit/src/verify.rs` — read-only consumer.
- `argo-audit/src/sink.rs` — read-only.
- DB schema — pure bench observation. Schema changes are follow-up.

## Acceptance criteria

- [ ] `cargo bench -p argo-audit chain_perf` runs end-to-end (may take minutes at 1M scale — that's expected).
- [ ] Bench results captured in standard criterion HTML output.
- [ ] PR body includes a results table per profile, e.g. `verify_session 1k=Xms / 10k=Yms / 100k=Zs / 1M=Ws`.
- [ ] If any profile exceeds 5s wall-clock, PR body flags which queries would benefit from indexing (without changing schema).
- [ ] Operator runbook documents: how to run benches, how to interpret growth curves, when to file an index brief.
- [ ] `cargo test -p argo-audit` passes (bench seeding shouldn't regress existing tests).
- [ ] `cargo clippy -p argo-audit --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-audit-chain-query-perf-at-scale`
- Title: `bench(audit): chain query perf at 1k/10k/100k/1M row scale (codex-audit-chain-query-perf-at-scale)`
- Worker verification: crate-scoped clippy + bench dry-run + per SWARM_PROTOCOL.md.

## Hidden constraints

- **Fixture seeding is the bottleneck.** Seeding 1M rows takes minutes; criterion's setup phase handles this — but cap it at `Iterations=10` for the 1M profile or it'll timeout.
- **Real chain hash computation per seed row.** Don't shortcut the hash — the bench is about real verify perf.
- **In-memory SQLite is faster than file-backed.** Document which profile is which; recommend file-backed for the operator runbook (matches production).
- **No production DB queries.** Bench uses a temp DB; no risk of perturbing real data.
- **Don't add `[[bench]]` to crates that don't have criterion already.** argo-audit needs the dev-dep added.
- **WAL mode** affects perf significantly. Bench seeds with `journal_mode=WAL` to match production.
