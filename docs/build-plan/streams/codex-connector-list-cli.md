# V1.4 — Connector enumeration CLI

**Owner**: codex. **Depends on**: nothing (pure read-side over
existing connector manifests).

## Scope

Today operators have to read source code or `manifests/connectors/`
filesystem to know what connector kinds exist. `argo connector
list-status` (queued in `codex-connector-authorize-cli`) shows
auth state per connector — but only for connectors that already
have manifests.

Add a simple enumeration that walks the connector crates + manifest
directory to show what kinds are available, what their auth model
is, and whether the connector is currently configured:

- `argo connector list [--installed-only] [--json]` — markdown table
  of `kind`, `auth_type` (oauth/token/none), `installed` (bool —
  manifest present), `methods` (count of MCP methods exposed).
- `argo connector show <kind> [--json]` — single-connector detail:
  manifest contents + method list + required env/credentials.

Pure read-side. No mutation. No new state. Reads from
`manifests/connectors/*.toml` and the connector crate metadata.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/connector_list.rs` (NEW)
- `crates/argo-cli/src/handlers/connector_show.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 2 pub mods
- `crates/argo-cli/src/cli_connector_cmds.rs` — narrowly: add
  `List` and `Show` variants. **Note**: `cli_connector_cmds.rs`
  does not yet exist on main as of tick180 — it will be created
  by `codex-connector-authorize-cli` (#905, currently NEEDS REWORK).
  Wait until #905 lands before claiming this brief, OR extend this
  brief's scope to create `cli_connector_cmds.rs` if #905 is still
  hanging when this is claimed.
- `crates/argo-cli/tests/connector_enum_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo connector list` returns markdown table with all known
      kinds (verifiable via grep of `manifests/connectors/`)
- [ ] `--installed-only` filters to connectors with present manifests
- [ ] `argo connector show gmail` returns a manifest dump + method
      count
- [ ] `--json` envelope universal across both subcommands
- [ ] 4 unit tests + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-connector-list-cli`
- PR title: `connector: argo connector list/show — enumerate available connector kinds (codex-connector-list-cli)`
