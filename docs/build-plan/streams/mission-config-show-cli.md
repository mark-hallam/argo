# mission-config-show-cli — `argo config show` (post-v1 polish P2)

**Owner**: codex. **Depends on**: none.
**Authored against**: origin/main `e2fe4b70` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-config-show-cli`.
**Source**: post-v1 polish — operator support triage often needs "what config is the running argo actually using?"

## Mission alignment

- **Observability** (CLAUDE.md priority #2) — config sources (file + env + default) are layered; surface the effective merged view.
- **Reliability** (#1) — debugging "but my env var should override" is impossible without this.

## Scope

P2. New CLI subcommand:
```bash
argo config show                     # full effective config (TOML)
argo config show --key gateway.bind  # narrow to one key
argo config show --json              # JSON output for scripting
argo config show --source            # annotate where each value came from
```

`--source` output sample:
```toml
[gateway]
bind = "127.0.0.1:8090"     # source: argo.toml at /Users/.../argo.toml
port = 8090                  # source: env ARGO_PORT
log_level = "info"           # source: default
```

Secrets:
- Any field of type `Redacted<T>` (post `mission-secret-redaction-newtype` MERGED) prints `"[redacted]"`. **If that brief hasn't merged yet, the field-by-name allowlist of "api_key|secret|token|password" matches print `"[redacted]"` regardless.**
- `--show-secrets` flag prints real values; requires `ARGO_SHOW_SECRETS=1` env var too (belt + suspenders).

LOC: ~150.

## Files you may touch

- `crates/argo-cli/src/cli_config_cmds.rs` (NEW or existing) — `Show { key, json, source, show_secrets }` variant. ≤30 LOC.
- `crates/argo-cli/src/handlers/config_show.rs` (NEW) — handler. ≤80 LOC.
- `crates/argo-cli/src/handlers/config_show_tests.rs` (NEW) — fixture-driven (golden TOML/JSON output). ≤80 LOC.
- `crates/argo-config/src/source_tracking.rs` (NEW) — optional helper to track origin of each value during load (file path vs env var vs default). ≤80 LOC.
- `crates/argo-config/src/lib.rs` — narrowly: expose source-tracking. ≤8 modified LOC.
- `docs/skycot/cli-reference.md` — add `config show` row. ≤8 modified LOC.

## Files you must NOT touch

- The actual config-load logic — read-only. Don't restructure how config merges.
- Test fixtures for argo-config — read-only.
- Any operator config files outside the repo.

## Acceptance criteria

- [ ] `argo config show` prints the full effective config as valid TOML.
- [ ] `argo config show --key gateway.bind` prints just that key's value.
- [ ] `argo config show --json` emits valid JSON parseable by `jq`.
- [ ] `argo config show --source` annotates each line with origin.
- [ ] Fields matching `api_key|secret|token|password` print `[redacted]` unless `--show-secrets` AND `ARGO_SHOW_SECRETS=1` both set.
- [ ] Round-trip: `argo config show > /tmp/x.toml && argo --config /tmp/x.toml ...` produces an instance with same effective config.
- [ ] `cargo test -p argo-config -p argo-cli` passes.
- [ ] `cargo clippy -p argo-config -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-config-show-cli`
- Title: `feat(cli): argo config show with source annotation + redaction (mission-config-show-cli)`
- Worker verification: crate-scoped clippy + tests + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **TOML output must round-trip.** A common bug: `toml::Value::String("foo")` writes `"foo"` but a typed `String` field at root writes `bar = "foo"`. Use the typed serializer.
- **Source tracking is optional.** If implementing source-tracking is too invasive, ship without it (and skip the `--source` flag); just emit the effective config. Brief degrades gracefully.
- **Redaction precedence**: type system (`Redacted<T>`) > name-pattern. Once `Redacted` lands, drop the name-pattern fallback.
- **`--key` is dotted-path.** Use `serde_json::Value::pointer("/gateway/bind")` style or hand-roll a 10-line dotted lookup.
- **Don't include compile-time defaults** that are never wired into runtime config. Only show fields that ARE in the effective config struct.
- **JSON null vs missing.** Use `serde_json::to_value(&config).unwrap()` and walk; if a field is `Option::None` skip it in `--source` output (don't print "source: default" for unset Options).
