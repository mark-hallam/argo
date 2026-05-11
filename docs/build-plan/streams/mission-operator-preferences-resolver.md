# Mission load-bearing wire — per-operator central preferences resolver

**Owner**: claude. **Depends on**: existing `argo-channels` crate (multi-channel approval surface — Slack, Discord, iMessage, WhatsApp, Email, Voice, SMS, just-merged WebSocket inbox via #1329); existing `argo-identity::Operator` type (verify in first 5min via `rg -l 'struct Operator' crates/argo-identity/src/`).
**Authored against**: origin/main `55047304` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-operator-preferences-resolver`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 #120 ("Add per-operator central preferences resolver"). Today each channel reads its own config silos — Slack reads `[channels.slack].operator_*`, Email reads `[channels.email].*`, Voice reads its own per-route config. There's no single seam an operator can hit to say "for me specifically, route approvals to Signal, fall back to iMessage after 10 min, never SMS, quiet hours 22:00–07:00." This brief lands the central preferences resolver so the dispatcher consults ONE source of truth before picking a channel.

## Scope

NEW module `crates/argo-channels/src/operator_preferences.rs` (≤ 280 LOC) — the resolver primitive:

```rust
pub struct OperatorPreferences {
    pub operator_id: OperatorId,
    pub primary_channel: ChannelKind,
    pub fallback_chain: Vec<ChannelChainStep>,
    pub quiet_hours: Option<QuietHours>,
    pub channel_overrides: HashMap<RiskLevel, ChannelKind>,
}
pub struct ChannelChainStep { pub channel: ChannelKind, pub after_seconds: u32 }
pub struct QuietHours { pub start_local: NaiveTime, pub end_local: NaiveTime, pub timezone: chrono_tz::Tz, pub honor_for: HashSet<ChannelKind> }
pub enum RiskLevel { Low, Medium, High, Critical }

pub trait OperatorPreferencesSource: Send + Sync {
    async fn for_operator(&self, op: &OperatorId) -> ArgoResult<OperatorPreferences>;
}

pub struct OperatorPreferencesResolver { source: Arc<dyn OperatorPreferencesSource> }
impl OperatorPreferencesResolver {
    pub fn new(source: Arc<dyn OperatorPreferencesSource>) -> Self;
    pub async fn resolve_initial(&self, op: &OperatorId, risk: RiskLevel, now: DateTime<Utc>) -> ArgoResult<ChannelKind>;
    pub async fn next_fallback(&self, op: &OperatorId, current: ChannelKind, elapsed_since_send: Duration, now: DateTime<Utc>) -> ArgoResult<Option<ChannelKind>>;
}
```

Wired with two concrete `OperatorPreferencesSource` impls in this brief:

1. **`TomlPreferencesSource`** in same crate — reads `[operator.<id>.preferences]` blocks from `argo.toml`. Reload on file-mtime change (cached, lock-free).
2. **`InMemoryPreferencesSource`** — test-only, accepts a `HashMap<OperatorId, OperatorPreferences>` at construction.

Dispatcher integration in `argo-channels` (the existing dispatch site) consults the resolver ONCE per approval-send decision; channel-specific config silos remain authoritative for in-channel rendering (Slack block-kit, Email MIME, etc.).

This is NOT in scope: per-operator preferences UI (separate frontend brief, in Track-D blocked queue); preferences-from-store (sqlite-backed source — separate brief once schema lands); preferences inheritance from tenant defaults (operator → tenant fallback chain — separate); the dispatch loop rewrite (this brief delivers the primitive + wires ONE pilot path; rest of dispatch sites migrate in follow-ups).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-channels/src/operator_preferences.rs` (NEW, ≤ 280 LOC) — types + `OperatorPreferencesResolver` + `TomlPreferencesSource` + `InMemoryPreferencesSource`
- `crates/argo-channels/src/operator_preferences_tests.rs` (NEW, sibling, ≤ 350 LOC) — 9 unit tests w/ `InMemoryPreferencesSource`: primary-no-quiet, quiet-honored, quiet-not-honored-by-channel-set, risk-override-trumps-primary, fallback-after-elapsed-returns-next, fallback-exhausted-returns-None, timezone-correctly-applied, mtime-reload-toml-source, file-missing-returns-typed-error
- `crates/argo-channels/src/lib.rs` — narrowly scoped: ONE `pub mod operator_preferences;` + re-exports
- `crates/argo-channels/Cargo.toml` — narrowly scoped: ADD `chrono-tz` + `notify` (file watcher) if not already present; verify first via `rg -l chrono-tz crates/argo-channels/`
- `crates/argo-channels/tests/preferences_smoke.rs` (NEW, ≤ 180 LOC) — integration smoke: write a temp `argo.toml` w/ 2 operator blocks; instantiate `TomlPreferencesSource`; resolve initial + fallback; mutate file + assert reload-on-mtime picks up the change
- ONE pilot wiring site in `argo-channels` dispatch (the existing call site that picks a channel today) — narrowly scoped: consult resolver, fall through to legacy config if resolver returns `ChannelKind` matching current default

## Files you must NOT touch

- `crates/argo-channels/src/{slack,email,sms,voice,whatsapp,discord}.rs` — channel-specific dispatch files; they don't change, the resolver is consulted BEFORE them
- `crates/argo-identity/**` — `Operator` / `OperatorId` types are stable; reuse only
- Any frontend / web crate
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `OperatorPreferences` round-trips through TOML with all four fields (primary_channel / fallback_chain / quiet_hours / channel_overrides)
- [ ] `resolve_initial` returns the primary channel when no quiet hours apply AND no risk override matches
- [ ] `resolve_initial` returns the first non-quiet-hour fallback when primary is in quiet
- [ ] `channel_overrides[risk]` trumps primary when present for that risk level
- [ ] `next_fallback` returns `Some(next)` when `elapsed >= after_seconds` of the next step; returns `None` when chain exhausted
- [ ] `QuietHours.honor_for` correctly excludes channels not in the set (e.g. critical-Voice still rings)
- [ ] `TomlPreferencesSource` reloads on file-mtime change (one tick of mtime check; no inotify required)
- [ ] Pilot dispatch site consults resolver; channel selection behavior unchanged for operators with no `[operator.<id>.preferences]` block (resolver returns same default)
- [ ] 9 unit tests + 1 integration smoke
- [ ] `cargo clippy -p argo-channels --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-channels` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-operator-preferences-resolver`
- PR title: `channels: per-operator central preferences resolver (primary + fallback + quiet hours + risk override) (mission-operator-preferences-resolver)`

## Notes for the implementer

- This brief is INTENTIONALLY scoped to the resolver primitive + TOML source + one pilot wire. Migrating every dispatch site to consult the resolver is queued as a follow-up brief once this lands and proves out.
- Quiet hours math: convert `now` to the operator's timezone, compare to `start_local / end_local` as `NaiveTime`. Handle the wrap-around case (e.g. 22:00–07:00 spans midnight) by checking `now >= start || now < end` vs the non-wrap case `start <= now < end`.
- File watcher: `notify` crate is fine but heavyweight; an mtime-poll-per-resolve (cached, 5s TTL) is simpler and the brief's acceptance criterion explicitly allows that. Choose simpler.
- `OperatorId` is a wrapper around UUID per `crates/argo-identity/src/types.rs`; the TOML key is the string representation (e.g. `[operator.018f-...preferences]`). Document this in module docs.
- The `channel_overrides[risk]` map is sparse — absent risk levels fall through to primary. Don't require operators to enumerate all 4 levels.
- Resist scope creep into store-backed preferences or UI surfacing — keep this brief's may-touch narrow.
