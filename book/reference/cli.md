# CLI reference

This page is auto-generated from `argo --help` recursively at
docs build time.

## Building this page locally

The book CI runs `scripts/build-cli-reference.sh` before
`mdbook build`. To preview locally:

```bash
cargo build --release -p argo-cli
./scripts/build-cli-reference.sh book/reference/cli.md
mdbook serve
```

The script walks every subcommand and writes a
`<details>`-collapsed help block per command into this page.

## Until the script runs

The complete CLI surface is large (130+ subcommands across
session / worker / memory / skill / channel / handoff / forge /
audit / budget / setup / vertical / roster / etc.). Quick
top-level pointers:

| Family | Examples |
|---|---|
| **Lifecycle** | `argo doctor`, `argo setup`, `argo onboard` |
| **Workspace** | `argo workspace init`, `argo backup create`, `argo restore` |
| **Vertical / Genesis** | `argo vertical list/show`, `argo genesis interactive/refine` |
| **Worker** | `argo work add/serve`, `argo worker show/pause/resume/list/anomalies` |
| **Actions** | `argo worker actions list/show/approve/execute/cancel`, `argo action replay` |
| **Memory** | `argo memory append/list/grep/forget`, `argo memory shared-team list/write` |
| **Skill** | `argo skill list/show/quality/search/publish/import/export`, `argo skill review list/show/approve/reject` |
| **Forge** | `argo forge optimize/bench/runs` |
| **Channels / Approvals** | `argo channel test`, `argo doctor channels`, `argo handoff list/show/cancel/rules` |
| **Budget** | `argo budget show/recent/anomalies/alerts/by-skill/watch` |
| **Audit** | `argo audit query/replay/export/report/verify`, `argo activity tail/since` |
| **Tenant / RBAC** | `argo tenant list/show/rename/quota`, `argo roster create/list/show`, `argo policy list/show/validate/apply` |
| **Compliance** | `argo gdpr delete`, `argo classify set/list/rules`, `argo feedback up/down` |
| **Plan / Runtime** | `argo plan generate/show/execute`, `argo subtask list/show/tree`, `argo workflow run/checkpoints/rollback` |
| **Recovery** | `argo recover <session-id>`, `argo recover-cache list/show` |
| **TUI / Voice** | `argo tui`, `argo voice` |
| **Daemon / Update** | `argo daemon install/uninstall`, `argo update` |
| **Cron / Schedule** | `argo cron list/show/create/delete` |
| **Trajectory / Eval** | `argo trajectory export`, `argo eval run/list/show/compare` |

## CLI flags universal across most commands

| Flag | Effect |
|---|---|
| `--json` | Emit a JSON envelope instead of human text |
| `--format human/json/csv` | Three-format output (where applicable) |
| `--tenant <id>` | Tenant scope (multi-tenant only) |
| `--vertical <id>` | Vertical scope |
| `--worker <id>` | Worker scope |
| `--since <duration>` | Time-window filter (e.g. `1h`, `7d`) |
| `--limit <n>` | Cap result count |
| `--dry-run` | Plan only, no side effects |
| `--non-interactive` | Read inputs from `<workspace>/.skycot/<cmd>.toml` |

## Auto-generation

The `scripts/build-cli-reference.sh` script discovers subcommands
by parsing the top-level `argo --help` output and recursively
descending into each subcommand. Output is appended to this file
between two HTML markers; the rest of the page is preserved.

```text
<!-- BEGIN AUTO-GENERATED CLI REFERENCE -->
…content emitted by the script…
<!-- END AUTO-GENERATED CLI REFERENCE -->
```

If the markers are missing, the script appends a fresh block at
the bottom.

<!-- BEGIN AUTO-GENERATED CLI REFERENCE -->

_(Run `scripts/build-cli-reference.sh book/reference/cli.md` to populate this section.)_

<!-- END AUTO-GENERATED CLI REFERENCE -->
