# V1.4 — Credentials management CLI

**Owner**: codex. **Depends on**: nothing.

## Scope

Today operators set credentials via env vars (POSTMARK_API_KEY,
TWILIO_AUTH_TOKEN, ANTHROPIC_API_KEY, etc.) — fragile,
error-prone, doesn't survive across shells. Add credentials
management:

- `argo cred set <key> [<value>]` — prompts (hidden) if value
  omitted; stores in `<workspace>/.argo/credentials.toml` with
  mode 0600.
- `argo cred list [--reveal]` — list keys (values masked unless
  `--reveal`).
- `argo cred clear <key>` / `argo cred clear-all`.
- `argo cred export --shell bash|zsh|fish` — emit `export FOO=bar`
  lines for piping into a shell rc file.
- `argo cred import --shell bash|zsh` — pull from current env.

Existing code paths reading env vars get a fallback: if env is
unset, read from credentials.toml. New helper
`argo_workspace::cred::get(key)` covers this.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/cred_*.rs` (5 NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 5 pub mods
- `crates/argo-cli/src/cli_cred_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: register
  `Cred` subcommand
- `crates/argo-config/src/cred.rs` (NEW — `argo-config` is the existing
  config crate; tick58 amendment applies, no new crate needed)
- `crates/argo-config/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/tests/cred_smoke.rs` (NEW)

### Scope-clarification amendment (tick137 — unblock)

The brief said "existing env-var-reading code gets a fallback". Codex
correctly noted those readers are not in the may-touch list. **Resolved
by descope, not expansion**: the fallback wiring is moved to a follow-up
brief `codex-credentials-fallback-wiring` (queued separately).

This brief now ships ONLY:
- `argo cred set/list/clear/clear-all/export/import` CLI surface
- `argo-config::cred::get(key)` helper that reads credentials.toml
  first, falls back to env::var

Workers continue reading env vars directly until the follow-up brief
threads `argo-config::cred::get(key)` through them. v1 unaffected:
operators set env vars as before, OR use `argo cred set` AND export
to shell. Choice; no breakage either way.

The follow-up brief is mechanical and small (each reader is one line:
`std::env::var("X")?` → `argo_config::cred::get("X")?`).

## Acceptance criteria
- [ ] All 5 subcommands work end-to-end
- [ ] credentials.toml has file mode 0600
- [ ] `--reveal` actually reveals; default masks
- [ ] Existing env-var-reading code still works (fallback to
      credentials.toml only when env unset)
- [ ] `argo cred export --shell bash` emits valid bash
- [ ] 6 unit tests + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-credentials-management-cli`
- PR title: `cli: argo cred — credentials management (codex-credentials-management-cli)`
