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
descending into each subcommand. Output is spliced into this file
between two HTML comment markers (`BEGIN AUTO-GENERATED CLI
REFERENCE` / `END AUTO-GENERATED CLI REFERENCE`); the rest of the
page is preserved. If the markers are missing, the script appends
a fresh block at the bottom.

Drift CI (`.github/workflows/cli-reference-drift.yml`) re-runs
the script on every PR that touches `crates/argo-cli/**` and
fails if the regenerated page differs from what is checked in.
Operator response: run the script locally and commit the diff.

<!-- BEGIN AUTO-GENERATED CLI REFERENCE -->
_Auto-generated from `argo --help` recursively. Do not edit by
hand — re-run `scripts/build-cli-reference.sh` to regenerate._

## `argo`

```text
Argo is the agent framework that one engineer can fully audit on a weekend.
See `docs/plan/argo-build-plan.md` for the full master spec.

Usage: argo [COMMAND]

Commands:
  agents            
  gateway           
  mcp               
  plugin            
  packs             Install and manage vertical template packs
  provider          
  model             
  skill             
  memory            
  cron              
  cred              Workspace credential storage and shell export helpers
  backup            Workspace backup create/list/verify
  restore           Restore a workspace backup tarball
  migrate           Data migrations (`mission-sqlcipher-3-migrate-cli`)
  daemon            macOS launchd-agent install/uninstall (Stream `claude-installer-macos`). Writes `~/Library/LaunchAgents/com.skycot.argo.plist` so the gateway starts on login + auto-restarts on crash. macOS-only
  connector         Manage OAuth and token credentials for MCP connectors
  work              Work queue v0 (#716). `add` enqueues a unit of work for a vertical-bound worker; `serve` runs a persistent loop that processes the queue using existing session + memory + chat primitives. Default provider `mock` works without an API key
  worker            Worker v3 execution-mode action ledger (#728). Inspect (and in later PRs: approve / execute / complete) the actions the worker has proposed. Distinct from `argo work` — this surface is the noun (the worker's output), not the verb (running the worker)
  approval          Approval audit surfaces
  compliance        Compliance surfaces (Phase C). Read-only over `audit_record` and tenant-scoped tables. - `soc2` — SOC2 Common Criteria 6.x audit-trail report (`claude-compliance-soc2-audit-trail`). - `export` — GDPR Article 15 right-to-access JSON envelope for one tenant; the export is itself audit-logged (`claude-compliance-gdpr-data-export`)
  tenant            
  operator          Operator-scoped surfaces. Currently: - `risk-overrides` — per-operator overrides of the manifest-driven default risk classification (Stream `mission-operator-specific-risk-overrides`). Upgrade-only by default; downgrades require `--allow-downgrade` so a compromised account can't silently relax approvals
  persona           Persona maintenance — `$EDITOR`-driven update of the human-facing persona row attached to one or more workers (Stream `claude-persona-edit-cli`)
  sandbox           Cloud-sandbox lifecycle ops (Stream `mission-e2b-reap-idle`). `e2b list` / `e2b reap-idle` for out-of-band cleanup of leaked E2B sandboxes — orthogonal to the CU executor's in-band Drop path
  org               Org chart, addressable names, manager chain (`claude-org-awareness`)
  roster            Worker rosters: group-level policy, channels, budget cap (`claude-worker-roster`)
  team              Org-chart-aware teams: the operator-facing bridge between `argo org` (the tree) and `argo roster` (group policy) (Stream `claude-worker-team-roster-cli`). Each `team` is a single id that resolves uniquely in both `org_unit` (name + parent link) and `roster` (default policy bundle)
  policy            Declarative org policies (Stream `claude-policy-as-code`): list, show, validate, apply YAML files at `<workspace>/policies/<name>.yaml` that constrain the auto-approval gate via the resolution chain action → worker → roster → workspace-file → default
  handoff-rules     Declarative handoff routing rules (`claude-handoff-routing-rules`)
  duty              Worker duty scheduler — autonomous cron-driven actions (Stream `claude-worker-duty-scheduler`, V1.3). list / show / pause / resume / fire-now
  handoff-auctions  Inspect handoff fallback auctions (`claude-skill-auction`). When the routing chain exhausts, the walker may record a `no_bidders` auction row in the audit trail; this surface lets operators read + cancel those
  handoff-policy    Workspace-level handoff policy (`claude-worker-handoff-policy`). Reads `.skycot/handoff-policy.toml` and answers "should we hand off at all?" via a first-match-wins rule list
  handoff-stats     Aggregate metrics over the orchestrator's `handoff` table (Stream `codex-handoff-stats-cli`). Markdown report by default; `--histogram` switches to an ASCII volume readout
  workflow          Declarative DAG workflows across multiple workers (Stream `claude-workflow-dag`). YAML schema describing tasks + edges; runs each task as a Session with output binding from upstream tasks
  trajectory        Export action trajectories for fine-tuning data pipelines (claude-trajectory-training-export)
  training          Export successful action trajectories as fine-tuning JSONL
  action            Action debugging tools that do not mutate the action ledger
  activity          
  errors            Error-rate dashboards backed by canonical audit rows
  sla               Availability/SLA dashboards backed by canonical audit rows
  audit             
  vertical          Vertical-template discovery surface (#650). Read-only — `list`, `show`, `validate`. Install / eval / run land in later PRs
  genesis           Charter → vertical generator (#G-genesis). Operator describes a role in plain English; Genesis emits a complete `verticals/<id>/` tree with safety judge guarantees on every action
  messages          Workspace-wide message rollups. Mirrors the gateway's `/messages/*` family
  workspace         
  config            Inspect the effective merged Argo configuration
  db                
  storage           
  costs             
  budget            
  session           
  handoff           
  forge             
  judge             
  route             
  eval              Validate eval JSONL, or run/list vertical eval fixtures
  incident          Walk the operator through a blameless postmortem (Stream `claude-incident-postmortem-template`). Reads stdin section by section, or `--from <yaml>` for non-interactive scripts; writes `docs/skycot/incidents/<YYYY-MM-DD>-<slug>.md`
  feedback          Operator-supplied thumbs up/down per action (Stream `claude-operator-feedback-channel`). Feeds the skill-learning loop as a strong-positive / strong-negative signal alongside the heuristic success metric
  classify          Tag records (memory rows, audit payloads, action params) with data classes (Stream `claude-data-classification`): `pii` / `phi` / `financial` / `secret` / `public` / `untagged`. Drives per-class retention windows and audit-export redaction
  plan              Decompose a coarse goal into a workflow DAG (Stream `claude-runtime-planner-trait`). Offline TemplatePlanner by default; `--llm` opts into the Anthropic-backed LlmPlanner
  subtask           Inspect the persistent subtask ledger (Stream `claude-subtask-ledger-store`). One row per node the Planner emits during decomposition; durable across restarts so operators can read a worker's plan after the fact
  import            
  cu                Computer-use trajectory inspection (`claude-cu-action-replay-cli`). `replay <session-id>` renders a recorded CU dispatch as markdown (+ optional GIF)
  replay            Time-travel session replay surface. `export` emits a self-contained tarball of one session (audit chain, attached skills, per-action bundles, manifest with checksum) for offline analysis or support hand-off (`mission-replay-cli-bundle-export`)
  contract          Apply / validate / show a `<role>.contract.toml` delegation contract (Stream `claude-delegation-contract-template`, V1.1 mission claim 2). Single artefact that captures charter + duties + KPI definitions + objectives + KRs + success criteria; `apply` materialises it into an existing worker through the same identity / OKR primitives that `argo worker charter set` and `argo worker okr set` already use
  trust             Cross-tenant trust-graph queries (Stream `mission-cross-tenant-trust-graph`, MISSION_BACKLOG Section 3 Claim 4 #81). `argo trust graph --from <t>` prints outgoing edges; `argo trust composed --from <a> --to <c>` prints the composed score and BFS path
  new               Start a fresh session row in the local store. Without a daemon to actually call the LLM this is just a header insert — useful for prototyping the read-side surface and for capturing externally-streamed messages via the gateway (`POST /sessions/<id>/messages`)
  resume            Re-render a recorded session: prints its header and every message in order. Read-only — for the live "continue this session against an LLM" flow, see the daemon's /sessions/<id>/runs binding (separate PR)
  setup             First-time interactive setup. Mirrors the `curl … | bash && argo setup` Hermes-style two-step install (#646). Idempotent: re-runnable to verify configuration or pick a different provider
  onboard           First-experience operator walkthrough — workspace bootstrap, credentials, first connector, first worker (genesis), first action, done. 7 stages; stages 3-7 may Skip cleanly. The non-developer's entry point — pairs with `argo setup` (the CI-style provider-key probe)
  steer             Append an out-of-band user message to a live session. Used to nudge a running agent without waiting for the next prompt
  trace             Inspect session traces. Without a subcommand, renders the audit-log timeline (`--session`) or one turn's span tree (`--turn`); with `explore <session>`, walks the local OTLP NDJSON sink and renders a markdown or JSON tree
  chat              Run one turn of the agent loop against a recorded session: loads the session's messages, calls the configured provider (Anthropic / OpenAI / OpenAI Responses), and persists the assistant's response. Requires the provider's API-key env var
  tui               Open the interactive TUI (read-only session viewer for now)
  tools             List the in-core tools the agent can call. Mirrors `GET /tools`
  doctor            Diagnose configuration / sandbox / port issues
  health            Run every read-only verification — workspace, provider manifests, skill hashes, memory hashes, audit chain — and exit non-zero on the first failure. Suitable as a single CI hook
  recover           LLM-assisted post-mortem for a failed session (Stream `claude-error-recovery-suggestions`). Walks the audit chain, calls Anthropic with the chain summary, returns 3 ranked fix suggestions. Cached per (session_id, audit_chain_hash) so re-runs are free. Advisory only — no auto-apply
  recover-cache     Inspect or invalidate cached suggestions written by `argo recover` (Stream `codex-recover-cache-cli`). Pure CRUD over the `recovery_cache` table — no LLM calls
  voice             Local voice TUI for approval flow (`claude-voice-local-tui-whisper`). Mic capture is opt-in: `--features whisper-native`
  latency           Show per-executor-kind action latency percentiles (Stream `claude-latency-percentiles-tracking`). Reads the `latency_buckets` table populated by the dispatch wrapper and prints `kind | n | p50 | p95 | p99 | max` as a markdown table (default) or as JSON
  update            Placeholder for an in-place upgrade command. Not yet wired in this build — see `argo self-update` for the working signed-release upgrade flow
  wizard            End-to-end operator setup wizard (Mission `mission-operator-onboarding-wizard`). `argo wizard onboard` replaces the five-command first-time bootstrap (detect → provider key → operator identity → vertical → first worker) with a single guided flow
  self-update       Verified binary self-upgrade (Stream `codex-self-update-cli`, V1.4). Queries GitHub Releases for a newer build, downloads the platform asset + detached Ed25519 signature, verifies against the embedded release pubkey, and atomically replaces the running binary. Best-effort gateway daemon restart
  completions       Print a shell completion script for `argo` to stdout (Stream `mission-cli-shell-completions`). Pipe to the shell's completion-loader path:
  security          Coordinated security operations. Subcommands: - `audit` — vuln-scan wrapper around `cargo audit` + `cargo deny check advisories` (Mission `mission-vuln-management-workflow`). See `docs/skycot/security/vuln-management.md`. - `rotate-keys` — key-rotation coordinator across DB encryption / credentials master / gateway bearer / channel HMAC / per-connector OAuth (Mission `mission-automated-key-rotation`). See the "Key rotation" section of `docs/skycot/RUNBOOK.md`
  billing           Stripe billing recovery — DLQ inspection + event replay
  help              Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

### `argo action`

```text
Action debugging tools that do not mutate the action ledger

Usage: argo action <COMMAND>

Commands:
  replay    Re-run a completed action against a fresh executor into a replay bundle
  history   Show recent actions for one worker, newest first
  timeline  Render an activity histogram for one worker
  recent    Show recent actions across workers
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo action history`

```text
Show recent actions for one worker, newest first

Usage: argo action history [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>  

Options:
      --since <SINCE>  
      --limit <LIMIT>  [default: 50]
      --include-skill  
      --json           
  -h, --help           Print help
```

#### `argo action recent`

```text
Show recent actions across workers

Usage: argo action recent [OPTIONS]

Options:
      --tenant <TENANT>  
      --worker <WORKER>  
      --limit <LIMIT>    [default: 50]
      --json             
  -h, --help             Print help
```

#### `argo action replay`

```text
Re-run a completed action against a fresh executor into a replay bundle

Usage: argo action replay [OPTIONS] <ACTION_ID>

Arguments:
  <ACTION_ID>  `action_id` from `argo worker actions list`

Options:
      --executor <EXECUTOR>      Override the executor kind instead of using the action's manifest default [possible values: human, api, computer_use]
      --dry-run                  Print the resolved replay plan without executing or writing a bundle
      --bundle-dir <BUNDLE_DIR>  Exact replay bundle directory to write. Defaults to a sibling replay timestamp dir
  -h, --help                     Print help
```

#### `argo action timeline`

```text
Render an activity histogram for one worker

Usage: argo action timeline [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>  

Options:
      --since <SINCE>              [default: 24h]
      --bucket-mins <BUCKET_MINS>  [default: 5]
      --json                       
  -h, --help                       Print help
```

### `argo activity`

```text
Usage: argo activity <COMMAND>

Commands:
  tail   Follow worker activity as audit rows arrive
  since  List recent worker activity in a duration window
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo activity since`

```text
List recent worker activity in a duration window

Usage: argo activity since [OPTIONS] <DURATION>

Arguments:
  <DURATION>  Lookback duration (examples: 30s, 5m, 1h, 1d)

Options:
      --limit <LIMIT>    Maximum rows to print [default: 100]
      --tenant <TENANT>  Filter to one tenant id
      --worker <WORKER>  Filter to one worker id
      --json             Emit newline-delimited JSON activity envelopes
  -h, --help             Print help
```

#### `argo activity tail`

```text
Follow worker activity as audit rows arrive

Usage: argo activity tail [OPTIONS]

Options:
      --tenant <TENANT>  Filter to one tenant id
      --worker <WORKER>  Filter to one worker id
      --since <SINCE>    Backfill rows from a recent duration before tailing (examples: 30s, 5m, 1h, 1d)
      --json             Emit newline-delimited JSON activity envelopes
  -h, --help             Print help
```

### `argo agents`

```text
Usage: argo agents [COMMAND]

Commands:
  list    List `[agents.*]` blocks from `argo.toml`
  create  Add a new `[agents.<name>]` block to `argo.toml`. Use the flags to populate keys; defaults are sensible enough that the agent shows up in `argo agents list` immediately
  delete  Remove an `[agents.<name>]` block from `argo.toml`
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo agents create`

```text
Add a new `[agents.<name>]` block to `argo.toml`. Use the flags to populate keys; defaults are sensible enough that the agent shows up in `argo agents list` immediately

Usage: argo agents create [OPTIONS] <NAME>

Arguments:
  <NAME>  

Options:
      --model <MODEL>                        `default_model` field. Defaults to "claude-3-5-sonnet"
      --toolset <TOOLSET>                    `default_toolset` field. Defaults to "default"
      --memory-namespace <MEMORY_NAMESPACE>  `memory_namespace` field. Defaults to the agent name
  -h, --help                                 Print help

Examples:
  argo agents create research-bot
  argo agents create scout --model claude-sonnet-4-6 --toolset web
```

#### `argo agents delete`

```text
Remove an `[agents.<name>]` block from `argo.toml`

Usage: argo agents delete <NAME>

Arguments:
  <NAME>  

Options:
  -h, --help  Print help

Examples:
  argo agents delete research-bot
```

#### `argo agents list`

```text
List `[agents.*]` blocks from `argo.toml`

Usage: argo agents list [OPTIONS]

Options:
      --json             Output as a JSON array for scripting. Equivalent to `--format json`
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
  -h, --help             Print help

Examples:
  argo agents list
  argo agents list --format json
  argo agents list --format markdown
```

### `argo approval`

```text
Approval audit surfaces

Usage: argo approval <COMMAND>

Commands:
  export   Export approval audit rows to CSV, JSON, or JSONL
  history  Search the approval history with --since/--until/--decision/ --worker/--action-kind/--limit/--format flags. Read-only over the worker-action ledger; mirrors the column set of `argo approval export` minus the payload excerpt
  policy   Approval-policy starter templates (Stream `claude-approval-policy-templates`, V1.4): list bundled + repo-local templates, print one resolved against its `extends` chain, or copy a bundled body into `.skycot/approval-policies/<name>.toml` for editing
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo approval export`

```text
Export approval audit rows to CSV, JSON, or JSONL

Usage: argo approval export [OPTIONS]

Options:
      --since <SINCE>              Include approvals requested at or after this time (`30d`, `12h`, or RFC3339)
      --until <UNTIL>              Include approvals requested at or before this time (`30d`, `12h`, or RFC3339)
      --decision <DECISION>        Filter by decision; may be repeated. Values: approve, deny, pending, timeout
      --worker <WORKER>            Strict equality filter on worker id/name
      --action-kind <ACTION_KIND>  Strict equality filter on action kind
      --limit <LIMIT>              Maximum rows to export; clamped to 1000 [default: 50]
      --format <FORMAT>            Output format: csv, json, or jsonl [default: csv]
      --out <OUT>                  Output path, or `-` for stdout [default: -]
      --force                      Allow overwriting an existing output file
  -h, --help                       Print help
```

#### `argo approval history`

```text
Search the approval history with --since/--until/--decision/ --worker/--action-kind/--limit/--format flags. Read-only over the worker-action ledger; mirrors the column set of `argo approval export` minus the payload excerpt

Usage: argo approval history [OPTIONS]

Options:
      --since <SINCE>              Include approvals requested at or after this time (`7d`, `24h`, `30m`, or RFC3339)
      --until <UNTIL>              Include approvals requested at or before this time (`7d`, `24h`, `30m`, or RFC3339)
      --decision <DECISION>        Filter by decision; may be repeated. Values: approve, deny, pending, timeout
      --worker <WORKER>            Strict equality filter on emitting worker
      --action-kind <ACTION_KIND>  Strict equality filter on action kind
      --limit <LIMIT>              Maximum rows to return; clamped to 1000 [default: 50]
      --format <FORMAT>            Output format: table, json (NDJSON), or markdown [default: table]
  -h, --help                       Print help
```

#### `argo approval policy`

```text
Approval-policy starter templates (Stream `claude-approval-policy-templates`, V1.4): list bundled + repo-local templates, print one resolved against its `extends` chain, or copy a bundled body into `.skycot/approval-policies/<name>.toml` for editing

Usage: argo approval policy <COMMAND>

Commands:
  list  List bundled + repo-local approval policy templates
  show  Print one template resolved against its `extends` chain
  init  Copy a bundled template into the local override directory
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo approval policy init`

```text
Copy a bundled template into the local override directory

Usage: argo approval policy init [OPTIONS] <NAME>

Arguments:
  <NAME>  Bundled template name (must match `policy list`)

Options:
      --force  Overwrite an existing local file
      --json   Emit `{name, path, bytes}` as JSON instead of a status line
  -h, --help   Print help
```

##### `argo approval policy list`

```text
List bundled + repo-local approval policy templates

Usage: argo approval policy list [OPTIONS]

Options:
      --json  Emit `{bundled, local, local_dir}` as JSON
  -h, --help  Print help
```

##### `argo approval policy show`

```text
Print one template resolved against its `extends` chain

Usage: argo approval policy show [OPTIONS] <NAME>

Arguments:
  <NAME>  Template name (bundled, or stem of a local `.skycot/approval-policies/<name>.toml` file)

Options:
      --json  Emit `{name, policy, toml}` as JSON
  -h, --help  Print help
```

### `argo audit`

```text
Usage: argo audit <COMMAND>

Commands:
  show            Show the kind histogram + (optionally) the last `--tail N` rows. Pass `--session <id>` to print every audit row for one session in `seq` order instead of the global histogram view
  query           Query audit rows with H-stack filters. Filters compose with each other and with `--since`, `--until`, and `--limit`
  verify          Verify the hash chain. Pass `--session <id>` to check one session only (fast on huge logs)
  verify-chain    Verify the full audit chain and print an operator summary
  export          Export audit rows for compliance review
  report          Render a human-readable compliance summary report
  breaks          Print just the chain-break rows (`{seq, kind, reason}`). Mirrors `GET /audit/breaks` (#406). One row per line in the default view, or a single JSON array with `--json`
  count           Print just the count of audit rows after filters. Mirrors `GET /audit/count` (#458)
  by-kind         Histogram of audit rows grouped by `kind`. Mirrors the gateway's `GET /audit/by-kind` (#570)
  by-vertical     Per-vertical audit-row histogram (#688). Joins audit_record.session_id -> session.vertical_id; rows from unbound sessions / no-session-id rows fall in `(none)`
  by-day          Daily audit-row rollup. Mirrors the gateway's `GET /audit/by-day` (#571)
  by-session      Top-N sessions by audit row count. Mirrors the gateway's `GET /audit/by-session` (#608)
  by-tool         Histogram of `tool_call`+`tool_result` audit rows grouped by tool name. Mirrors the gateway's `GET /audit/by-tool` (#613)
  tool-errors     Histogram of `tool_result` audit rows where `is_error=true`, grouped by tool. Mirrors the gateway's `GET /audit/tool-errors` (#617). Operational signal for spotting broken tool integrations
  by-day-by-kind  2D rollup: audit rows bucketed by `(day, kind)`. Mirrors the gateway's `GET /audit/by-day-by-kind` (#638). Useful for "did tool_call volume spike Tuesday?" without joining by-day + by-kind client-side
  archive         Archive audit-chain rows older than `--retention-days` into a signed `.tar.gz` bundle under `<archive-dir>` (#228, Tier S)
  list-archives   List every bundle recorded in `<archive-dir>/manifest.jsonl`
  verify-archive  Recompute SHA-256 + HMAC of `<bundle_id>` and assert match
  replay          Replay one session's audit chain in seq order for incident investigation (Stream `claude-audit-replay`). `pretty` (default) prints one-line headlines per row; `raw` emits NDJSON for piping into `jq`
  help            Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo audit archive`

```text
Archive audit-chain rows older than `--retention-days` into a signed `.tar.gz` bundle under `<archive-dir>` (#228, Tier S)

Usage: argo audit archive [OPTIONS]

Options:
      --retention-days <RETENTION_DAYS>
          Retention window in days; rows older are archive candidates [default: 90]
      --prune
          Delete archived rows from the live table after the bundle is durable. Off by default — opt in only after verifying
      --hard-keep-days <HARD_KEEP_DAYS>
          Floor (days) below which rows are NEVER pruned, regardless of `--retention-days`. Guards against operator typos
      --archive-dir <ARCHIVE_DIR>
          Where to write the bundle + manifest.jsonl. Default `<workspace>/audit-archive/`
      --json
          Emit a JSON summary instead of human text
  -h, --help
          Print help
```

#### `argo audit breaks`

```text
Print just the chain-break rows (`{seq, kind, reason}`). Mirrors `GET /audit/breaks` (#406). One row per line in the default view, or a single JSON array with `--json`

Usage: argo audit breaks [OPTIONS]

Options:
      --json  Output as a single JSON array
  -h, --help  Print help
```

#### `argo audit by-day`

```text
Daily audit-row rollup. Mirrors the gateway's `GET /audit/by-day` (#571)

Usage: argo audit by-day [OPTIONS]

Options:
      --days <DAYS>  Lookback window in days (default 30, cap 365) [default: 30]
      --json         Output as a single JSON object (`{by_day}`)
  -h, --help         Print help
```

#### `argo audit by-day-by-kind`

```text
2D rollup: audit rows bucketed by `(day, kind)`. Mirrors the gateway's `GET /audit/by-day-by-kind` (#638). Useful for "did tool_call volume spike Tuesday?" without joining by-day + by-kind client-side

Usage: argo audit by-day-by-kind [OPTIONS]

Options:
      --days <DAYS>  Lookback window in days (default 30, cap 365) [default: 30]
      --json         Output as a single JSON object (`{by_day_by_kind}`)
  -h, --help         Print help
```

#### `argo audit by-kind`

```text
Histogram of audit rows grouped by `kind`. Mirrors the gateway's `GET /audit/by-kind` (#570)

Usage: argo audit by-kind [OPTIONS]

Options:
      --since-minutes <SINCE_MINUTES>  Filter to rows created within this many recent minutes
      --json                           Output as a single JSON object (`{by_kind}`)
  -h, --help                           Print help
```

#### `argo audit by-session`

```text
Top-N sessions by audit row count. Mirrors the gateway's `GET /audit/by-session` (#608)

Usage: argo audit by-session [OPTIONS]

Options:
      --limit <LIMIT>                  Top-N session limit (default 50, cap 500) [default: 50]
      --since-minutes <SINCE_MINUTES>  Filter to rows created within this many recent minutes
      --json                           Output as a single JSON object (`{by_session}`)
  -h, --help                           Print help
```

#### `argo audit by-tool`

```text
Histogram of `tool_call`+`tool_result` audit rows grouped by tool name. Mirrors the gateway's `GET /audit/by-tool` (#613)

Usage: argo audit by-tool [OPTIONS]

Options:
      --since-minutes <SINCE_MINUTES>  Filter to rows created within this many recent minutes
      --json                           Output as a single JSON object (`{by_tool}`)
  -h, --help                           Print help
```

#### `argo audit by-vertical`

```text
Per-vertical audit-row histogram (#688). Joins audit_record.session_id -> session.vertical_id; rows from unbound sessions / no-session-id rows fall in `(none)`

Usage: argo audit by-vertical [OPTIONS]

Options:
      --since-minutes <SINCE_MINUTES>  
      --json                           
  -h, --help                           Print help
```

#### `argo audit count`

```text
Print just the count of audit rows after filters. Mirrors `GET /audit/count` (#458)

Usage: argo audit count [OPTIONS]

Options:
      --kind <KIND>                    Filter by audit kind (e.g. tool_call, turn_start)
      --since-minutes <SINCE_MINUTES>  Filter to rows created within this many recent minutes
      --session <SESSION>              Filter to rows on one session (uuid)
      --json                           Output as a single JSON object
  -h, --help                           Print help
```

#### `argo audit export`

```text
Export audit rows for compliance review

Usage: argo audit export [OPTIONS]

Options:
      --format <FORMAT>                Output format: `ndjson`, `jsonl`, `json`, `csv`, or `tarball`. `jsonl` is a SIEM-friendly synonym for `ndjson`; `json` emits a single top-level array [default: ndjson]
      --bundle-out <BUNDLE_OUT>        Write output to a path instead of stdout
      --since <SINCE>                  Lower created-at bound: YYYY-MM-DD, RFC3339, or a compact duration like `7d`, `24h`, `30m`, `90s` (relative to now)
      --until <UNTIL>                  Upper created-at bound: YYYY-MM-DD or RFC3339
      --worker-id <WORKER_ID>          Filter by payload worker id
      --tenant <TENANT>                Filter by tenant id
      --session <SESSION>              Legacy session filter retained for incident drill-downs
      --since-minutes <SINCE_MINUTES>  Filter to rows created within this many recent minutes. Combined with `--session` if both are set
      --gdpr-redact-emails             Replace emails in payloads with deterministic placeholders
      --json                           Emit a JSON summary envelope instead of records
      --kind <KINDS>                   Comma-separated list of `AuditKind` labels to include (`mission-audit-csv-export`). When set, only rows whose `kind` matches one of the listed labels are exported
  -h, --help                           Print help
```

#### `argo audit list-archives`

```text
List every bundle recorded in `<archive-dir>/manifest.jsonl`

Usage: argo audit list-archives [OPTIONS]

Options:
      --archive-dir <ARCHIVE_DIR>  
      --json                       
  -h, --help                       Print help
```

#### `argo audit query`

```text
Query audit rows with H-stack filters. Filters compose with each other and with `--since`, `--until`, and `--limit`

Usage: argo audit query [OPTIONS]

Options:
      --decision <DECISION>  Filter auto-approval rows by decision
      --worker <WORKER>      Filter rows by payload `worker_id`
      --session <SESSION>    Filter rows by session UUID
      --escalation           Include escalation/handoff lifecycle audit rows
      --since <SINCE>        Lower created-at bound, RFC3339/ISO-8601 timestamp
      --until <UNTIL>        Upper created-at bound, RFC3339/ISO-8601 timestamp
      --limit <LIMIT>        Maximum rows to return [default: 100]
      --json                 Output as a single JSON object with query envelope
  -h, --help                 Print help
```

#### `argo audit replay`

```text
Replay one session's audit chain in seq order for incident investigation (Stream `claude-audit-replay`). `pretty` (default) prints one-line headlines per row; `raw` emits NDJSON for piping into `jq`

Usage: argo audit replay [OPTIONS] <SESSION_ID>

Arguments:
  <SESSION_ID>  Session id (UUID) to replay

Options:
      --from-seq <FROM_SEQ>  Lower bound on `seq` (inclusive)
      --until <UNTIL>        Upper bound (inclusive) on `created_at` as RFC-3339
      --format <FORMAT>      Output format: `pretty` (default) or `raw` (NDJSON) [default: pretty]
  -h, --help                 Print help
```

#### `argo audit report`

```text
Render a human-readable compliance summary report

Usage: argo audit report [OPTIONS]

Options:
      --format <FORMAT>        Output format: markdown or html [default: markdown]
      --output <OUTPUT>        Write report to a path instead of stdout
      --since <SINCE>          Lower created-at bound: YYYY-MM-DD or RFC3339
      --until <UNTIL>          Upper created-at bound: YYYY-MM-DD or RFC3339
      --worker-id <WORKER_ID>  Filter by payload worker id
      --tenant <TENANT>        Filter by tenant id
      --json                   Emit a JSON summary envelope
  -h, --help                   Print help
```

#### `argo audit show`

```text
Show the kind histogram + (optionally) the last `--tail N` rows. Pass `--session <id>` to print every audit row for one session in `seq` order instead of the global histogram view

Usage: argo audit show [OPTIONS]

Options:
      --tail <TAIL>                    
      --json                           Output as a single JSON object (kind histogram + tail). Equivalent to `--format json`
      --format <FORMAT>                Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
      --session <SESSION>              Session id to scope to. With this set, the output is the session's full audit chain (seq order, oldest first)
      --kinds <KINDS>                  Comma-separated list of audit kinds to include (case- sensitive label match, e.g. `ToolCall,ToolResult`). When set, the tail / per-session view drops every other kind
      --since-minutes <SINCE_MINUTES>  Filter to rows created within this many recent minutes. Applies to both the tail view and `--session`
      --watch                          `tail -f` style: poll for new rows every 250ms and print them as they arrive. Combine with `--session <id>` to watch one session live; Ctrl-C to exit
  -h, --help                           Print help
```

#### `argo audit tool-errors`

```text
Histogram of `tool_result` audit rows where `is_error=true`, grouped by tool. Mirrors the gateway's `GET /audit/tool-errors` (#617). Operational signal for spotting broken tool integrations

Usage: argo audit tool-errors [OPTIONS]

Options:
      --since-minutes <SINCE_MINUTES>  Filter to rows created within this many recent minutes
      --json                           Output as a single JSON object (`{tool_errors}`)
  -h, --help                           Print help
```

#### `argo audit verify`

```text
Verify the hash chain. Pass `--session <id>` to check one session only (fast on huge logs)

Usage: argo audit verify [OPTIONS]

Options:
      --from <FROM>        
      --to <TO>            
      --session <SESSION>  
      --json               Output as a single JSON object (ok / rows / breaks). Equivalent to `--format json`
      --format <FORMAT>    Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
  -h, --help               Print help
```

#### `argo audit verify-archive`

```text
Recompute SHA-256 + HMAC of `<bundle_id>` and assert match

Usage: argo audit verify-archive [OPTIONS] <BUNDLE_ID>

Arguments:
  <BUNDLE_ID>  Bundle id (UUIDv7) as printed by `list-archives`

Options:
      --archive-dir <ARCHIVE_DIR>  
      --json                       
  -h, --help                       Print help
```

#### `argo audit verify-chain`

```text
Verify the full audit chain and print an operator summary

Usage: argo audit verify-chain [OPTIONS]

Options:
      --json  Emit a single JSON object instead of the text report
  -h, --help  Print help
```

### `argo backup`

```text
Workspace backup create/list/verify

Usage: argo backup <COMMAND>

Commands:
  create  Create a tar.gz backup of the workspace .argo directory
  list    List backups under .argo/backups
  verify  Verify a backup tarball's manifest and file hashes
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo backup create`

```text
Create a tar.gz backup of the workspace .argo directory

Usage: argo backup create [OPTIONS]

Options:
      --output <OUTPUT>      Write the backup to this path instead of .argo/backups
      --include-credentials  Include .argo/credentials.toml in the archive
      --json                 Emit a JSON envelope
  -h, --help                 Print help

Examples:
  argo backup create
  argo backup create --output /backups/$(date +%F).tar.gz --json
  argo backup create --include-credentials
```

#### `argo backup list`

```text
List backups under .argo/backups

Usage: argo backup list [OPTIONS]

Options:
      --json  Emit a JSON envelope
  -h, --help  Print help

Examples:
  argo backup list
  argo backup list --json
```

#### `argo backup verify`

```text
Verify a backup tarball's manifest and file hashes

Usage: argo backup verify [OPTIONS] <PATH>

Arguments:
  <PATH>  

Options:
      --json  Emit a JSON envelope
  -h, --help  Print help

Examples:
  argo backup verify .argo/backups/2026-05-13.tar.gz
  argo backup verify ./snap.tar.gz --json
```

### `argo billing`

```text
Stripe billing recovery — DLQ inspection + event replay

Usage: argo billing <COMMAND>

Commands:
  replay  Replay one or more dead-lettered Stripe events. Reads from `billing_event_dead_letter`; no Stripe API call
  dlq     Inspect or resolve dead-lettered webhook events
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo billing dlq`

```text
Inspect or resolve dead-lettered webhook events

Usage: argo billing dlq <COMMAND>

Commands:
  list     List dead-letter entries with reasons + retry counts
  resolve  Mark a DLQ entry resolved without running replay. Use when the operator has fixed the underlying state out-of-band and wants the entry off the queue
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo billing dlq list`

```text
List dead-letter entries with reasons + retry counts

Usage: argo billing dlq list [OPTIONS]

Options:
      --since <SINCE>     Only show entries failed within this duration (e.g. `24h`). Default: no lower bound
      --include-resolved  Include rows that have already been resolved
      --limit <LIMIT>     Row cap. Default 100 [default: 100]
      --json              Emit JSON instead of human text
  -h, --help              Print help
```

##### `argo billing dlq resolve`

```text
Mark a DLQ entry resolved without running replay. Use when the operator has fixed the underlying state out-of-band and wants the entry off the queue

Usage: argo billing dlq resolve [OPTIONS] --event-id <EVENT_ID>

Options:
      --event-id <EVENT_ID>  Stripe `evt_…` id
      --note <NOTE>          Free-text note recorded with the resolution
      --json                 Emit JSON instead of human text
  -h, --help                 Print help
```

#### `argo billing replay`

```text
Replay one or more dead-lettered Stripe events. Reads from `billing_event_dead_letter`; no Stripe API call

Usage: argo billing replay [OPTIONS]

Options:
      --event-id <EVENT_ID>  Replay a single event by its Stripe `evt_…` id
      --since <SINCE>        Replay every unresolved DLQ entry whose `failed_at` is within this duration of now. Accepts `30m`, `24h`, `7d`
      --limit <LIMIT>        Per-batch row cap (replay-since only). Default 100 [default: 100]
      --json                 Emit a JSON envelope to stdout instead of human text
  -h, --help                 Print help
```

### `argo budget`

```text
Usage: argo budget <COMMAND>

Commands:
  show          Show daily/monthly rollups, policy headroom, and last five charges for one worker
  recent        List recent worker-budget charges
  anomalies     List spend-anomaly events (Welford z-score on per-worker USD-per-hour history; threshold env-overridable via `ARGO_SPEND_ANOMALY_Z`, default 4.0). Stream `claude-spend-anomaly-alerts`
  anomaly-tick  Run one anomaly-detection tick: walk every active worker, fold the previous-hour USD-per-hour into the (worker, hour-of-day) baseline, and write a `worker_spend_anomaly` row when |z| exceeds the configured threshold. Designed for cron / scheduler invocation
  by-skill      Rank spend by skill or show one skill's daily trend
  watch         Watch worker budget spend and fire push alerts on 80% / 100% daily-cap threshold crossings
  alerts        Inspect or test budget alert deliveries
  help          Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo budget alerts`

```text
Inspect or test budget alert deliveries

Usage: argo budget alerts <COMMAND>

Commands:
  list  List recent budget alerts
  test  Fire a synthetic budget alert immediately, bypassing dedup
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo budget alerts list`

```text
List recent budget alerts

Usage: argo budget alerts list [OPTIONS]

Options:
      --worker <WORKER>
          Restrict to one worker id

      --since <SINCE>
          Lookback window, e.g. 30m, 1h, 7d, or RFC3339

      --limit <LIMIT>
          Cap the row count
          
          [default: 50]

      --json
          Emit one JSON object per alert

      --format <FORMAT>
          Possible values:
          - human: Human-readable text
          - json:  JSON output matching the legacy `--json` switch
          - csv:   RFC 4180 CSV with a header row
          
          [default: human]

  -h, --help
          Print help (see a summary with '-h')
```

##### `argo budget alerts test`

```text
Fire a synthetic budget alert immediately, bypassing dedup

Usage: argo budget alerts test [OPTIONS] --worker <WORKER> --threshold <THRESHOLD>

Options:
      --worker <WORKER>        Worker id to include in the alert
      --threshold <THRESHOLD>  Threshold to test
      --json                   Emit a JSON envelope
  -h, --help                   Print help
```

#### `argo budget anomalies`

```text
List spend-anomaly events (Welford z-score on per-worker USD-per-hour history; threshold env-overridable via `ARGO_SPEND_ANOMALY_Z`, default 4.0). Stream `claude-spend-anomaly-alerts`

Usage: argo budget anomalies [OPTIONS]

Options:
      --worker <WORKER>
          Restrict to one worker

      --since <SINCE>
          Lookback window — accepts e.g. `60s`, `30m`, `24h`, `7d`

      --limit <LIMIT>
          Cap the row count
          
          [default: 50]

      --json
          Emit a JSON envelope instead of the human-readable table

      --format <FORMAT>
          Possible values:
          - human: Human-readable text
          - json:  JSON output matching the legacy `--json` switch
          - csv:   RFC 4180 CSV with a header row
          
          [default: human]

  -h, --help
          Print help (see a summary with '-h')
```

#### `argo budget anomaly-tick`

```text
Run one anomaly-detection tick: walk every active worker, fold the previous-hour USD-per-hour into the (worker, hour-of-day) baseline, and write a `worker_spend_anomaly` row when |z| exceeds the configured threshold. Designed for cron / scheduler invocation

Usage: argo budget anomaly-tick [OPTIONS]

Options:
      --json  Emit the per-worker decisions as JSON instead of the human summary
  -h, --help  Print help
```

#### `argo budget by-skill`

```text
Rank spend by skill or show one skill's daily trend

Usage: argo budget by-skill [OPTIONS] [SKILL_NAME]

Arguments:
  [SKILL_NAME]  

Options:
      --history              
      --since <SINCE>        
      --limit <LIMIT>        [default: 10]
      --vertical <VERTICAL>  
      --json                 
  -h, --help                 Print help
```

#### `argo budget recent`

```text
List recent worker-budget charges

Usage: argo budget recent [OPTIONS]

Options:
      --limit <LIMIT>
          [default: 50]

      --worker <WORKER>
          

      --since <SINCE>
          

      --json
          

      --format <FORMAT>
          Possible values:
          - human: Human-readable text
          - json:  JSON output matching the legacy `--json` switch
          - csv:   RFC 4180 CSV with a header row
          
          [default: human]

  -h, --help
          Print help (see a summary with '-h')
```

#### `argo budget show`

```text
Show daily/monthly rollups, policy headroom, and last five charges for one worker

Usage: argo budget show [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>
          

Options:
      --json
          

      --format <FORMAT>
          Possible values:
          - human: Human-readable text
          - json:  JSON output matching the legacy `--json` switch
          - csv:   RFC 4180 CSV with a header row
          
          [default: human]

  -h, --help
          Print help (see a summary with '-h')
```

#### `argo budget watch`

```text
Watch worker budget spend and fire push alerts on 80% / 100% daily-cap threshold crossings

Usage: argo budget watch [OPTIONS]

Options:
      --worker <WORKER>  Restrict the watch pass to one worker id
      --once             Run a single pass instead of looping forever
      --json             Emit JSON tick summaries
  -h, --help             Print help
```

### `argo chat`

```text
Run one turn of the agent loop against a recorded session: loads the session's messages, calls the configured provider (Anthropic / OpenAI / OpenAI Responses), and persists the assistant's response. Requires the provider's API-key env var

Usage: argo chat [OPTIONS] <ID>

Arguments:
  <ID>  Session id (from `argo new` / `argo session list`)

Options:
      --user <USER>
          Optional fresh user message to append before the call. Equivalent to a `POST /sessions/<id>/messages` round-trip
      --stream
          Stream the response token-by-token to stdout. Final persisted message is the same — but you see it live
  -i, --interactive
          Drop into a REPL: read user input, run a turn, print the response, repeat. Ctrl-D / EOF exits cleanly
      --max-tool-rounds <MAX_TOOL_ROUNDS>
          Override `MAX_TOOL_ROUNDS` (default 8). Useful when debugging — set to 1 to fail fast on the first tool call, set higher for legitimately long planning chains
  -h, --help
          Print help

Examples:
  export ANTHROPIC_API_KEY=sk-ant-...
  argo chat 01J9XYZ... --user "summarise progress so far"
  argo chat 01J9XYZ... --interactive --stream
```

### `argo classify`

```text
Tag records (memory rows, audit payloads, action params) with data classes (Stream `claude-data-classification`): `pii` / `phi` / `financial` / `secret` / `public` / `untagged`. Drives per-class retention windows and audit-export redaction

Usage: argo classify <COMMAND>

Commands:
  set    Tag a record with a data class. The target is given as `<kind>:<id>` (e.g. `memory_entry:01964…`, `audit_record:42`). Class must be one of `pii`, `phi`, `financial`, `secret`, `public`, `untagged`
  list   List recorded tags. All filters are optional
  rules  Print the configured per-class handling rules (`retention_days`, `redact_in_export`). Reads `<workspace>/.argo/classification.toml` if present and merges over shipped defaults
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo classify list`

```text
List recorded tags. All filters are optional

Usage: argo classify list [OPTIONS]

Options:
      --class <CLASS>    Restrict to one class (`pii` / `phi` / ...)
      --kind <KIND>      Restrict to one record kind (e.g. `memory_entry`)
      --worker <WORKER>  Restrict to one author / `tagged_by` value
      --expired          Show only tags whose retention window has expired (older than the per-class `retention_days`). Useful before running a prune
      --limit <LIMIT>    Cap the number of rows printed. Default 100 [default: 100]
      --json             Output as a single JSON array
  -h, --help             Print help
```

#### `argo classify rules`

```text
Print the configured per-class handling rules (`retention_days`, `redact_in_export`). Reads `<workspace>/.argo/classification.toml` if present and merges over shipped defaults

Usage: argo classify rules [OPTIONS]

Options:
      --json  Output as a single JSON object keyed by class
  -h, --help  Print help
```

#### `argo classify set`

```text
Tag a record with a data class. The target is given as `<kind>:<id>` (e.g. `memory_entry:01964…`, `audit_record:42`). Class must be one of `pii`, `phi`, `financial`, `secret`, `public`, `untagged`

Usage: argo classify set [OPTIONS] <TARGET> <CLASS>

Arguments:
  <TARGET>  `<record_kind>:<record_id>` — e.g. `memory_entry:abc`
  <CLASS>   One of `pii`, `phi`, `financial`, `secret`, `public`, `untagged`

Options:
      --by <BY>  Optional actor that owns this tag (worker id, operator handle, automation rule). Stored in `tagged_by`
      --json     Output as a single JSON object
  -h, --help     Print help
```

### `argo completions`

```text
Print a shell completion script for `argo` to stdout (Stream `mission-cli-shell-completions`). Pipe to the shell's completion-loader path:

```text argo completions bash > /usr/local/etc/bash_completion.d/argo argo completions zsh  > "${fpath[1]}/_argo" argo completions fish > ~/.config/fish/completions/argo.fish ```

Usage: argo completions <SHELL>

Arguments:
  <SHELL>
          Target shell — `bash`, `zsh`, `fish`, `powershell`, or `elvish`. Validated by `clap_complete::Shell`
          
          [possible values: bash, elvish, fish, powershell, zsh]

Options:
  -h, --help
          Print help (see a summary with '-h')
```

### `argo compliance`

```text
Compliance surfaces (Phase C). Read-only over `audit_record` and tenant-scoped tables. - `soc2` — SOC2 Common Criteria 6.x audit-trail report (`claude-compliance-soc2-audit-trail`). - `export` — GDPR Article 15 right-to-access JSON envelope for one tenant; the export is itself audit-logged (`claude-compliance-gdpr-data-export`)

Usage: argo compliance <COMMAND>

Commands:
  soc2    SOC2 Common Criteria 6.x audit-trail report. Groups events in `[--from, --to)` by control category (Access, ChangeMgmt, DataMgmt, Security, Other)
  export  Emit tenant-scoped compliance data with filters and an integrity stamp
  delete  Cascade-delete every tenant-scoped row for GDPR right-to- erasure. Audit rows are tombstoned in place (PII fields replaced with `<erased>`, hash chain re-stitched). The whole operation is a single transaction; `--dry-run` rolls it back
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo compliance delete`

```text
Cascade-delete every tenant-scoped row for GDPR right-to- erasure. Audit rows are tombstoned in place (PII fields replaced with `<erased>`, hash chain re-stitched). The whole operation is a single transaction; `--dry-run` rolls it back

Usage: argo compliance delete [OPTIONS] --tenant <TENANT> --confirm <CONFIRM>

Options:
      --tenant <TENANT>    Tenant id (UUID) whose data should be deleted
      --confirm <CONFIRM>  Confirmation id — must exactly match `--tenant`. Mismatch exits 2 without touching any row
      --dry-run            Report per-table row counts that *would* be deleted, then rollback. No data is modified
      --json               Emit the summary as a single JSON object instead of text
  -h, --help               Print help
```

#### `argo compliance export`

```text
Emit tenant-scoped compliance data with filters and an integrity stamp

Usage: argo compliance export [OPTIONS] --tenant <TENANT>

Options:
      --tenant <TENANT>          Tenant id (UUID) whose data should be exported
      --format <FORMAT>          Output shape: `json`, `csv`, or `pdf`. `gdpr` remains a backwards-compatible alias for `json` [default: json]
      --start-date <START_DATE>  Include rows created on or after this RFC3339 date/time
      --end-date <END_DATE>      Include rows created on or before this RFC3339 date/time
      --session-id <SESSION_ID>  Restrict rows to this session id
      --out <OUT>                Write the JSON envelope to this path. Defaults to stdout
  -h, --help                     Print help
```

#### `argo compliance soc2`

```text
SOC2 Common Criteria 6.x audit-trail report. Groups events in `[--from, --to)` by control category (Access, ChangeMgmt, DataMgmt, Security, Other)

Usage: argo compliance soc2 [OPTIONS] --from <FROM> --to <TO>

Options:
      --from <FROM>      Window start, RFC3339. Inclusive
      --to <TO>          Window end, RFC3339. Exclusive
      --actor <ACTOR>    Optional actor id filter
      --action <ACTION>  Optional action / kind filter
      --tenant <TENANT>  Optional tenant id filter
      --format <FORMAT>  Output format: `markdown` (default), `json`, or `csv` [default: markdown]
  -h, --help             Print help
```

### `argo config`

```text
Inspect the effective merged Argo configuration

Usage: argo config <COMMAND>

Commands:
  show  Show the effective merged configuration
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo config show`

```text
Show the effective merged configuration

Usage: argo config show [OPTIONS]

Options:
      --key <KEY>     Dotted key to print, for example `gateway.host`
      --json          Emit JSON instead of TOML/text
      --source        Annotate TOML lines with their source layer
      --show-secrets  Reveal secret-looking fields. Also requires ARGO_SHOW_SECRETS=1
  -h, --help          Print help
```

### `argo connector`

```text
Manage OAuth and token credentials for MCP connectors

Usage: argo connector <COMMAND>

Commands:
  list         List known connector kinds and whether their manifests are installed
  show         Show one connector's manifest, method list, and required credentials
  authorize    Authorize one connector and store credentials in this workspace
  list-status  Show authorization status for every known connector
  revoke       Delete stored credentials for one connector
  test         Run a no-op API call using stored credentials
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo connector authorize`

```text
Authorize one connector and store credentials in this workspace

Usage: argo connector authorize [OPTIONS] <KIND>

Arguments:
  <KIND>  

Options:
      --token <TOKEN>                  Token for PAT/bot-token connectors. Omit to prompt without echo
      --client-id <CLIENT_ID>          OAuth client id override; otherwise read from env/credentials.toml
      --client-secret <CLIENT_SECRET>  OAuth client secret override; otherwise read from env/credentials.toml
      --scope <SCOPES>                 Add/override OAuth scopes. Repeatable
      --redirect-port <REDIRECT_PORT>  Local callback port for OAuth redirects. Use 0 for any free port [default: 8975]
      --no-browser                     Do not launch the browser; print the authorization URL instead
      --json                           Emit a JSON envelope
  -h, --help                           Print help
```

#### `argo connector list`

```text
List known connector kinds and whether their manifests are installed

Usage: argo connector list [OPTIONS]

Options:
      --installed-only  Only show connectors with manifests/connectors/<kind>.toml present
      --json            Emit a JSON envelope
  -h, --help            Print help
```

#### `argo connector list-status`

```text
Show authorization status for every known connector

Usage: argo connector list-status [OPTIONS]

Options:
      --json  
  -h, --help  Print help
```

#### `argo connector revoke`

```text
Delete stored credentials for one connector

Usage: argo connector revoke [OPTIONS] <KIND>

Arguments:
  <KIND>  

Options:
      --json  
  -h, --help  Print help
```

#### `argo connector show`

```text
Show one connector's manifest, method list, and required credentials

Usage: argo connector show [OPTIONS] <KIND>

Arguments:
  <KIND>  

Options:
      --json  Emit a JSON envelope
  -h, --help  Print help
```

#### `argo connector test`

```text
Run a no-op API call using stored credentials

Usage: argo connector test [OPTIONS] <KIND>

Arguments:
  <KIND>  

Options:
      --json  
  -h, --help  Print help
```

### `argo contract`

```text
Apply / validate / show a `<role>.contract.toml` delegation contract (Stream `claude-delegation-contract-template`, V1.1 mission claim 2). Single artefact that captures charter + duties + KPI definitions + objectives + KRs + success criteria; `apply` materialises it into an existing worker through the same identity / OKR primitives that `argo worker charter set` and `argo worker okr set` already use

Usage: argo contract <COMMAND>

Commands:
  apply     Materialise a `<role>.contract.toml` file into an existing worker (charter + KPI definitions + objectives + KRs). Use `--dry-run` to preview the planned mutations without writing
  validate  Schema-check only — parses the file, runs semantic validation, and reports counts. Never touches the store
  show      Render a worker's current state as TOML so it round-trips with `apply`. Reads charter + kpis blob + objectives/KRs
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo contract apply`

```text
Materialise a `<role>.contract.toml` file into an existing worker (charter + KPI definitions + objectives + KRs). Use `--dry-run` to preview the planned mutations without writing

Usage: argo contract apply [OPTIONS] <FILE>

Arguments:
  <FILE>  Path to the `<role>.contract.toml` file

Options:
      --tenant <TENANT>  Tenant id whose worker namespace should receive the contract
      --worker <WORKER>  Worker id (UUIDv7) — required unless `--dry-run`
      --dry-run          Print the planned mutations without persisting anything
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

#### `argo contract show`

```text
Render a worker's current state as TOML so it round-trips with `apply`. Reads charter + kpis blob + objectives/KRs

Usage: argo contract show [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7) to read

Options:
      --tenant <TENANT>  Tenant id whose worker namespace owns the row
      --json             Emit JSON instead of TOML
  -h, --help             Print help
```

#### `argo contract validate`

```text
Schema-check only — parses the file, runs semantic validation, and reports counts. Never touches the store

Usage: argo contract validate [OPTIONS] <FILE>

Arguments:
  <FILE>  Path to the `<role>.contract.toml` file

Options:
      --json  Emit JSON instead of a human-readable summary
  -h, --help  Print help
```

### `argo costs`

```text
Usage: argo costs [COMMAND]

Commands:
  cap          Set a hard cap (`--session`, `--day`, or `--month`) in USD. Pass `--reset` instead of a `<usd>` value to remove the cap (the field falls back to the workspace default)
  show         Show the configured caps + actual-spend roll-up
  export       Dump every row in `spend_ledger` as NDJSON (one JSON object per line). Pass `--out <path>` to write to a file; otherwise streams to stdout. `--since-minutes` scopes to recent rows
  alert        List sessions whose total spend exceeds `--threshold-usd`. Useful for spotting runaway sessions / abuse / model misconfiguration without parsing the ledger by hand
  total        Print just the today / month-to-date / all-time spend roll-up, nothing else. Mirrors the gateway's `GET /costs/total` endpoint
  by-day       Daily spend rollup — chronological list of `{day, spent_usd_micros}` for the last `--days` days. Mirrors the gateway's `GET /costs/by-day` endpoint
  by-model     Top-50 spend buckets grouped by `(provider, model)`. Mirrors the gateway's `GET /costs/by-model`
  by-provider  Top-50 spend buckets grouped by provider only (no model split). Mirrors the gateway's `GET /costs/by-provider` (#632)
  by-session   Top-50 spend buckets grouped by `session_id`. Mirrors the gateway's `GET /costs/by-session`
  recent       Workspace-wide recent spend ledger entries. Mirrors the gateway's `GET /costs/recent` (#625)
  forecast     Linear extrapolation of end-of-period spend from current trajectory: `rate = sum(spend in lookback) / lookback_seconds`, `projected_additional = rate × time-remaining`. The ±20% band is a flat v1 heuristic
  anomalies    Read-only spend anomaly table view
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo costs alert`

```text
List sessions whose total spend exceeds `--threshold-usd`. Useful for spotting runaway sessions / abuse / model misconfiguration without parsing the ledger by hand

Usage: argo costs alert [OPTIONS]

Options:
      --threshold-usd <THRESHOLD_USD>  USD threshold; sessions with `SUM(amount_usd_micros)` > `threshold * 1e6` are reported [default: 1]
      --json                           Output as NDJSON (one row per offending session)
  -h, --help                           Print help
```

#### `argo costs anomalies`

```text
Read-only spend anomaly table view

Usage: argo costs anomalies [OPTIONS]

Options:
      --since <SINCE>    Filter to anomalies observed after RFC3339 or YYYY-MM-DD
      --limit <LIMIT>    Cap row count (default 50, hard max 1000) [default: 50]
      --format <FORMAT>  Output format [default: markdown] [possible values: markdown, json]
      --tenant <TENANT>  Filter to one tenant id via the worker table
  -h, --help             Print help
```

#### `argo costs by-day`

```text
Daily spend rollup — chronological list of `{day, spent_usd_micros}` for the last `--days` days. Mirrors the gateway's `GET /costs/by-day` endpoint

Usage: argo costs by-day [OPTIONS]

Options:
      --days <DAYS>  Lookback window in days (default 30, cap 365) [default: 30]
      --json         Output as a single JSON object (`{by_day}`)
  -h, --help         Print help
```

#### `argo costs by-model`

```text
Top-50 spend buckets grouped by `(provider, model)`. Mirrors the gateway's `GET /costs/by-model`

Usage: argo costs by-model [OPTIONS]

Options:
      --json  Output as a single JSON object (`{by_model}`)
  -h, --help  Print help
```

#### `argo costs by-provider`

```text
Top-50 spend buckets grouped by provider only (no model split). Mirrors the gateway's `GET /costs/by-provider` (#632)

Usage: argo costs by-provider [OPTIONS]

Options:
      --json  Output as a single JSON object (`{by_provider}`)
  -h, --help  Print help
```

#### `argo costs by-session`

```text
Top-50 spend buckets grouped by `session_id`. Mirrors the gateway's `GET /costs/by-session`

Usage: argo costs by-session [OPTIONS]

Options:
      --json  Output as a single JSON object (`{by_session}`)
  -h, --help  Print help
```

#### `argo costs cap`

```text
Set a hard cap (`--session`, `--day`, or `--month`) in USD. Pass `--reset` instead of a `<usd>` value to remove the cap (the field falls back to the workspace default)

Usage: argo costs cap [OPTIONS] [USD]

Arguments:
  [USD]  USD cap value. Either this OR `--reset` must be set

Options:
      --session  
      --day      
      --month    
      --reset    Remove the cap field instead of setting it
  -h, --help     Print help
```

#### `argo costs export`

```text
Dump every row in `spend_ledger` as NDJSON (one JSON object per line). Pass `--out <path>` to write to a file; otherwise streams to stdout. `--since-minutes` scopes to recent rows

Usage: argo costs export [OPTIONS]

Options:
      --out <OUT>                      
      --since-minutes <SINCE_MINUTES>  Filter to rows charged within this many recent minutes
  -h, --help                           Print help
```

#### `argo costs forecast`

```text
Linear extrapolation of end-of-period spend from current trajectory: `rate = sum(spend in lookback) / lookback_seconds`, `projected_additional = rate × time-remaining`. The ±20% band is a flat v1 heuristic

Usage: argo costs forecast [OPTIONS]

Options:
      --horizon <HORIZON>
          Horizon: end-of-day, end-of-month, or end-of-quarter

          Possible values:
          - eod: End of today (UTC)
          - eom: End of current month (UTC)
          - eoq: End of current quarter (UTC)
          
          [default: eom]

      --lookback <LOOKBACK>
          Lookback window. Suffixes: `s|m|h|d` (e.g. `24h`, `7d`, `30d`)
          
          [default: 24h]

      --format <FORMAT>
          Output format

          Possible values:
          - markdown: Markdown table (default)
          - json:     Single-line JSON object
          
          [default: markdown]

  -h, --help
          Print help (see a summary with '-h')
```

#### `argo costs recent`

```text
Workspace-wide recent spend ledger entries. Mirrors the gateway's `GET /costs/recent` (#625)

Usage: argo costs recent [OPTIONS]

Options:
      --limit <LIMIT>                  Tail size (default 50, cap 500) [default: 50]
      --since-minutes <SINCE_MINUTES>  Filter to charges within this many recent minutes
      --json                           Output as a JSON array
  -h, --help                           Print help
```

#### `argo costs show`

```text
Show the configured caps + actual-spend roll-up

Usage: argo costs show [OPTIONS]

Options:
      --json             Output as a single JSON object. Equivalent to `--format json`
      --by-model         Group spend by `(provider, model)` and print the top spenders instead of the default ceilings view
      --by-session       Group spend by `session_id` and print the top spenders instead of the default ceilings view
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
  -h, --help             Print help
```

#### `argo costs total`

```text
Print just the today / month-to-date / all-time spend roll-up, nothing else. Mirrors the gateway's `GET /costs/total` endpoint

Usage: argo costs total [OPTIONS]

Options:
      --json  Output as a single JSON object (`{today, this_month, all_time}` in usd_micros) for scripting
  -h, --help  Print help
```

### `argo cred`

```text
Workspace credential storage and shell export helpers

Usage: argo cred <COMMAND>

Commands:
  set                 Store one credential. If value is omitted, prompt without echo
  list                List stored credential keys; values are masked by default
  clear               Remove one stored credential
  clear-all           Remove every stored credential
  export              Emit shell export statements for stored credentials
  import              Import credential-looking values from the current environment
  backend             Set the credential storage backend in `argo.toml` (`[credentials] backend = "file" | "keyring" | "auto"`)
  migrate-to-keyring  Migrate workspace credentials from the file store into the OS keyring. Atomic: either every key migrates or none — keyring writes are rolled back on partial failure and the file is left intact
  encrypt-all         Rewrite the workspace credentials file with every plaintext entry wrapped in an age envelope under the workspace master key (`ARGO_CREDENTIALS_MASTER_KEY` env or `~/.skycot/.master_key` file). Already-encrypted entries pass through unchanged
  help                Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo cred backend`

```text
Set the credential storage backend in `argo.toml` (`[credentials] backend = "file" | "keyring" | "auto"`)

Usage: argo cred backend <BACKEND>

Arguments:
  <BACKEND>  One of: `file`, `keyring`, `auto`

Options:
  -h, --help  Print help
```

#### `argo cred clear`

```text
Remove one stored credential

Usage: argo cred clear <KEY>

Arguments:
  <KEY>  

Options:
  -h, --help  Print help

Examples:
  argo cred clear OPENAI_API_KEY
```

#### `argo cred clear-all`

```text
Remove every stored credential

Usage: argo cred clear-all

Options:
  -h, --help  Print help

Examples:
  argo cred clear-all
```

#### `argo cred encrypt-all`

```text
Rewrite the workspace credentials file with every plaintext entry wrapped in an age envelope under the workspace master key (`ARGO_CREDENTIALS_MASTER_KEY` env or `~/.skycot/.master_key` file). Already-encrypted entries pass through unchanged

Usage: argo cred encrypt-all [OPTIONS]

Options:
      --path <PATH>  Override the credentials file path (defaults to `<workspace>/.argo/credentials.toml`)
      --json         Emit a JSON report instead of human-readable output
  -h, --help         Print help
```

#### `argo cred export`

```text
Emit shell export statements for stored credentials

Usage: argo cred export --shell <SHELL>

Options:
      --shell <SHELL>  
  -h, --help           Print help

Examples:
  eval "$(argo cred export --shell bash)"
  argo cred export --shell zsh
```

#### `argo cred import`

```text
Import credential-looking values from the current environment

Usage: argo cred import --shell <SHELL>

Options:
      --shell <SHELL>  
  -h, --help           Print help

Examples:
  argo cred import --shell bash
```

#### `argo cred list`

```text
List stored credential keys; values are masked by default

Usage: argo cred list [OPTIONS]

Options:
      --reveal  
  -h, --help    Print help

Examples:
  argo cred list
  argo cred list --reveal
```

#### `argo cred migrate-to-keyring`

```text
Migrate workspace credentials from the file store into the OS keyring. Atomic: either every key migrates or none — keyring writes are rolled back on partial failure and the file is left intact

Usage: argo cred migrate-to-keyring [OPTIONS]

Options:
      --dry-run                List entries that would be migrated without modifying any state
      --from-file <FROM_FILE>  Override the source credentials file (defaults to `<workspace>/.argo/credentials.toml`)
  -h, --help                   Print help
```

#### `argo cred set`

```text
Store one credential. If value is omitted, prompt without echo

Usage: argo cred set <KEY> [VALUE]

Arguments:
  <KEY>    
  [VALUE]  

Options:
  -h, --help  Print help

Examples:
  argo cred set ANTHROPIC_API_KEY
  argo cred set OPENAI_API_KEY sk-...   # value inline (history risk)
```

### `argo cron`

```text
Usage: argo cron <COMMAND>

Commands:
  list         List scheduled jobs in the workspace's `cron/jobs/` directory
  create       Create a new job (`<name> <schedule> <prompt> <platform> <chat>`)
  edit         
  delete       Delete a job by name
  trigger      Trigger a job immediately (mainly for tests / manual reruns)
  tick         Run one scheduler tick at the current time. Reports which jobs would fire; pass `--dry-run` to skip writing the last-fired cursor
  daemon       Run the cron tick loop in the foreground at a fixed interval, holding `cron/.tick.lock` so multiple daemons don't double-fire. Exits with a non-zero status if the lock is already held
  init-system  Install the four canonical system maintenance jobs (Forge optimize / quality recompute / Dreams curation / A/B promotion) into `cron/jobs/` by copying the templates from `cron/system/`. Existing files are preserved unless `--force` is passed
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo cron create`

```text
Create a new job (`<name> <schedule> <prompt> <platform> <chat>`)

Usage: argo cron create <NAME> <SCHEDULE> <PROMPT> <PLATFORM> <CHAT>

Arguments:
  <NAME>      
  <SCHEDULE>  
  <PROMPT>    
  <PLATFORM>  
  <CHAT>      

Options:
  -h, --help  Print help
```

#### `argo cron daemon`

```text
Run the cron tick loop in the foreground at a fixed interval, holding `cron/.tick.lock` so multiple daemons don't double-fire. Exits with a non-zero status if the lock is already held

Usage: argo cron daemon [OPTIONS]

Options:
      --interval-secs <INTERVAL_SECS>  Interval between ticks (seconds). Default 60s [default: 60]
      --max-ticks <MAX_TICKS>          Stop after N ticks (mainly for tests / manual checks). Omit to run until SIGINT
  -h, --help                           Print help
```

#### `argo cron delete`

```text
Delete a job by name

Usage: argo cron delete <NAME>

Arguments:
  <NAME>  

Options:
  -h, --help  Print help
```

#### `argo cron edit`

```text
Usage: argo cron edit <NAME>

Arguments:
  <NAME>  

Options:
  -h, --help  Print help
```

#### `argo cron init-system`

```text
Install the four canonical system maintenance jobs (Forge optimize / quality recompute / Dreams curation / A/B promotion) into `cron/jobs/` by copying the templates from `cron/system/`. Existing files are preserved unless `--force` is passed

Usage: argo cron init-system [OPTIONS]

Options:
      --dry-run  Preview what would be written without touching disk
      --force    Overwrite existing job files in `cron/jobs/`
  -h, --help     Print help
```

#### `argo cron list`

```text
List scheduled jobs in the workspace's `cron/jobs/` directory

Usage: argo cron list [OPTIONS]

Options:
      --json             Output as JSON (one object per job) for scripting. Equivalent to `--format json`
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
  -h, --help             Print help
```

#### `argo cron tick`

```text
Run one scheduler tick at the current time. Reports which jobs would fire; pass `--dry-run` to skip writing the last-fired cursor

Usage: argo cron tick [OPTIONS]

Options:
      --dry-run  
  -h, --help     Print help
```

#### `argo cron trigger`

```text
Trigger a job immediately (mainly for tests / manual reruns)

Usage: argo cron trigger <NAME>

Arguments:
  <NAME>  

Options:
  -h, --help  Print help
```

### `argo cu`

```text
Computer-use trajectory inspection (`claude-cu-action-replay-cli`). `replay <session-id>` renders a recorded CU dispatch as markdown (+ optional GIF)

Usage: argo cu <COMMAND>

Commands:
  replay  Reconstruct a recorded computer-use session as a navigable artifact (markdown trajectory; optional GIF). Mirrors `argo session replay --stream` shape but specialised for the CU per-action `browser.jsonl` + `step-NNN.png` bundle
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo cu replay`

```text
Reconstruct a recorded computer-use session as a navigable artifact (markdown trajectory; optional GIF). Mirrors `argo session replay --stream` shape but specialised for the CU per-action `browser.jsonl` + `step-NNN.png` bundle

Usage: argo cu replay [OPTIONS] <SESSION_ID>

Arguments:
  <SESSION_ID>  CU session id — the `action_id` of the dispatch whose trajectory to render. Cross-tenant lookups return `NotFound`

Options:
      --out <OUT>          Write markdown to PATH instead of stdout. Ignored when `--format json`
      --gif <GIF>          Also encode the per-step screenshots as a GIF written to PATH. Default fps = 1 (CU steps are slow)
      --gif-fps <GIF_FPS>  Frames per second for `--gif`. Bound `1..=30` [default: 1]
      --from <FROM>        Restrict the rendered trajectory to steps `>= N` (1-indexed, inclusive)
      --to <TO>            Restrict the rendered trajectory to steps `<= N` (1-indexed, inclusive)
      --inline             Embed screenshots as `data:image/png;base64,...` URLs rather than `file://` paths. Useful when the markdown will be shipped to a host that doesn't see the bundle directory
      --format <FORMAT>    `markdown` (default) or `json`. JSON emits NDJSON, one-step-per-line, mirroring `session replay --stream` [default: markdown]
  -h, --help               Print help
```

### `argo daemon`

```text
macOS launchd-agent install/uninstall (Stream `claude-installer-macos`). Writes `~/Library/LaunchAgents/com.skycot.argo.plist` so the gateway starts on login + auto-restarts on crash. macOS-only

Usage: argo daemon <COMMAND>

Commands:
  install    Render the launchd plist + write it to `<launch-agents-dir>/com.skycot.argo.plist` + run `launchctl load` on it. `--dry-run` prints the rendered plist without writing
  uninstall  `launchctl unload` the agent + remove the plist file. Idempotent — succeeds even if the agent isn't installed
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo daemon install`

```text
Render the launchd plist + write it to `<launch-agents-dir>/com.skycot.argo.plist` + run `launchctl load` on it. `--dry-run` prints the rendered plist without writing

Usage: argo daemon install [OPTIONS]

Options:
      --json                      Emit `{plist_path, bin, loaded}` as JSON instead of the human-readable summary
      --dry-run                   Print the rendered plist + target path without writing to disk or invoking `launchctl`
      --bin <BIN>                 Override the path to the `argo` binary written into the plist's ProgramArguments. Defaults to the running binary's path (`std::env::current_exe()`)
      --launch-agents-dir <PATH>  Override the LaunchAgents directory. Defaults to `$HOME/Library/LaunchAgents`. Tests use this to point at a TempDir
  -h, --help                      Print help

Examples:
  argo daemon install
  argo daemon install --dry-run
  argo daemon install --json
```

#### `argo daemon uninstall`

```text
`launchctl unload` the agent + remove the plist file. Idempotent — succeeds even if the agent isn't installed

Usage: argo daemon uninstall [OPTIONS]

Options:
      --json                      
      --dry-run                   
      --launch-agents-dir <PATH>  
  -h, --help                      Print help

Examples:
  argo daemon uninstall
  argo daemon uninstall --dry-run --json
```

### `argo db`

```text
Usage: argo db <COMMAND>

Commands:
  migrate-up    Migrate up to latest
  migrate-down  Migrate down to a specific version
  status        Show current schema version + per-table row counts
  prune         Drop sessions whose `started_at` is older than `--older-than-days` days. Pass `--force` to actually delete; otherwise dry-runs
  vacuum        Run SQLite `VACUUM` to reclaim file-space after a prune
  version       Print the current SQLite schema migration version. Same number `db status` reports under "schema_version", but scripts often just want the bare integer
  integrity     Run SQLite `PRAGMA integrity_check` against the workspace store. Prints "ok" when healthy, otherwise the diagnostic rows. Mirrors the gateway's `GET /db/integrity`
  rekey         Rotate the at-rest encryption key in place via `PRAGMA rekey` (Stream `mission-encryption-at-rest-sqlcipher`). The old key is loaded from `[security.encryption] key_source` in `argo.toml`; the new key is supplied via `--new-key-source`. Stock builds (no SQLCipher-linked driver) succeed but do not actually encrypt — see `docs/skycot/RUNBOOK.md`
  help          Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo db integrity`

```text
Run SQLite `PRAGMA integrity_check` against the workspace store. Prints "ok" when healthy, otherwise the diagnostic rows. Mirrors the gateway's `GET /db/integrity`

Usage: argo db integrity [OPTIONS]

Options:
      --json  Output as `{ok, check}` JSON
  -h, --help  Print help
```

#### `argo db migrate-down`

```text
Migrate down to a specific version

Usage: argo db migrate-down --to <TO>

Options:
      --to <TO>  
  -h, --help     Print help
```

#### `argo db migrate-up`

```text
Migrate up to latest

Usage: argo db migrate-up

Options:
  -h, --help  Print help
```

#### `argo db prune`

```text
Drop sessions whose `started_at` is older than `--older-than-days` days. Pass `--force` to actually delete; otherwise dry-runs

Usage: argo db prune [OPTIONS]

Options:
      --older-than-days <OLDER_THAN_DAYS>  [default: 90]
      --force                              
  -h, --help                               Print help
```

#### `argo db rekey`

```text
Rotate the at-rest encryption key in place via `PRAGMA rekey` (Stream `mission-encryption-at-rest-sqlcipher`). The old key is loaded from `[security.encryption] key_source` in `argo.toml`; the new key is supplied via `--new-key-source`. Stock builds (no SQLCipher-linked driver) succeed but do not actually encrypt — see `docs/skycot/RUNBOOK.md`

Usage: argo db rekey [OPTIONS] --new-key-source <NEW_KEY_SOURCE>

Options:
      --new-key-source <NEW_KEY_SOURCE>
          `env:VAR` or `file:/path` for the *new* key. `none` is rejected — disabling encryption is a separate workflow
      --json
          JSON output for scripting
  -h, --help
          Print help
```

#### `argo db status`

```text
Show current schema version + per-table row counts

Usage: argo db status [OPTIONS]

Options:
      --json             Output as a single JSON object. Equivalent to `--format json`
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
  -h, --help             Print help
```

#### `argo db vacuum`

```text
Run SQLite `VACUUM` to reclaim file-space after a prune

Usage: argo db vacuum

Options:
  -h, --help  Print help
```

#### `argo db version`

```text
Print the current SQLite schema migration version. Same number `db status` reports under "schema_version", but scripts often just want the bare integer

Usage: argo db version [OPTIONS]

Options:
      --json  Output as a single JSON object `{version}`
  -h, --help  Print help
```

### `argo doctor`

```text
Diagnose configuration / sandbox / port issues

Usage: argo doctor [OPTIONS] [COMMAND]

Commands:
  approvals      List approval requests expired by the timeout sweeper
  fix            Apply safe doctor remediations, or one named fix with --issue
  watch          Continuously emit the doctor JSON envelope until Ctrl-C
  wal-stats      Inspect SQLite WAL state — checkpoints the WAL (PASSIVE) and reports pragma readouts plus `*.db-wal` file size. Operator inspection for the `mission-sqlite-wal-tuning` profiles
  install-hooks  Copy the canonical pre-commit secret-scan hook into `.git/hooks/pre-commit`. Refuses to overwrite an existing hook unless `--force` is passed (operator may have local customisations). Mission `mission-secret-scanning-pre-commit-hook`
  throttles      List the in-process per-action-type throttle buckets (`mission-per-action-type-throttling`). Reports the configured policies and any live `(tenant_id, action_type)` bucket state held by the dispatch gate. Output is empty when `[runtime.action_throttle] enabled = false`
  help           Print this message or the help of the given subcommand(s)

Options:
      --json             Emit a single JSON object with all the checks. Default is the human-readable text report. Equivalent to `--format json`
      --summary          Emit a one-line health summary. With `--json`, emits a compact summary envelope instead of the full report
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
      --skip-network     Skip DNS, TLS, and provider-auth probes. Also honored when ARGO_DOCTOR_SKIP_NETWORK=1 is set
  -h, --help             Print help

Examples:
  argo doctor
  argo doctor --json
  argo doctor --summary
```

#### `argo doctor approvals`

```text
List approval requests expired by the timeout sweeper

Usage: argo doctor approvals [OPTIONS]

Options:
      --since <SINCE>  Look-back window. Supports bare seconds plus `s`, `m`, and `h` suffixes [default: 24h]
      --json           Emit a JSON array instead of text
  -h, --help           Print help
```

#### `argo doctor fix`

```text
Apply safe doctor remediations, or one named fix with --issue

Usage: argo doctor fix [OPTIONS]

Options:
      --auto           Allow confirmation-level fixes such as migrations
      --issue <ISSUE>  Run only one fix id
      --json           Emit a JSON envelope with fix results
  -h, --help           Print help
```

#### `argo doctor install-hooks`

```text
Copy the canonical pre-commit secret-scan hook into `.git/hooks/pre-commit`. Refuses to overwrite an existing hook unless `--force` is passed (operator may have local customisations). Mission `mission-secret-scanning-pre-commit-hook`

Usage: argo doctor install-hooks [OPTIONS]

Options:
      --force  Overwrite the existing `.git/hooks/pre-commit` even if the operator has a custom hook there
      --json   Emit a JSON envelope describing the install outcome
  -h, --help   Print help
```

#### `argo doctor throttles`

```text
List the in-process per-action-type throttle buckets (`mission-per-action-type-throttling`). Reports the configured policies and any live `(tenant_id, action_type)` bucket state held by the dispatch gate. Output is empty when `[runtime.action_throttle] enabled = false`

Usage: argo doctor throttles [OPTIONS]

Options:
      --json  Emit a single JSON object instead of the text report
  -h, --help  Print help
```

#### `argo doctor wal-stats`

```text
Inspect SQLite WAL state — checkpoints the WAL (PASSIVE) and reports pragma readouts plus `*.db-wal` file size. Operator inspection for the `mission-sqlite-wal-tuning` profiles

Usage: argo doctor wal-stats [OPTIONS]

Options:
      --json  Emit a single JSON object instead of the text report
  -h, --help  Print help
```

#### `argo doctor watch`

```text
Continuously emit the doctor JSON envelope until Ctrl-C

Usage: argo doctor watch [OPTIONS]

Options:
      --interval <INTERVAL>  Poll interval. Supports bare seconds plus `ms`, `s`, and `m` suffixes [default: 30s]
      --clear                Clear the terminal before each cycle for a live dashboard redraw
  -h, --help                 Print help
```

### `argo duty`

```text
Worker duty scheduler — autonomous cron-driven actions (Stream `claude-worker-duty-scheduler`, V1.3). list / show / pause / resume / fire-now

Usage: argo duty <COMMAND>

Commands:
  list      List duties for one tenant in created-at order
  show      Show one duty by id (uuid)
  pause     Pause a duty until the given UTC instant. Supply `--until <RFC3339>` for an explicit deadline; without it the duty pauses 24h from now
  resume    Resume a paused duty (clears `paused_until`)
  fire-now  Fire a duty immediately, ignoring cron + gate. Records the fire as a `manual_fire` decision in the link table
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo duty fire-now`

```text
Fire a duty immediately, ignoring cron + gate. Records the fire as a `manual_fire` decision in the link table

Usage: argo duty fire-now [OPTIONS] <ID>

Arguments:
  <ID>  Duty id (uuid)

Options:
      --json  Emit JSON
  -h, --help  Print help
```

#### `argo duty list`

```text
List duties for one tenant in created-at order

Usage: argo duty list [OPTIONS] --tenant <TENANT>

Options:
      --tenant <TENANT>  Tenant id (uuid)
      --json             Emit JSON instead of the human table
  -h, --help             Print help
```

#### `argo duty pause`

```text
Pause a duty until the given UTC instant. Supply `--until <RFC3339>` for an explicit deadline; without it the duty pauses 24h from now

Usage: argo duty pause [OPTIONS] <ID>

Arguments:
  <ID>  Duty id (uuid)

Options:
      --until <UNTIL>  Resume-at instant (RFC 3339, UTC)
      --json           Emit JSON
  -h, --help           Print help
```

#### `argo duty resume`

```text
Resume a paused duty (clears `paused_until`)

Usage: argo duty resume [OPTIONS] <ID>

Arguments:
  <ID>  Duty id (uuid)

Options:
      --json  Emit JSON
  -h, --help  Print help
```

#### `argo duty show`

```text
Show one duty by id (uuid)

Usage: argo duty show [OPTIONS] <ID>

Arguments:
  <ID>  Duty id (uuid)

Options:
      --json  Emit JSON
  -h, --help  Print help
```

### `argo errors`

```text
Error-rate dashboards backed by canonical audit rows

Usage: argo errors <COMMAND>

Commands:
  dashboard  Show HTTP 5xx rate from `audit_record` request rows
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo errors dashboard`

```text
Show HTTP 5xx rate from `audit_record` request rows

Usage: argo errors dashboard [OPTIONS]

Options:
      --window <WINDOW>  Time window to inspect, e.g. `30m`, `1h`, or `7d` [default: 1h]
      --tenant <TENANT>  Optional tenant id filter
      --route <ROUTE>    Optional route/path filter. A trailing `*` is treated as a prefix
      --json             Emit a JSON envelope instead of a table
  -h, --help             Print help
```

### `argo eval`

```text
Validate eval JSONL, or run/list vertical eval fixtures

Usage: argo eval <COMMAND>

Commands:
  validate       Validate eval seeds. Accepts either a JSONL path (legacy generic task format) or a vertical id; the latter loads the vertical's eval suites and reports a pass-rate envelope (`mvp-vertical-eval-harness`)
  run            Run a vertical's eval fixtures through the dry-run worker pipeline and report pass/fail totals
  list           List recent eval runs. With `--vertical`, filters to one vertical
  show           Show a single eval run
  skill-honesty  Self-evaluation honesty test: sample recent skill runs, re-judge them via `argo-judge::adjudicate`, and flag the skill as SUSPICIOUS when its self-reported success rate diverges from the independent judge by more than the configured threshold (mission-eval-self-honesty-test). Diagnostic only — does not demote trust grades
  compare        Compare two eval runs side by side
  help           Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo eval compare`

```text
Compare two eval runs side by side

Usage: argo eval compare [OPTIONS] <RUN_A> <RUN_B>

Arguments:
  <RUN_A>  Baseline eval run id or JSON path
  <RUN_B>  Candidate eval run id or JSON path

Options:
      --by <BY>            Group deltas by fixture, skill, or metric [default: fixture]
      --show-regressions   Show only fixtures worse than run A in markdown output
      --show-improvements  Show only fixtures better than run A in markdown output
      --json               Output as a single JSON object
  -h, --help               Print help
```

#### `argo eval list`

```text
List recent eval runs. With `--vertical`, filters to one vertical

Usage: argo eval list [OPTIONS]

Options:
      --vertical <VERTICAL>  Vertical id to filter by
      --limit <LIMIT>        Maximum number of runs to return [default: 20]
      --json                 Output as a single JSON object
  -h, --help                 Print help
```

#### `argo eval run`

```text
Run a vertical's eval fixtures through the dry-run worker pipeline and report pass/fail totals

Usage: argo eval run [OPTIONS] --vertical <VERTICAL>

Options:
      --vertical <VERTICAL>  Vertical id to load from $ARGO_VERTICALS_DIR or ./verticals
      --score                Also score every fixture with the deterministic judge rules exposed by argo-vertical
      --limit <LIMIT>        Cap the number of fixtures executed, after manifest suite order is applied
      --stream <STREAM>      Streaming output mode (Stream `claude-eval-streaming`): `off` (batch — historical default), `simple` (default — one line per fixture as it completes), `verbose` (NDJSON per fixture for piping). Combine with `--json` to skip per-fixture lines and emit only the JSON envelope at the end [default: simple]
      --json                 Output as a single JSON object
  -h, --help                 Print help
```

#### `argo eval show`

```text
Show a single eval run

Usage: argo eval show [OPTIONS] <RUN>

Arguments:
  <RUN>  Eval run id or path to an eval run JSON file

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

#### `argo eval skill-honesty`

```text
Self-evaluation honesty test: sample recent skill runs, re-judge them via `argo-judge::adjudicate`, and flag the skill as SUSPICIOUS when its self-reported success rate diverges from the independent judge by more than the configured threshold (mission-eval-self-honesty-test). Diagnostic only — does not demote trust grades

Usage: argo eval skill-honesty [OPTIONS] <SKILL_ID>

Arguments:
  <SKILL_ID>  Skill id to audit

Options:
      --sample <SAMPLE>      Override the default 20-sample window
      --provider <PROVIDER>  Provider manifest id used to construct the judge transport [default: anthropic]
      --model <MODEL>        Judge model id (e.g. `claude-sonnet-4-6`) [default: claude-sonnet-4-6]
      --json                 Output as a single JSON object
  -h, --help                 Print help
```

#### `argo eval validate`

```text
Validate eval seeds. Accepts either a JSONL path (legacy generic task format) or a vertical id; the latter loads the vertical's eval suites and reports a pass-rate envelope (`mvp-vertical-eval-harness`)

Usage: argo eval validate [OPTIONS] <TARGET>

Arguments:
  <TARGET>  Either a path to a JSONL fixtures file, or the id of a vertical loadable from `$ARGO_VERTICALS_DIR`/`./verticals`

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

### `argo feedback`

```text
Operator-supplied thumbs up/down per action (Stream `claude-operator-feedback-channel`). Feeds the skill-learning loop as a strong-positive / strong-negative signal alongside the heuristic success metric

Usage: argo feedback <COMMAND>

Commands:
  up    Record a thumbs-up for an action
  down  Record a thumbs-down for an action
  list  List feedback rows. Optionally filter by worker and/or vote
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo feedback down`

```text
Record a thumbs-down for an action

Usage: argo feedback down [OPTIONS] <ACTION_ID>

Arguments:
  <ACTION_ID>  Action id the operator is voting on

Options:
      --comment <COMMENT>  Optional free-form comment captured alongside the vote
      --worker <WORKER>    Tag the vote with the worker that emitted the action
      --skill <SKILL>      Skill name this action invoked (Stream `mission-skill-ratings-publisher`). When provided and the underlying skill is marketplace-installed, the vote is ALSO recorded as a publisher-side `skill_rating` row alongside the local `operator_feedback` row
      --json               Output as a single JSON object
  -h, --help               Print help
```

#### `argo feedback list`

```text
List feedback rows. Optionally filter by worker and/or vote

Usage: argo feedback list [OPTIONS]

Options:
      --worker <WORKER>  Restrict to one worker's actions
      --vote <VOTE>      Restrict to one vote (`up` or `down`)
      --limit <LIMIT>    Cap the number of rows printed. Default 50 [default: 50]
      --json             Output as a single JSON array
  -h, --help             Print help
```

#### `argo feedback up`

```text
Record a thumbs-up for an action

Usage: argo feedback up [OPTIONS] <ACTION_ID>

Arguments:
  <ACTION_ID>  Action id the operator is voting on

Options:
      --comment <COMMENT>  Optional free-form comment captured alongside the vote
      --worker <WORKER>    Tag the vote with the worker that emitted the action. Used by `argo feedback list --worker <id>`
      --skill <SKILL>      Skill name this action invoked (Stream `mission-skill-ratings-publisher`). When provided and the underlying skill is marketplace-installed, the vote is ALSO recorded as a publisher-side `skill_rating` row alongside the local `operator_feedback` row
      --json               Output as a single JSON object
  -h, --help               Print help
```

### `argo forge`

```text
Usage: argo forge <COMMAND>

Commands:
  optimize     
  bench        Run an eval-task JSONL file through the agent loop and score each task. Records a `forge_run` row with the mean score + pass rate when the run finishes. Skill name is captured for the row's metadata; the active skill body isn't (yet) injected into the prompt — that lands with the daemon-side prompt-loader PR
  promote      
  runs         
  enqueue      Enqueue a forge optimize run for the v1.1 daemon to pick up later. Creates a `forge_run` row with status='queued' and a parallel `forge_queue` payload. Returns the new run id; the daemon claims + runs it on its next tick. Use `argo forge runs list --status queued` to see what's waiting
  daemon       Long-running standalone daemon — periodically sweeps stalled `running` forge_run rows to `failed`. First step in the v1.1 forge-daemon split: replaces the manual `argo forge runs sweep` cadence with a worker that survives gateway restarts. Future iterations will pick up queued rows and run them
  cost-report  Per-skill cost-attribution rollup over the recorded `forge_run` history (mission-forge-cost-attribution). Sums per-call token counts, model-priced LLM cost, E2B sandbox cost, and a stamped total — grouped by `skill_name`, ordered by total cost descending. Lets operators identify which skills cost the most to optimize and tune their budgets accordingly
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo forge bench`

```text
Run an eval-task JSONL file through the agent loop and score each task. Records a `forge_run` row with the mean score + pass rate when the run finishes. Skill name is captured for the row's metadata; the active skill body isn't (yet) injected into the prompt — that lands with the daemon-side prompt-loader PR

Usage: argo forge bench [OPTIONS] --examples <EXAMPLES> <SKILL>

Arguments:
  <SKILL>  

Options:
      --examples <EXAMPLES>  Path to a JSONL eval-task file (parsed by `argo eval validate`)
      --provider <PROVIDER>  Provider id (matches `manifests/providers/<id>.toml`). Defaults to `anthropic` [default: anthropic]
      --model <MODEL>        Model id (matches an entry in the manifest's `[[models]]` list). Defaults to `claude-3-5-sonnet-20241022` [default: claude-3-5-sonnet-20241022]
  -h, --help                 Print help
```

#### `argo forge cost-report`

```text
Per-skill cost-attribution rollup over the recorded `forge_run` history (mission-forge-cost-attribution). Sums per-call token counts, model-priced LLM cost, E2B sandbox cost, and a stamped total — grouped by `skill_name`, ordered by total cost descending. Lets operators identify which skills cost the most to optimize and tune their budgets accordingly

Usage: argo forge cost-report [OPTIONS]

Options:
      --skill <SKILL>  Exact-match filter on `skill_name`. Omit to aggregate across every skill in the recorded history
      --since <SINCE>  Start of the time window (inclusive lower bound on `started_at`). Accepts RFC-3339 (`2026-05-01T00:00:00Z`) or `YYYY-MM-DD`. Omit for "all time"
      --json           Output as a single JSON envelope `{rows:[…], totals}` for scripting
  -h, --help           Print help
```

#### `argo forge daemon`

```text
Long-running standalone daemon — periodically sweeps stalled `running` forge_run rows to `failed`. First step in the v1.1 forge-daemon split: replaces the manual `argo forge runs sweep` cadence with a worker that survives gateway restarts. Future iterations will pick up queued rows and run them

Usage: argo forge daemon [OPTIONS]

Options:
      --interval-secs <INTERVAL_SECS>
          Seconds between sweep ticks (default 60) [default: 60]
      --older-than-minutes <OLDER_THAN_MINUTES>
          Stalled-row cutoff in minutes (default 60). Rows stuck in `running` older than this get flipped to `failed` on each tick [default: 60]
      --max-ticks <MAX_TICKS>
          Stop after this many ticks (test/CI hook). Default runs forever
  -h, --help
          Print help
```

#### `argo forge enqueue`

```text
Enqueue a forge optimize run for the v1.1 daemon to pick up later. Creates a `forge_run` row with status='queued' and a parallel `forge_queue` payload. Returns the new run id; the daemon claims + runs it on its next tick. Use `argo forge runs list --status queued` to see what's waiting

Usage: argo forge enqueue [OPTIONS] --examples <EXAMPLES> <SKILL>

Arguments:
  <SKILL>  

Options:
      --examples <EXAMPLES>      
      --iterations <ITERATIONS>  [default: 12]
      --budget <BUDGET>          [default: 5]
      --json                     Output the new run id as JSON instead of plain text
  -h, --help                     Print help
```

#### `argo forge optimize`

```text
Usage: argo forge optimize [OPTIONS] --examples <EXAMPLES> <SKILL>

Arguments:
  <SKILL>  

Options:
      --examples <EXAMPLES>      
      --iterations <ITERATIONS>  [default: 12]
      --budget <BUDGET>          [default: 5]
      --stream                   Emit progress as newline-delimited JSON on stdout instead of the human-readable table. Each line is either `{"event":"start", run_id, ...}`, `{"event":"iter", iter, version, mean_score, …}`, or `{"event":"done", best_score, best_version, status}`. Lets the TUI / watch scripts / gateway consume forge progress without parsing the table
  -h, --help                     Print help
```

#### `argo forge promote`

```text
Usage: argo forge promote --version <VERSION> <SKILL>

Arguments:
  <SKILL>  

Options:
      --version <VERSION>  
  -h, --help               Print help
```

#### `argo forge runs`

```text
Usage: argo forge runs <COMMAND>

Commands:
  count      Print just the count of recorded forge runs
  list       List recorded forge runs. Mirrors the gateway's `/forge/runs` endpoint with the same filter set
  show       
  abort      
  requeue    Requeue a terminal forge run (failed / aborted / succeeded) by copying its queued payload into a fresh row with status='queued'. The daemon picks it up on its next tick. The source row stays as a historical record. 404 when the source has no payload (was run via `?in_process=true`). Mirrors the gateway's `POST /forge/runs/<id>/requeue`
  payload    Show the queued payload (skill / examples / iterations / budget_usd) for a forge run. Mirrors the gateway's `GET /forge/runs/<id>/payload`. 404 when the row has no forge_queue entry (e.g. ran via `?in_process=true`)
  audit      Audit rows linked to one forge run via the payload's `$.run_id` field. Mirrors the gateway's `GET /forge/runs/<id>/audit` (#634). Returns the forge.run.start + forge.run.end + per-iter rows ordered by seq ASC
  abort-all  Flip every running forge run to `aborted`. Operator emergency-stop / fresh-slate before a workspace migration. Mirrors `POST /forge/runs/abort-all`
  sweep      Sweep stalled `running` rows whose `started_at` is older than `--older-than-minutes` ago (default 60). Used to recover from a gateway crash mid-optimize: in-memory state is gone, but the row stays in `running` until manually aborted. Flips status to `failed`. Mirrors `POST /forge/runs/sweep`
  by-day     Daily forge-run start rollup. Mirrors the gateway's `GET /forge/runs/by-day` (#572)
  by-status  Histogram of forge_run rows grouped by `status`. Mirrors the gateway's `GET /forge/runs/by-status` (#584)
  by-skill   Histogram of forge_run rows grouped by `skill_name`. Mirrors the gateway's `GET /forge/runs/by-skill` (#598)
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo forge runs abort`

```text
Usage: argo forge runs abort <ID>

Arguments:
  <ID>  

Options:
  -h, --help  Print help
```

##### `argo forge runs abort-all`

```text
Flip every running forge run to `aborted`. Operator emergency-stop / fresh-slate before a workspace migration. Mirrors `POST /forge/runs/abort-all`

Usage: argo forge runs abort-all

Options:
  -h, --help  Print help
```

##### `argo forge runs audit`

```text
Audit rows linked to one forge run via the payload's `$.run_id` field. Mirrors the gateway's `GET /forge/runs/<id>/audit` (#634). Returns the forge.run.start + forge.run.end + per-iter rows ordered by seq ASC

Usage: argo forge runs audit [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --json  Output as a single JSON object {run_id, rows:[…]}
  -h, --help  Print help
```

##### `argo forge runs by-day`

```text
Daily forge-run start rollup. Mirrors the gateway's `GET /forge/runs/by-day` (#572)

Usage: argo forge runs by-day [OPTIONS]

Options:
      --days <DAYS>  Lookback window in days (default 30, cap 365) [default: 30]
      --json         Output as a single JSON object (`{by_day}`)
  -h, --help         Print help
```

##### `argo forge runs by-skill`

```text
Histogram of forge_run rows grouped by `skill_name`. Mirrors the gateway's `GET /forge/runs/by-skill` (#598)

Usage: argo forge runs by-skill [OPTIONS]

Options:
      --since-minutes <SINCE_MINUTES>  Filter to runs started within this many recent minutes
      --json                           Output as a single JSON object (`{by_skill}`)
  -h, --help                           Print help
```

##### `argo forge runs by-status`

```text
Histogram of forge_run rows grouped by `status`. Mirrors the gateway's `GET /forge/runs/by-status` (#584)

Usage: argo forge runs by-status [OPTIONS]

Options:
      --since-minutes <SINCE_MINUTES>  Filter to runs started within this many recent minutes
      --json                           Output as a single JSON object (`{by_status}`)
  -h, --help                           Print help
```

##### `argo forge runs count`

```text
Print just the count of recorded forge runs

Usage: argo forge runs count [OPTIONS]

Options:
      --status <STATUS>                Filter by status: running|succeeded|failed|aborted
      --since-minutes <SINCE_MINUTES>  Filter to runs started within this many recent minutes. Mirrors the gateway's `?since-minutes=N`
      --json                           Output as a single JSON object {count, status?}
  -h, --help                           Print help
```

##### `argo forge runs list`

```text
List recorded forge runs. Mirrors the gateway's `/forge/runs` endpoint with the same filter set

Usage: argo forge runs list [OPTIONS]

Options:
      --limit <LIMIT>
          [default: 50]
      --json
          Output as JSON (one object per row). Equivalent to `--format json`
      --format <FORMAT>
          Output format: `text` (default), `json`, or `markdown` (renders a markdown table). `--json` wins if both are passed [default: text]
      --since-minutes <SINCE_MINUTES>
          Filter to runs started within this many recent minutes
      --status <STATUS>
          Status filter: running|succeeded|failed|aborted
      --score-min <SCORE_MIN>
          Inclusive lower bound on `final_score`
      --score-max <SCORE_MAX>
          Inclusive upper bound on `final_score`
      --skill <SKILL>
          Exact-match filter on skill_name
      --has-candidate <HAS_CANDIDATE>
          `true` to keep only rows with a non-NULL candidate_version [possible values: true, false]
      --iterations-min <ITERATIONS_MIN>
          Inclusive lower bound on `iterations`. Mirrors the gateway's `?iterations-min=N`
      --iterations-max <ITERATIONS_MAX>
          Inclusive upper bound on `iterations`. Mirrors the gateway's `?iterations-max=N`
      --sort-by <SORT_BY>
          Sort order: `started-at` (default, descending), `score` (final_score descending; NULLs sink to the bottom), or `spend` (spend_usd_micros descending) [default: started-at]
  -h, --help
          Print help
```

##### `argo forge runs payload`

```text
Show the queued payload (skill / examples / iterations / budget_usd) for a forge run. Mirrors the gateway's `GET /forge/runs/<id>/payload`. 404 when the row has no forge_queue entry (e.g. ran via `?in_process=true`)

Usage: argo forge runs payload [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --json  Output as a single JSON object {run_id, payload}
  -h, --help  Print help
```

##### `argo forge runs requeue`

```text
Requeue a terminal forge run (failed / aborted / succeeded) by copying its queued payload into a fresh row with status='queued'. The daemon picks it up on its next tick. The source row stays as a historical record. 404 when the source has no payload (was run via `?in_process=true`). Mirrors the gateway's `POST /forge/runs/<id>/requeue`

Usage: argo forge runs requeue [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --json  Output as a single JSON object {requeued, source_id, run_id}
  -h, --help  Print help
```

##### `argo forge runs show`

```text
Usage: argo forge runs show <ID>

Arguments:
  <ID>  

Options:
  -h, --help  Print help
```

##### `argo forge runs sweep`

```text
Sweep stalled `running` rows whose `started_at` is older than `--older-than-minutes` ago (default 60). Used to recover from a gateway crash mid-optimize: in-memory state is gone, but the row stays in `running` until manually aborted. Flips status to `failed`. Mirrors `POST /forge/runs/sweep`

Usage: argo forge runs sweep [OPTIONS]

Options:
      --older-than-minutes <OLDER_THAN_MINUTES>
          Cutoff in minutes. Default 60 — runs older than this that are still in `running` get flipped to `failed` [default: 60]
  -h, --help
          Print help
```

### `argo gateway`

```text
Usage: argo gateway <COMMAND>

Commands:
  start   Bind a TCP listener and serve the JSON HTTP gateway. The first cut answers `GET /health` + `GET /version`; the `Adapter` routing surface from `argo-gateway` lands later. Browser-facing surfaces mounted ahead of the bearer gate: `/dashboard/*` (operator dashboard) and `/marketplace/*` (skill marketplace MVP — browse + install cross-tenant skills)
  stop    
  status  One-line status: running PID + listening address
  routes  Print the gateway's static route catalog. No daemon required — reads the in-binary catalog. `--method GET|POST` filters by verb; `--grep <sub>` filters by path/description substring (case-insensitive); `--limit N` truncates to the first N rows; `--json` for the machine-readable form
  audit   Inspect per-HTTP-request gateway audit rows
  probe   Hit one of the K8s-style gateway probe endpoints (#630) and return exit 0 on success, 1 on failure. Useful for shell scripts / CI gates / boot-up health checks that want a binary signal without parsing `argo doctor`. Default kind is `ready` (DB-aware); pass `live` for the minimal liveness probe
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo gateway audit`

```text
Inspect per-HTTP-request gateway audit rows

Usage: argo gateway audit <COMMAND>

Commands:
  list  List request audit rows newest-first
  show  Show one request audit row by request id
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo gateway audit list`

```text
List request audit rows newest-first

Usage: argo gateway audit list [OPTIONS]

Options:
      --since <SINCE>          Include rows at or after this time (`1h ago`, `30m`, or RFC3339)
      --principal <PRINCIPAL>  Filter by exact principal id
      --status <STATUS>        Filter by exact HTTP status code
      --json                   Output a JSON envelope
      --limit <LIMIT>          Maximum rows to return (1..1000) [default: 100]
  -h, --help                   Print help
```

##### `argo gateway audit show`

```text
Show one request audit row by request id

Usage: argo gateway audit show [OPTIONS] <REQUEST_ID>

Arguments:
  <REQUEST_ID>  UUID request id

Options:
      --json  Output as JSON
  -h, --help  Print help
```

#### `argo gateway probe`

```text
Hit one of the K8s-style gateway probe endpoints (#630) and return exit 0 on success, 1 on failure. Useful for shell scripts / CI gates / boot-up health checks that want a binary signal without parsing `argo doctor`. Default kind is `ready` (DB-aware); pass `live` for the minimal liveness probe

Usage: argo gateway probe [OPTIONS] [KIND]

Arguments:
  [KIND]  Probe kind: `live` (cheap liveness) or `ready` (verifies the SQLite store responds) [default: ready]

Options:
      --host <HOST>  Bind host (default 127.0.0.1) [default: 127.0.0.1]
      --port <PORT>  Bind port (default 7400) [default: 7400]
      --json         Output as a JSON object instead of human text
  -h, --help         Print help
```

#### `argo gateway routes`

```text
Print the gateway's static route catalog. No daemon required — reads the in-binary catalog. `--method GET|POST` filters by verb; `--grep <sub>` filters by path/description substring (case-insensitive); `--limit N` truncates to the first N rows; `--json` for the machine-readable form

Usage: argo gateway routes [OPTIONS]

Options:
      --json             Output as JSON. Equivalent to `--format json`
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown` (renders the catalog as a markdown table — paste-able into READMEs). `--json` wins if both are passed [default: text]
      --method <METHOD>  
      --grep <GREP>      Case-insensitive substring filter on both path and description
      --limit <LIMIT>    Truncate the output to the first N rows after filters
  -h, --help             Print help
```

#### `argo gateway start`

```text
Bind a TCP listener and serve the JSON HTTP gateway. The first cut answers `GET /health` + `GET /version`; the `Adapter` routing surface from `argo-gateway` lands later. Browser-facing surfaces mounted ahead of the bearer gate: `/dashboard/*` (operator dashboard) and `/marketplace/*` (skill marketplace MVP — browse + install cross-tenant skills)

Usage: argo gateway start [OPTIONS]

Options:
      --bind <BIND>                   [default: 127.0.0.1]
      --port <PORT>                   [default: 7400]
      --max-requests <MAX_REQUESTS>   Stop after N requests (mainly for tests; omit to run until SIGINT / SIGTERM)
      --quiet                         Suppress the boot-time route catalog banner. Top of the precedence chain — overrides `ARGO_BANNER_*` env vars and any `[startup_banner]` TOML section
      --bind-non-loopback             Opt in to binding a non-loopback address for dev/recovery
      --accept-insecure-bind-warning  Required with `--bind-non-loopback`; records operator acceptance of the plaintext bind warning
      --insecure-bind                 Deprecated old escape hatch. Use `--bind-non-loopback` plus `--accept-insecure-bind-warning` instead
  -h, --help                          Print help
```

#### `argo gateway status`

```text
One-line status: running PID + listening address

Usage: argo gateway status [OPTIONS]

Options:
      --json  Output as a JSON object `{running, pid?, addr?}`
  -h, --help  Print help
```

#### `argo gateway stop`

```text
Usage: argo gateway stop

Options:
  -h, --help  Print help
```

### `argo genesis`

```text
Charter → vertical generator (#G-genesis). Operator describes a role in plain English; Genesis emits a complete `verticals/<id>/` tree with safety judge guarantees on every action

Usage: argo genesis <COMMAND>

Commands:
  list-archetypes       List the built-in Genesis archetypes operators can start from
  show-archetype        Show one archetype's action vocabulary, persona traits, and channels
  suggest               Preview the top archetype matches for a charter without writing files
  create                Generate a vertical from a plain-English charter. Runs the archetype classifier, assembles the manifest, auto-fixes sensitive actions to require approval, runs the safety judge, then writes the complete `verticals/<id>/` tree to `--output`
  refine                Re-run the Genesis LLM refinement pass against an existing vertical
  interactive           Walk the operator through a 6-prompt Q&A then generate a vertical from the answers. Stream `claude-genesis-charter-conversation`. Use `--non-interactive <yaml>` to feed pre-recorded answers (CI-friendly) and `--refine` to also run the LLM refinement pass. The final vertical lands at `--output`, the same as `argo genesis create`
  import                Import an existing prompt source (LangChain `*.py`, markdown, or raw text) into a minimal vertical scaffold. Genesis revise + the action/eval/memory generators can then flesh the scaffold out. v0 reads local paths only — URL fetch needs a security review and lands in a follow-up
  revise                Surgically revise an already-generated vertical from operator feedback. Updates one or more of the system prompt, action vocabulary, and eval-seed list while preserving anything wrapped in `[KEEP]…[/KEEP]` markers. See `docs/build-plan/streams/claude-genesis-iteration-cycle.md`
  from-job-description  Ingest a job description (PDF path, plain text, or http(s) URL), synthesise a charter, and run the Genesis pipeline against it. Mission `mission-genesis-job-description-ingest` (#64)
  from-vertical         Seed Genesis from an existing sibling vertical. Operator supplies a charter file + the `--seed <vertical-id>` of an already-installed in-tenant vertical; Genesis runs the standard archetype pipeline, then merges the seed's `[actions]`, `[skills]`, `[workflows]` shape into the baseline. The seed's id is stamped onto the resulting manifest's `vertical.tags` as `"seeded-from:<id>"` so downstream eval / Forge cohort analysis can group skills by seed lineage. Mission `mission-genesis-transfer-from-tenant`
  export                Export an installed vertical as a portable tar.gz bundle
  help                  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo genesis create`

```text
Generate a vertical from a plain-English charter. Runs the archetype classifier, assembles the manifest, auto-fixes sensitive actions to require approval, runs the safety judge, then writes the complete `verticals/<id>/` tree to `--output`

Usage: argo genesis create [OPTIONS] --charter <CHARTER> --persona <PERSONA> --output <OUTPUT>

Options:
      --charter <CHARTER>  Plain-English description of the role. Required. Example: `"be my marketing intern, run weekly campaigns to cold leads"`
      --persona <PERSONA>  Public-facing persona name (`maya`, `sam`, …). Used in the worker_id + the persona prompt's signoff. Must contain only `[a-zA-Z0-9_-]`
      --output <OUTPUT>    Output directory. Genesis refuses to clobber a non-empty dir — use a fresh path
      --json               Emit a JSON summary instead of the human-readable post-create report
      --refine             Stream `claude-genesis-llm-refinement` — opt-in LLM refinement pass on top of the deterministic pipeline. Requires `ANTHROPIC_API_KEY`. The safety judge re-runs on every action the model proposes; refinement failures fall back silently to the deterministic output with a `tracing::warn!`
      --allow-broad-role   Mission `mission-genesis-non-archetype-roles`. When set, charters whose LLM-classifier confidence falls below the 0.6 broad-role threshold route to LLM-driven broader generation instead of forcing the closest archetype. Without this flag (the default), low-confidence charters surface an explicit error so cautious operators don't silently switch generation paths
  -h, --help               Print help
```

#### `argo genesis export`

```text
Export an installed vertical as a portable tar.gz bundle

Usage: argo genesis export [OPTIONS] --vertical <VERTICAL>

Options:
      --vertical <VERTICAL>
          Vertical id discoverable via ARGO_VERTICALS_DIR or ./verticals
      --output <OUTPUT>
          Output path. Defaults to `<vertical>.argo-vertical.tar.gz`
      --include-skills <INCLUDE_SKILLS>
          Include `skills/` from the vertical root when present [default: true] [possible values: true, false]
      --manifest-only
          Write only BUNDLE.toml instead of a tarball
      --force
          Overwrite an existing output file
  -h, --help
          Print help
```

#### `argo genesis from-job-description`

```text
Ingest a job description (PDF path, plain text, or http(s) URL), synthesise a charter, and run the Genesis pipeline against it. Mission `mission-genesis-job-description-ingest` (#64).

Input is a single positional `<input>` interpreted in this order: 1. `http://` or `https://` prefix → URL fetch (30s timeout, 5MB cap). 2. Path that exists and ends in `.pdf` → PDF text extraction. 3. Path that exists → read as plain text. 4. Anything else → treated as a raw text body.

Usage: argo genesis from-job-description [OPTIONS] <INPUT>

Arguments:
  <INPUT>
          Path-or-URL-or-raw-text body. See command docs above

Options:
      --json
          Emit the full `GenesisDraft` as JSON instead of a human readable summary

  -h, --help
          Print help (see a summary with '-h')
```

#### `argo genesis from-vertical`

```text
Seed Genesis from an existing sibling vertical. Operator supplies a charter file + the `--seed <vertical-id>` of an already-installed in-tenant vertical; Genesis runs the standard archetype pipeline, then merges the seed's `[actions]`, `[skills]`, `[workflows]` shape into the baseline. The seed's id is stamped onto the resulting manifest's `vertical.tags` as `"seeded-from:<id>"` so downstream eval / Forge cohort analysis can group skills by seed lineage. Mission `mission-genesis-transfer-from-tenant`.

Without `--seed`, prefer `argo genesis create` — this subcommand ALWAYS seeds.

Usage: argo genesis from-vertical [OPTIONS] --seed <SEED> --persona <PERSONA> --output <OUTPUT> <CHARTER_FILE>

Arguments:
  <CHARTER_FILE>
          Path to a plain-text file containing the role charter. Read as UTF-8; first 4 KiB is treated as the charter

Options:
      --seed <SEED>
          Vertical id to seed from (must be discoverable under `$ARGO_VERTICALS_DIR` or `./verticals`)

      --persona <PERSONA>
          Public-facing persona name. Same shape as `create`

      --output <OUTPUT>
          Output directory for the generated vertical. Genesis refuses to clobber a non-empty dir

      --json
          Emit a JSON summary instead of the human-readable post-create report

  -h, --help
          Print help (see a summary with '-h')
```

#### `argo genesis import`

```text
Import an existing prompt source (LangChain `*.py`, markdown, or raw text) into a minimal vertical scaffold. Genesis revise + the action/eval/memory generators can then flesh the scaffold out. v0 reads local paths only — URL fetch needs a security review and lands in a follow-up

Usage: argo genesis import [OPTIONS] --source <SOURCE> --name <NAME>

Options:
      --source <SOURCE>  Local path to the prompt source. Required
      --name <NAME>      Vertical id for the new scaffold (lowercase ASCII + digits + `-`, same shape as `vertical.id`)
      --format <FORMAT>  Source format. `auto` (default) sniffs by extension: `*.py` → langchain, `*.md` → markdown, anything else → raw [default: auto]
      --output <OUTPUT>  Parent directory for the scaffold. The vertical lands at `<output>/<name>/` [default: verticals]
      --json             Emit a single JSON envelope summarising the import instead of human-readable lines
  -h, --help             Print help
```

#### `argo genesis interactive`

```text
Walk the operator through a 6-prompt Q&A then generate a vertical from the answers. Stream `claude-genesis-charter-conversation`. Use `--non-interactive <yaml>` to feed pre-recorded answers (CI-friendly) and `--refine` to also run the LLM refinement pass. The final vertical lands at `--output`, the same as `argo genesis create`

Usage: argo genesis interactive [OPTIONS] --output <OUTPUT>

Options:
      --output <OUTPUT>         Output directory for the generated vertical. Genesis refuses to clobber a non-empty dir
      --non-interactive <YAML>  Skip stdin prompting; load answers from a YAML file shaped like `CharterDialogue` (see `argo-genesis::charter_dialogue`)
      --json                    Emit a single JSON envelope summarising the dialogue and resulting vertical instead of the human-readable post-create report
      --refine                  Run the LLM refinement pass on the rendered charter. Same contract as `argo genesis create --refine`: requires `ANTHROPIC_API_KEY` and falls back silently to the deterministic plan on transient API failures
  -h, --help                    Print help
```

#### `argo genesis list-archetypes`

```text
List the built-in Genesis archetypes operators can start from

Usage: argo genesis list-archetypes [OPTIONS]

Options:
      --json  Emit a single JSON object instead of a table
  -h, --help  Print help
```

#### `argo genesis refine`

```text
Re-run the Genesis LLM refinement pass against an existing vertical

Usage: argo genesis refine [OPTIONS] <VERTICAL_ID>

Arguments:
  <VERTICAL_ID>  Existing vertical id discoverable via ARGO_VERTICALS_DIR or ./verticals

Options:
      --judge
          Explicitly run the safety judge after each refinement iteration
      --no-judge
          Skip the safety judge re-run after refinement
      --max-iterations <MAX_ITERATIONS>
          Maximum LLM refinement iterations to run [default: 1]
      --dry-run
          Print the manifest diff without writing files
      --json
          Emit a JSON envelope
      --show-vocab-evolution
          After the refinement pass, print the list of action_types newly added to the manifest's vocabulary during this run (#60 mission load-bearing wire). Empty list when no vocabulary grew
  -h, --help
          Print help
```

#### `argo genesis revise`

```text
Surgically revise an already-generated vertical from operator feedback. Updates one or more of the system prompt, action vocabulary, and eval-seed list while preserving anything wrapped in `[KEEP]…[/KEEP]` markers. See `docs/build-plan/streams/claude-genesis-iteration-cycle.md`

Usage: argo genesis revise [OPTIONS] --vertical <VERTICAL> --feedback <FEEDBACK>

Options:
      --vertical <VERTICAL>  Vertical id to revise (directory name under the verticals root)
      --feedback <FEEDBACK>  Plain-English operator feedback (`"be more terse"`, `"drop the social-media actions"`, …)
      --target <TARGET>      Which artefact(s) to update: `prompt`, `actions`, `eval`, or `all` (default) [default: all]
      --dry-run              Print the diff and exit without writing to disk
      --json                 Emit a JSON envelope instead of human-readable lines
  -h, --help                 Print help
```

#### `argo genesis show-archetype`

```text
Show one archetype's action vocabulary, persona traits, and channels

Usage: argo genesis show-archetype [OPTIONS] <NAME>

Arguments:
  <NAME>  Archetype label or slug, e.g. `Marketing Intern` or `marketing_intern`

Options:
      --json  Emit a single JSON object instead of text
  -h, --help  Print help
```

#### `argo genesis suggest`

```text
Preview the top archetype matches for a charter without writing files

Usage: argo genesis suggest [OPTIONS] --charter <CHARTER>

Options:
      --charter <CHARTER>  Plain-English role charter to classify
      --json               Emit a single JSON object instead of a table
  -h, --help               Print help
```

### `argo handoff`

```text
Usage: argo handoff <COMMAND>

Commands:
  list    
  show    
  cancel  
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo handoff cancel`

```text
Usage: argo handoff cancel [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --tenant <TENANT>  
      --note <NOTE>      
      --json             
  -h, --help             Print help
```

#### `argo handoff list`

```text
Usage: argo handoff list [OPTIONS]

Options:
      --tenant <TENANT>    
      --session <SESSION>  
      --status <STATUS>    
      --worker <WORKER>    
      --limit <LIMIT>      [default: 25]
      --json               
  -h, --help               Print help
```

#### `argo handoff show`

```text
Usage: argo handoff show [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --tenant <TENANT>  
      --json             
  -h, --help             Print help
```

### `argo handoff-auctions`

```text
Inspect handoff fallback auctions (`claude-skill-auction`). When the routing chain exhausts, the walker may record a `no_bidders` auction row in the audit trail; this surface lets operators read + cancel those

Usage: argo handoff-auctions <COMMAND>

Commands:
  list    List recent auctions, newest-first. Optional `--status` filter (`open` / `closed` / `cancelled` / `no_bidders`)
  show    Show one auction by id with all its bids
  cancel  Cancel an open auction. No-op on closed / cancelled auctions
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo handoff-auctions cancel`

```text
Cancel an open auction. No-op on closed / cancelled auctions

Usage: argo handoff-auctions cancel [OPTIONS] <ID>

Arguments:
  <ID>  Auction id

Options:
      --json  Emit JSON
  -h, --help  Print help
```

#### `argo handoff-auctions list`

```text
List recent auctions, newest-first. Optional `--status` filter (`open` / `closed` / `cancelled` / `no_bidders`)

Usage: argo handoff-auctions list [OPTIONS]

Options:
      --status <STATUS>  Filter by status string
      --limit <LIMIT>    Cap the number of rows printed. Default 100 [default: 100]
      --json             Emit JSON
  -h, --help             Print help
```

#### `argo handoff-auctions show`

```text
Show one auction by id with all its bids

Usage: argo handoff-auctions show [OPTIONS] <ID>

Arguments:
  <ID>  Auction id

Options:
      --json  Emit JSON
  -h, --help  Print help
```

### `argo handoff-policy`

```text
Workspace-level handoff policy (`claude-worker-handoff-policy`). Reads `.skycot/handoff-policy.toml` and answers "should we hand off at all?" via a first-match-wins rule list

Usage: argo handoff-policy <COMMAND>

Commands:
  show      Print the parsed `.skycot/handoff-policy.toml`. With `--explain <action>`, also trace which rule (if any) fires for that action_kind
  validate  Schema-check a handoff-policy file. Defaults to `.skycot/handoff-policy.toml` under the current dir; pass a path to validate a different file
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo handoff-policy show`

```text
Print the parsed `.skycot/handoff-policy.toml`. With `--explain <action>`, also trace which rule (if any) fires for that action_kind

Usage: argo handoff-policy show [OPTIONS]

Options:
      --explain <action>  Trace the matched rule for this action kind
      --json              Emit JSON
  -h, --help              Print help
```

#### `argo handoff-policy validate`

```text
Schema-check a handoff-policy file. Defaults to `.skycot/handoff-policy.toml` under the current dir; pass a path to validate a different file

Usage: argo handoff-policy validate [OPTIONS] [PATH]

Arguments:
  [PATH]  Path to the policy file (defaults to `.skycot/handoff-policy.toml`)

Options:
      --json  Emit JSON
  -h, --help  Print help
```

### `argo handoff-rules`

```text
Declarative handoff routing rules (`claude-handoff-routing-rules`)

Usage: argo handoff-rules <COMMAND>

Commands:
  list    List rules for a tenant in priority order
  show    Show one rule (uuid OR name lookup)
  add     Add a rule from a JSON file describing the DSL
  remove  Remove a rule (uuid OR name)
  test    Simulate rule application against a hypothetical handoff without persisting anything
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo handoff-rules add`

```text
Add a rule from a JSON file describing the DSL

Usage: argo handoff-rules add [OPTIONS] --tenant <TENANT> <PATH>

Arguments:
  <PATH>  Path to JSON describing the rule (name + when + then + priority + active)

Options:
      --tenant <TENANT>  Tenant id
      --json             Emit JSON instead of the human-friendly recap
  -h, --help             Print help
```

#### `argo handoff-rules list`

```text
List rules for a tenant in priority order

Usage: argo handoff-rules list [OPTIONS] --tenant <TENANT>

Options:
      --tenant <TENANT>  Tenant id
      --json             Emit JSON
  -h, --help             Print help
```

#### `argo handoff-rules remove`

```text
Remove a rule (uuid OR name)

Usage: argo handoff-rules remove [OPTIONS] --tenant <TENANT> <RULE>

Arguments:
  <RULE>  Rule uuid OR name

Options:
      --tenant <TENANT>  Tenant id
      --json             Emit JSON
  -h, --help             Print help
```

#### `argo handoff-rules show`

```text
Show one rule (uuid OR name lookup)

Usage: argo handoff-rules show [OPTIONS] --tenant <TENANT> <RULE>

Arguments:
  <RULE>  Rule uuid OR name

Options:
      --tenant <TENANT>  Tenant id
      --json             Emit JSON
  -h, --help             Print help
```

#### `argo handoff-rules test`

```text
Simulate rule application against a hypothetical handoff without persisting anything

Usage: argo handoff-rules test [OPTIONS] --tenant <TENANT> --expected-outcome <EXPECTED_OUTCOME>

Options:
      --tenant <TENANT>
          Tenant id whose rules should be loaded
      --expected-outcome <EXPECTED_OUTCOME>
          Hypothetical `expected_outcome` text
      --hour <HOUR>
          Hypothetical hour-of-day UTC (`0..24`). Defaults to the current hour
      --json
          Emit JSON
  -h, --help
          Print help
```

### `argo handoff-stats`

```text
Aggregate metrics over the orchestrator's `handoff` table (Stream `codex-handoff-stats-cli`). Markdown report by default; `--histogram` switches to an ASCII volume readout

Usage: argo handoff-stats [OPTIONS]

Options:
      --tenant <TENANT>            Tenant uuid. Falls back to `ARGO_DEFAULT_TENANT`
      --since <SINCE>              Window size as `<n><unit>` (`s`/`m`/`h`/`d`). Default `7d` for the report mode and `1d` for `--histogram`
      --by <BY>                    Group axis for the breakdown table: `worker` (handed-off-FROM, the default), `skill` (groups by `expected_outcome` since handoff rows do not carry a typed skill column), or `destination` (handed-off-TO, projected to a worker id or `human:<addr>` label)
      --histogram                  Switch to ASCII volume histogram mode
      --bucket-mins <BUCKET_MINS>  Bucket size in minutes for `--histogram`. Default 60 (a 24-bucket day at the default `--since 1d`). Clamped to `>= 1` [default: 60]
      --json                       Emit a JSON envelope instead of the markdown rendering
  -h, --help                       Print help
```

### `argo health`

```text
Run every read-only verification — workspace, provider manifests, skill hashes, memory hashes, audit chain — and exit non-zero on the first failure. Suitable as a single CI hook

Usage: argo health [OPTIONS]

Options:
      --json             Output a single JSON object with per-check status. Equivalent to `--format json`
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
  -h, --help             Print help

Examples:
  argo health
  argo health --json
```

### `argo import`

```text
Usage: argo import <COMMAND>

Commands:
  hermes  
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo import hermes`

```text
Usage: argo import hermes <PATH>

Arguments:
  <PATH>  

Options:
  -h, --help  Print help
```

### `argo incident`

```text
Walk the operator through a blameless postmortem (Stream `claude-incident-postmortem-template`). Reads stdin section by section, or `--from <yaml>` for non-interactive scripts; writes `docs/skycot/incidents/<YYYY-MM-DD>-<slug>.md`

Usage: argo incident <COMMAND>

Commands:
  new     Walk the operator through a blameless postmortem and write a dated, slugged file under `docs/skycot/incidents/`. Reads a YAML fixture with `--from <path>` for non-interactive use (cron / scripts); otherwise prompts per section over stdin
  bundle  Snapshot diagnostic state into a `tar.gz` for on-call sharing (Stream `mission-incident-response-runbook`). Captures `argo doctor`, audit tail, worker restarts, gateway metrics, audit archives, build identity, redacted `argo.toml`, and a log tail. Read-only on system state
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo incident bundle`

```text
Snapshot diagnostic state into a `tar.gz` for on-call sharing (Stream `mission-incident-response-runbook`). Captures `argo doctor`, audit tail, worker restarts, gateway metrics, audit archives, build identity, redacted `argo.toml`, and a log tail. Read-only on system state

Usage: argo incident bundle [OPTIONS]

Options:
      --since <SINCE>        Time window for audit / restart / log captures (e.g. `1h`, `30m`). Default `1h` [default: 1h]
      --output <OUTPUT>      Output path. Default `incident-<utc-ts>.tar.gz` in cwd
      --log-tail <LOG_TAIL>  Path to recent stderr log tail; without it the section records "no log path provided"
  -h, --help                 Print help
```

#### `argo incident new`

```text
Walk the operator through a blameless postmortem and write a dated, slugged file under `docs/skycot/incidents/`. Reads a YAML fixture with `--from <path>` for non-interactive use (cron / scripts); otherwise prompts per section over stdin

Usage: argo incident new [OPTIONS]

Options:
      --from <FROM>        YAML fixture (matches `_template_example.yaml`)
      --out-dir <OUT_DIR>  Override the default `docs/skycot/incidents/` output dir [default: docs/skycot/incidents]
      --force              Overwrite an existing postmortem at the same path
  -h, --help               Print help
```

### `argo judge`

```text
Usage: argo judge <COMMAND>

Commands:
  verdict            
  override           
  eval-disagreement  Run the judge-vs-human disagreement calibration suite over a directory of pre-labelled fixtures. Reports agreement rate overall and per tag bucket. With `--bail-below <rate>`, exits non-zero when the overall rate falls below the threshold — suitable for CI gating on judge-model upgrades. The default threshold of 0.85 is a starting point; tune per-tenant. The judge transport is built with `temperature = 0` for reproducibility — see `argo-judge::eval_suite` for details
  help               Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo judge eval-disagreement`

```text
Run the judge-vs-human disagreement calibration suite over a directory of pre-labelled fixtures. Reports agreement rate overall and per tag bucket. With `--bail-below <rate>`, exits non-zero when the overall rate falls below the threshold — suitable for CI gating on judge-model upgrades. The default threshold of 0.85 is a starting point; tune per-tenant. The judge transport is built with `temperature = 0` for reproducibility — see `argo-judge::eval_suite` for details

Usage: argo judge eval-disagreement [OPTIONS]

Options:
      --fixtures <FIXTURES>      Directory of `*.json` fixtures. Defaults to the bundled `crates/argo-judge/fixtures/disagreement/` directory
      --provider <PROVIDER>      Provider manifest id used to construct the judge transport [default: anthropic]
      --model <MODEL>            Judge model id (e.g. `claude-sonnet-4-6`) [default: claude-sonnet-4-6]
      --json                     Emit a structured JSON envelope instead of a text summary
      --bail-below <BAIL_BELOW>  Exit non-zero if the overall agreement rate is strictly below this value. Range `[0.0, 1.0]`. Default `0.85` [default: 0.85]
  -h, --help                     Print help
```

#### `argo judge override`

```text
Usage: argo judge override --to <TO> <ID>

Arguments:
  <ID>  

Options:
      --to <TO>  
  -h, --help     Print help
```

#### `argo judge verdict`

```text
Usage: argo judge verdict <ID>

Arguments:
  <ID>  

Options:
  -h, --help  Print help
```

### `argo latency`

```text
Show per-executor-kind action latency percentiles (Stream `claude-latency-percentiles-tracking`). Reads the `latency_buckets` table populated by the dispatch wrapper and prints `kind | n | p50 | p95 | p99 | max` as a markdown table (default) or as JSON

Usage: argo latency [OPTIONS]

Options:
      --format <FORMAT>  Output format: `markdown` (default) or `json` [default: markdown]
  -h, --help             Print help

Examples:
  argo latency
  argo latency --format json
```

### `argo mcp`

```text
Usage: argo mcp <COMMAND>

Commands:
  serve      Serve the workspace store as an MCP server over stdio
  tools      Discover the tools an MCP server exposes (`claude-mcp-tool-discovery-vis`). Connects to the server over stdio using the manifest declaration in the named vertical, calls `tools/list`, and renders the result
  registry   Manage the per-tenant MCP server registry (`codex-mcp-server-registry-cli`). Backs the dispatch path when `ARGO_MCP_REGISTRY_BACKED=1` is set
  image-gen  Boot the `argo-mcp-image-gen` MCP server over stdio (`mission-image-generation-mcp-server`). Reads `[mcp.image_gen]` from `--config`, dispatches `image.generate` + `image.edit` to OpenAI-compatible or stub backends, writes a `cost_estimate` audit row before each call
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo mcp image-gen`

```text
Boot the `argo-mcp-image-gen` MCP server over stdio (`mission-image-generation-mcp-server`). Reads `[mcp.image_gen]` from `--config`, dispatches `image.generate` + `image.edit` to OpenAI-compatible or stub backends, writes a `cost_estimate` audit row before each call

Usage: argo mcp image-gen [OPTIONS]

Options:
      --config <CONFIG>  Path to `argo.toml`. Defaults to `./argo.toml` [default: argo.toml]
      --require-confirm  Short-circuit calls with a cost estimate ≥ 20¢
  -h, --help             Print help
```

#### `argo mcp registry`

```text
Manage the per-tenant MCP server registry (`codex-mcp-server-registry-cli`). Backs the dispatch path when `ARGO_MCP_REGISTRY_BACKED=1` is set

Usage: argo mcp registry <COMMAND>

Commands:
  list    List registered MCP servers for a tenant
  add     Register one MCP server for a tenant
  remove  Remove one MCP server registration
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo mcp registry add`

```text
Register one MCP server for a tenant

Usage: argo mcp registry add [OPTIONS] --kind <KIND> --url <BASE_URL> <SERVER_ID>

Arguments:
  <SERVER_ID>  Server id referenced by MCP-executor actions

Options:
      --kind <KIND>                Server kind or transport family (e.g. `stdio`, `http`)
      --url <BASE_URL>             Base URL for the MCP server
      --secrets-ref <SECRETS_REF>  Opaque reference into the workspace credential store
      --tenant <TENANT>            Tenant id. Defaults to the current workspace tenant
      --json                       Emit a JSON envelope
  -h, --help                       Print help
```

##### `argo mcp registry list`

```text
List registered MCP servers for a tenant

Usage: argo mcp registry list [OPTIONS]

Options:
      --tenant <TENANT>  Tenant id. Defaults to the current workspace tenant
      --format <FORMAT>  Output format: `markdown` (default) or `json` (NDJSON) [default: markdown]
      --json             Emit NDJSON; equivalent to `--format json`
  -h, --help             Print help
```

##### `argo mcp registry remove`

```text
Remove one MCP server registration

Usage: argo mcp registry remove [OPTIONS] <SERVER_ID>

Arguments:
  <SERVER_ID>  Server id to remove

Options:
      --tenant <TENANT>  Tenant id. Defaults to the current workspace tenant
      --force            Remove even when active actions reference this server
      --json             Emit a JSON envelope
  -h, --help             Print help
```

#### `argo mcp serve`

```text
Serve the workspace store as an MCP server over stdio

Usage: argo mcp serve

Options:
  -h, --help  Print help
```

#### `argo mcp tools`

```text
Discover the tools an MCP server exposes (`claude-mcp-tool-discovery-vis`). Connects to the server over stdio using the manifest declaration in the named vertical, calls `tools/list`, and renders the result

Usage: argo mcp tools [OPTIONS] --vertical <VERTICAL> <SERVER_ID>

Arguments:
  <SERVER_ID>  Server id as declared in the vertical manifest's `[mcp_servers]` section

Options:
      --vertical <VERTICAL>  Vertical id whose manifest declares the server
      --format <FORMAT>      Output format: `markdown` (default) or `json` (NDJSON, one tool per line) [default: markdown]
      --json                 Equivalent to `--format json`
      --filter <FILTER>      Regex narrowing the tool list by name
      --schema-only          Omit descriptions for terse output
  -h, --help                 Print help
```

### `argo memory`

```text
Usage: argo memory <COMMAND>

Commands:
  shared-team  Inspect and seed tenant-scoped shared-team memory
  list         
  view         
  append       Append a fresh memory entry. `<file>` is one of memory|user|soul|agents. Pass `--vertical <id>` (#680, Rule 7) to tag the entry with a vertical namespace; vertical-aware reads via `--vertical` will then surface it. Pass `--session <uuid>` (#697) to attribute the entry to a specific session — required when the entry will later be promoted via `argo skill review promote`, since the promote audit row attributes to that session id
  count        Print just the count of active memory entries. Mirrors `argo session count`. Default: one int per line; `--json` emits `{count, file}`
  verify       Verify a memory entry's body matches its stored SHA-256 hash. Pass `--all` to verify every active entry across all four memory files in the workspace
  edit         
  revert       
  forget       
  lock         
  unlock       
  dream-now    Run a Dreams API summary now for one worker. Walks the previous calendar month's recent rows, calls the Dreams API, and persists the curated summary as a memory row tagged `source=dreams_api`. Refuses if the worker has any open anomaly events. Honours `ARGO_DREAMS_DAILY_CAP_USD_MICROS`. Available only when `argo-memory/dreams-backend` is enabled in the build (V2.0 Integration A)
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo memory append`

```text
Append a fresh memory entry. `<file>` is one of memory|user|soul|agents. Pass `--vertical <id>` (#680, Rule 7) to tag the entry with a vertical namespace; vertical-aware reads via `--vertical` will then surface it. Pass `--session <uuid>` (#697) to attribute the entry to a specific session — required when the entry will later be promoted via `argo skill review promote`, since the promote audit row attributes to that session id

Usage: argo memory append [OPTIONS] <FILE> <BODY>

Arguments:
  <FILE>  
  <BODY>  

Options:
      --vertical <VERTICAL>  
      --session <SESSION>    
  -h, --help                 Print help
```

#### `argo memory count`

```text
Print just the count of active memory entries. Mirrors `argo session count`. Default: one int per line; `--json` emits `{count, file}`

Usage: argo memory count [OPTIONS]

Options:
      --file <FILE>                    Limit to one file: memory|user|soul|agents (default counts across all four)
      --since-minutes <SINCE_MINUTES>  Filter to entries learned within this many recent minutes. Mirrors the gateway's `?since-minutes=N`
      --json                           Output as a single JSON object
  -h, --help                           Print help
```

#### `argo memory dream-now`

```text
Run a Dreams API summary now for one worker. Walks the previous calendar month's recent rows, calls the Dreams API, and persists the curated summary as a memory row tagged `source=dreams_api`. Refuses if the worker has any open anomaly events. Honours `ARGO_DREAMS_DAILY_CAP_USD_MICROS`. Available only when `argo-memory/dreams-backend` is enabled in the build (V2.0 Integration A)

Usage: argo memory dream-now [OPTIONS] <WORKER>

Arguments:
  <WORKER>  Worker id to dream for (UUID)

Options:
      --scope <SCOPE>  Memory scope (worker_self|role|shared_team). Defaults to `worker_self` [default: worker_self]
      --json           Emit a single JSON object instead of human-readable text
  -h, --help           Print help
```

#### `argo memory edit`

```text
Usage: argo memory edit <ENTRY>

Arguments:
  <ENTRY>  

Options:
  -h, --help  Print help
```

#### `argo memory forget`

```text
Usage: argo memory forget <ENTRY>

Arguments:
  <ENTRY>  

Options:
  -h, --help  Print help
```

#### `argo memory list`

```text
Usage: argo memory list [OPTIONS]

Options:
      --json                           Output as JSON (one object per entry) for scripting. Equivalent to `--format json`
      --format <FORMAT>                Output format: `text` (default), `json`, or `markdown` (renders a markdown table). `--json` wins if both are passed [default: text]
      --file <FILE>                    Limit to one file: memory|user|soul|agents
      --sort-by <SORT_BY>              Sort order: `learned-at` (default, descending) or `size` (body byte length descending — biggest first) [default: learned-at]
      --limit <LIMIT>                  Truncate to the first N rows (after sorting). Useful when the corpus is large and you only want the top-N most-recent / biggest entries
      --since-minutes <SINCE_MINUTES>  Filter to entries learned within this many recent minutes. Mirrors the gateway's `?since-minutes=N`
      --lock <LOCK>                    Filter by lock status: `none` / `user-protected` / `content-locked`. Mirrors the gateway's `?lock=…`
      --body-contains <BODY_CONTAINS>  Case-insensitive substring filter on the entry body. Mirrors the gateway's `?body-contains=…`
      --vertical <VERTICAL>            Scope to a vertical's namespace (#680, Rule 7). Returns ONLY entries with `vertical_id = <id>` — excludes legacy workspace-global rows. Without this flag the legacy list-all behaviour is preserved
  -h, --help                           Print help
```

#### `argo memory lock`

```text
Usage: argo memory lock <ENTRY>

Arguments:
  <ENTRY>  

Options:
  -h, --help  Print help
```

#### `argo memory revert`

```text
Usage: argo memory revert <ENTRY>

Arguments:
  <ENTRY>  

Options:
  -h, --help  Print help
```

#### `argo memory shared-team`

```text
Inspect and seed tenant-scoped shared-team memory

Usage: argo memory shared-team <COMMAND>

Commands:
  list   List recent shared-team entries for one tenant
  write  Write an operator-seeded shared-team entry
  grep   Search shared-team entries with a case-insensitive regex
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo memory shared-team grep`

```text
Search shared-team entries with a case-insensitive regex

Usage: argo memory shared-team grep [OPTIONS] --tenant <TENANT> --pattern <PATTERN>

Options:
      --tenant <TENANT>    Tenant id (UUID)
      --pattern <PATTERN>  Regex pattern to match against entry content
      --json               Emit JSON instead of a human-readable table
  -h, --help               Print help
```

##### `argo memory shared-team list`

```text
List recent shared-team entries for one tenant

Usage: argo memory shared-team list [OPTIONS] --tenant <TENANT>

Options:
      --tenant <TENANT>  Tenant id (UUID)
      --limit <LIMIT>    Maximum number of newest rows to return
      --json             Emit JSON instead of a human-readable table
  -h, --help             Print help
```

##### `argo memory shared-team write`

```text
Write an operator-seeded shared-team entry

Usage: argo memory shared-team write [OPTIONS] --tenant <TENANT> --author <AUTHOR> --content <CONTENT>

Options:
      --tenant <TENANT>    Tenant id (UUID)
      --author <AUTHOR>    Worker id credited as the author
      --content <CONTENT>  Entry content
      --json               Emit JSON instead of a human-readable acknowledgement
  -h, --help               Print help
```

#### `argo memory unlock`

```text
Usage: argo memory unlock <ENTRY>

Arguments:
  <ENTRY>  

Options:
  -h, --help  Print help
```

#### `argo memory verify`

```text
Verify a memory entry's body matches its stored SHA-256 hash. Pass `--all` to verify every active entry across all four memory files in the workspace

Usage: argo memory verify [OPTIONS] [ENTRY]

Arguments:
  [ENTRY]  

Options:
      --all   Verify every active memory entry instead of one named entry
  -h, --help  Print help
```

#### `argo memory view`

```text
Usage: argo memory view [OPTIONS] <ENTRY>

Arguments:
  <ENTRY>  

Options:
      --json  Output the entry as a single JSON object (incl. body)
  -h, --help  Print help
```

### `argo messages`

```text
Workspace-wide message rollups. Mirrors the gateway's `/messages/*` family

Usage: argo messages <COMMAND>

Commands:
  by-day          Daily message rollup. Mirrors the gateway's `GET /messages/by-day`
  by-role         Histogram of messages grouped by role. Mirrors the gateway's `GET /messages/by-role` (#611)
  by-session      Top-N sessions by message row count. Mirrors the gateway's `GET /messages/by-session` (#615)
  by-day-by-role  2D rollup: messages bucketed by `(day, role)`. Mirrors the gateway's `GET /messages/by-day-by-role` (#640)
  recent          Workspace-wide recent message tail. Mirrors the gateway's `GET /messages/recent` (#619)
  help            Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo messages by-day`

```text
Daily message rollup. Mirrors the gateway's `GET /messages/by-day`

Usage: argo messages by-day [OPTIONS]

Options:
      --days <DAYS>  Lookback window in days (default 30, cap 365) [default: 30]
      --json         Output as a single JSON object (`{by_day}`)
  -h, --help         Print help
```

#### `argo messages by-day-by-role`

```text
2D rollup: messages bucketed by `(day, role)`. Mirrors the gateway's `GET /messages/by-day-by-role` (#640)

Usage: argo messages by-day-by-role [OPTIONS]

Options:
      --days <DAYS>  Lookback window in days (default 30, cap 365) [default: 30]
      --json         Output as a single JSON object (`{by_day_by_role}`)
  -h, --help         Print help
```

#### `argo messages by-role`

```text
Histogram of messages grouped by role. Mirrors the gateway's `GET /messages/by-role` (#611)

Usage: argo messages by-role [OPTIONS]

Options:
      --since-minutes <SINCE_MINUTES>  Filter to messages within this many recent minutes
      --json                           Output as a single JSON object (`{by_role}`)
  -h, --help                           Print help
```

#### `argo messages by-session`

```text
Top-N sessions by message row count. Mirrors the gateway's `GET /messages/by-session` (#615)

Usage: argo messages by-session [OPTIONS]

Options:
      --limit <LIMIT>                  Top-N session limit (default 50, cap 500) [default: 50]
      --since-minutes <SINCE_MINUTES>  Filter to messages within this many recent minutes
      --json                           Output as a single JSON object (`{by_session}`)
  -h, --help                           Print help
```

#### `argo messages recent`

```text
Workspace-wide recent message tail. Mirrors the gateway's `GET /messages/recent` (#619)

Usage: argo messages recent [OPTIONS]

Options:
      --limit <LIMIT>                  Tail size (default 50, cap 500) [default: 50]
      --role <ROLE>                    Filter to one role (user/assistant/tool/system)
      --since-minutes <SINCE_MINUTES>  Filter to messages within this many recent minutes
      --json                           Output as a JSON array
  -h, --help                           Print help
```

### `argo migrate`

```text
Data migrations (`mission-sqlcipher-3-migrate-cli`)

Usage: argo migrate <COMMAND>

Commands:
  encrypt  Convert an unencrypted SQLite database to SQLCipher-encrypted in place. Creates a timestamped backup under `~/.argo/backups/`, verifies row counts across every user table, then atomically swaps the encrypted file in. Idempotent: running against an already-encrypted DB is a clean no-op
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo migrate encrypt`

```text
Convert an unencrypted SQLite database to SQLCipher-encrypted in place. Creates a timestamped backup under `~/.argo/backups/`, verifies row counts across every user table, then atomically swaps the encrypted file in. Idempotent: running against an already-encrypted DB is a clean no-op

Usage: argo migrate encrypt [OPTIONS] --db <DB>

Options:
      --db <DB>
          Path to the unencrypted SQLite database file to convert

      --key-source <KEY_SOURCE>
          Where the raw 32-byte encryption key comes from. `env` reads `SKYCOT_DB_KEY_BASE64` (base64-encoded 32 bytes). `keyring` will route through the per-tenant `argo_config::cred::db_key` API once PR 2 / PR 4 land; today it falls back to the env-var path with a one-line notice

          Possible values:
          - env:     Read base64-encoded 32-byte key from `SKYCOT_DB_KEY_BASE64`
          - keyring: OS keyring (Keychain / Secret Service). Currently delegates to the env-var path; rewired by PR 4 (`mission-sqlcipher-4-default-flip`)
          
          [default: env]

      --dry-run
          Print the intended actions without writing anything

      --json
          Emit a `{ok, migrate_encrypt: {...}}` JSON envelope instead of a status line. Pairs cleanly with the rest of the CLI's list/show/status JSON contract

      --yes
          Skip the interactive "this will rewrite your database" confirmation. Required for non-interactive contexts (CI, runbooks, cron). `--dry-run` does not need this

  -h, --help
          Print help (see a summary with '-h')

Examples:
  argo migrate encrypt --db ~/.argo/session.db
  argo migrate encrypt --db ./session.db --key-source env
  argo migrate encrypt --db ./session.db --dry-run --json
```

### `argo model`

```text
Usage: argo model <COMMAND>

Commands:
  list  
  set   
  test  
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo model list`

```text
Usage: argo model list [OPTIONS]

Options:
      --json             Output as JSON. Equivalent to `--format json`
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
  -h, --help             Print help
```

#### `argo model set`

```text
Usage: argo model set <MODEL>

Arguments:
  <MODEL>  

Options:
  -h, --help  Print help
```

#### `argo model test`

```text
Usage: argo model test <MODEL>

Arguments:
  <MODEL>  

Options:
  -h, --help  Print help
```

### `argo new`

```text
Start a fresh session row in the local store. Without a daemon to actually call the LLM this is just a header insert — useful for prototyping the read-side surface and for capturing externally-streamed messages via the gateway (`POST /sessions/<id>/messages`)

Usage: argo new [OPTIONS] --provider <PROVIDER> --model <MODEL>

Options:
      --provider <PROVIDER>  Provider id (matches `manifests/providers/<id>.toml`)
      --model <MODEL>        Model id (e.g. claude-haiku, gpt-5-1)
      --title <TITLE>        Optional human-readable title
      --vertical <VERTICAL>  Bind the new session to a vertical-template id (#651). The vertical must be discoverable via `argo vertical list`
  -h, --help                 Print help

Examples:
  argo new --provider anthropic --model claude-sonnet-4-6
  argo new --provider mock --model mock --title "smoke test"
  argo new --provider anthropic --model claude-sonnet-4-6 --vertical gtm-engineer
```

### `argo onboard`

```text
First-experience operator walkthrough — workspace bootstrap, credentials, first connector, first worker (genesis), first action, done. 7 stages; stages 3-7 may Skip cleanly. The non-developer's entry point — pairs with `argo setup` (the CI-style provider-key probe)

Usage: argo onboard [OPTIONS]

Options:
      --non-interactive  Run without prompts. Reads `<workspace>/.argo/onboarding.toml` (or `--config <path>`) for stage inputs
      --json             Emit `{stages, any_fail, count}` as JSON instead of the pretty checklist
      --config <PATH>    Override the default `<workspace>/.argo/onboarding.toml` path used by `--non-interactive`
  -h, --help             Print help

Examples:
  argo onboard
  argo onboard --non-interactive
  argo onboard --non-interactive --json --config .argo/onboarding.toml
```

### `argo operator`

```text
Operator-scoped surfaces. Currently: - `risk-overrides` — per-operator overrides of the manifest-driven default risk classification (Stream `mission-operator-specific-risk-overrides`). Upgrade-only by default; downgrades require `--allow-downgrade` so a compromised account can't silently relax approvals

Usage: argo operator <COMMAND>

Commands:
  risk-overrides  Per-operator overrides of the manifest-driven default risk classification (Stream `mission-operator-specific-risk-overrides`). Upgrade-only by default — downgrades require `--allow-downgrade` so that a compromised account can't silently relax approval gates
  help            Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo operator risk-overrides`

```text
Per-operator overrides of the manifest-driven default risk classification (Stream `mission-operator-specific-risk-overrides`). Upgrade-only by default — downgrades require `--allow-downgrade` so that a compromised account can't silently relax approval gates

Usage: argo operator risk-overrides <COMMAND>

Commands:
  set     Set or replace an operator's override for one `(action-kind, [--vertical <id>])` scope
  get     Show every override on file for one operator
  list    List every override across every operator
  delete  Delete one override row keyed on `(operator-id, action-kind, [--vertical <id>])`
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo operator risk-overrides delete`

```text
Delete one override row keyed on `(operator-id, action-kind, [--vertical <id>])`

Usage: argo operator risk-overrides delete [OPTIONS] <OPERATOR_ID> <ACTION_KIND>

Arguments:
  <OPERATOR_ID>  Operator id
  <ACTION_KIND>  Action kind whose override is being cleared

Options:
      --vertical <VERTICAL>  Restrict the delete to a single vertical scope. Omit to clear the all-verticals row
      --json                 Emit JSON
  -h, --help                 Print help
```

##### `argo operator risk-overrides get`

```text
Show every override on file for one operator

Usage: argo operator risk-overrides get [OPTIONS] <OPERATOR_ID>

Arguments:
  <OPERATOR_ID>  Operator id

Options:
      --json  Emit JSON
  -h, --help  Print help
```

##### `argo operator risk-overrides list`

```text
List every override across every operator

Usage: argo operator risk-overrides list [OPTIONS]

Options:
      --json  Emit JSON
  -h, --help  Print help
```

##### `argo operator risk-overrides set`

```text
Set or replace an operator's override for one `(action-kind, [--vertical <id>])` scope

Usage: argo operator risk-overrides set [OPTIONS] <OPERATOR_ID> <ACTION_KIND> <RISK_LEVEL>

Arguments:
  <OPERATOR_ID>
          Operator id

  <ACTION_KIND>
          Action kind the override applies to (e.g. `send_email_external`)

  <RISK_LEVEL>
          Replacement risk level

          Possible values:
          - low:    Read-only or draft-only work
          - medium: Reversible public or monetary side effects
          - high:   Human-touching, irreversible, or high-cost side effects

Options:
      --vertical <VERTICAL>
          Restrict the override to a single vertical id. Omit to apply across every vertical this operator touches

      --allow-downgrade
          Acknowledge that this override is allowed to lower the classification below the manifest default. Without this flag, downgrades are refused at resolution time

      --json
          Emit JSON instead of a one-line summary

  -h, --help
          Print help (see a summary with '-h')
```

### `argo org`

```text
Org chart, addressable names, manager chain (`claude-org-awareness`)

Usage: argo org <COMMAND>

Commands:
  tree        Render the org chart for `<tenant>` as an ASCII tree
  member      Resolve a name-or-alias to its canonical OrgMember row
  reports-to  Walk the manager chain from a member toward the org root
  ingest      Bulk-import an org chart from a YAML doc
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo org ingest`

```text
Bulk-import an org chart from a YAML doc

Usage: argo org ingest [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to a YAML file whose top-level keys are `tenant`, `org`, `units`, and `members`

Options:
      --json  Emit JSON summary instead of a one-line text recap
  -h, --help  Print help
```

#### `argo org member`

```text
Resolve a name-or-alias to its canonical OrgMember row

Usage: argo org member [OPTIONS] --tenant <TENANT> <NAME_OR_ALIAS>

Arguments:
  <NAME_OR_ALIAS>  Canonical name OR alias to resolve

Options:
      --tenant <TENANT>  Tenant id whose org chart should be queried
      --json             Emit JSON
  -h, --help             Print help
```

#### `argo org reports-to`

```text
Walk the manager chain from a member toward the org root

Usage: argo org reports-to [OPTIONS] --tenant <TENANT> <MEMBER>

Arguments:
  <MEMBER>  Member name-or-alias to start the walk from

Options:
      --tenant <TENANT>  Tenant id
      --json             Emit JSON
  -h, --help             Print help
```

#### `argo org tree`

```text
Render the org chart for `<tenant>` as an ASCII tree

Usage: argo org tree [OPTIONS] --tenant <TENANT>

Options:
      --tenant <TENANT>  Tenant id whose org chart should be rendered
      --json             Emit JSON (one object per unit/member) instead of the ASCII tree
  -h, --help             Print help
```

### `argo packs`

```text
Install and manage vertical template packs

Usage: argo packs <COMMAND>

Commands:
  install   Install a pack release tarball into the local cache
  list      List cached packs
  update    Refresh one cached pack version
  dev-link  Point a pack name at a local checkout
  remove    Remove a dev-link and cached pack versions
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo packs dev-link`

```text
Point a pack name at a local checkout

Usage: argo packs dev-link [OPTIONS] <NAME> <PATH>

Arguments:
  <NAME>  
  <PATH>  

Options:
      --json  
  -h, --help  Print help
```

#### `argo packs install`

```text
Install a pack release tarball into the local cache

Usage: argo packs install [OPTIONS] --version <VERSION> <NAME>

Arguments:
  <NAME>  

Options:
      --version <VERSION>    
      --base-url <BASE_URL>  
      --force                
      --json                 
  -h, --help                 Print help
```

#### `argo packs list`

```text
List cached packs

Usage: argo packs list [OPTIONS]

Options:
      --json  
  -h, --help  Print help
```

#### `argo packs remove`

```text
Remove a dev-link and cached pack versions

Usage: argo packs remove [OPTIONS] <NAME>

Arguments:
  <NAME>  

Options:
      --version <VERSION>  
      --yes                
      --json               
  -h, --help               Print help
```

#### `argo packs update`

```text
Refresh one cached pack version

Usage: argo packs update [OPTIONS] --version <VERSION> <NAME>

Arguments:
  <NAME>  

Options:
      --version <VERSION>    
      --base-url <BASE_URL>  
      --json                 
  -h, --help                 Print help
```

### `argo persona`

```text
Persona maintenance — `$EDITOR`-driven update of the human-facing persona row attached to one or more workers (Stream `claude-persona-edit-cli`)

Usage: argo persona <COMMAND>

Commands:
  edit     Open `$EDITOR` on a TOML view of the named persona. Validates name regex, voice ≤ 4 KB, bio ≤ 1 KB. On validation failure the temp file is left in place and its path is printed so the operator can recover the unsaved edit
  history  List append-only persona revisions for one worker, newest first (Stream `mission-persona-evolution-audit-revert`)
  revert   Revert one persona field to a prior value cited by `rev_id` (Stream `mission-persona-evolution-audit-revert`). Prints the diff (target old / current / target old applied) and prompts for confirmation unless `--yes` is set. The revert itself is recorded as a NEW revision
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo persona edit`

```text
Open `$EDITOR` on a TOML view of the named persona. Validates name regex, voice ≤ 4 KB, bio ≤ 1 KB. On validation failure the temp file is left in place and its path is printed so the operator can recover the unsaved edit.

`$EDITOR` resolution chain: `$ARGO_EDITOR` > `$VISUAL` > `$EDITOR` > `vi`.

Usage: argo persona edit [OPTIONS] <NAME>

Arguments:
  <NAME>
          Persona name (the `name` column on the `persona` row)

Options:
      --tenant <TENANT>
          Optional tenant id (UUID). Required only when more than one persona row matches `name`

      --in-place <PATH>
          Skip the `$EDITOR` shell-out and read the new TOML from this path. Used by tests + scripts

      --json
          Emit a `{ok, name, bytes}` JSON object instead of the human-readable summary

  -h, --help
          Print help (see a summary with '-h')
```

#### `argo persona history`

```text
List append-only persona revisions for one worker, newest first (Stream `mission-persona-evolution-audit-revert`)

Usage: argo persona history [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUID) whose persona log to show

Options:
      --limit <LIMIT>  Maximum revisions to display. Defaults to 50 [default: 50]
      --json           Emit a JSON array instead of the human-readable table
  -h, --help           Print help
```

#### `argo persona revert`

```text
Revert one persona field to a prior value cited by `rev_id` (Stream `mission-persona-evolution-audit-revert`). Prints the diff (target old / current / target old applied) and prompts for confirmation unless `--yes` is set. The revert itself is recorded as a NEW revision

Usage: argo persona revert [OPTIONS] <REV_ID>

Arguments:
  <REV_ID>  Revision id (UUID) whose `old_value` becomes the new live persona value

Options:
      --yes   Skip the interactive confirmation prompt
      --json  Emit `{ok, rev_id, new_rev_id}` JSON instead of the human-readable confirmation line
  -h, --help  Print help
```

### `argo plan`

```text
Decompose a coarse goal into a workflow DAG (Stream `claude-runtime-planner-trait`). Offline TemplatePlanner by default; `--llm` opts into the Anthropic-backed LlmPlanner

Usage: argo plan <COMMAND>

Commands:
  generate  Decompose a coarse goal into a workflow DAG. The default uses the offline TemplatePlanner so this is hermetic. Pass `--llm` (with `ANTHROPIC_API_KEY` set) to use the LlmPlanner. The YAML goes to stdout unless `--output` is given
  show      Render a previously generated YAML + reasoning sidecar. `target` may be a YAML file or a "run id" — for v1 the run id is interpreted as a path under `<workspace>/.argo/plan_runs/<id>.yaml`
  execute   Run a workflow YAML through the offline mock executor (or dry-run only with `--dry-run`). Real action emit is gated by a follow-up brief; for v1 this proves the DAG is shaped correctly
  template  Manage Dreams-distilled learned plan templates (Stream `claude-dreams-plan-templates`). Cohorts of similar successful Planner decompositions get distilled by the Dreams API into reusable workflow YAML; operators promote good ones via this surface so the TemplatePlanner can match them ahead of the shipped defaults
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo plan execute`

```text
Run a workflow YAML through the offline mock executor (or dry-run only with `--dry-run`). Real action emit is gated by a follow-up brief; for v1 this proves the DAG is shaped correctly

Usage: argo plan execute [OPTIONS] <YAML_PATH>

Arguments:
  <YAML_PATH>  Workflow YAML to run

Options:
      --dry-run  Validate + topo-sort only, do not run any tasks
      --json     Output as a single JSON object
  -h, --help     Print help
```

#### `argo plan generate`

```text
Decompose a coarse goal into a workflow DAG. The default uses the offline TemplatePlanner so this is hermetic. Pass `--llm` (with `ANTHROPIC_API_KEY` set) to use the LlmPlanner. The YAML goes to stdout unless `--output` is given

Usage: argo plan generate [OPTIONS] --worker <WORKER> <GOAL>

Arguments:
  <GOAL>  One-line goal for the planner

Options:
      --worker <WORKER>      Worker id to default tasks to. Required
      --llm                  Use the LlmPlanner instead of TemplatePlanner
      --vertical <VERTICAL>  Optional vertical id forwarded to the LlmPlanner prompt
      --output <OUTPUT>      Path to write the YAML. Default: stdout
      --json                 Output the JSON envelope (`{reasoning, workflow_yaml, planner_kind, template_name}`) instead of bare YAML
  -h, --help                 Print help
```

#### `argo plan show`

```text
Render a previously generated YAML + reasoning sidecar. `target` may be a YAML file or a "run id" — for v1 the run id is interpreted as a path under `<workspace>/.argo/plan_runs/<id>.yaml`

Usage: argo plan show [OPTIONS] <TARGET>

Arguments:
  <TARGET>  File path or run id

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

#### `argo plan template`

```text
Manage Dreams-distilled learned plan templates (Stream `claude-dreams-plan-templates`). Cohorts of similar successful Planner decompositions get distilled by the Dreams API into reusable workflow YAML; operators promote good ones via this surface so the TemplatePlanner can match them ahead of the shipped defaults

Usage: argo plan template <COMMAND>

Commands:
  list     List Dreams-distilled learned plan templates. Filter by `--grade` (`pending` / `promoted` / `rejected`)
  show     Show one learned plan template — header + workflow body
  promote  Promote a Pending template to Promoted so the TemplatePlanner can match it
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo plan template list`

```text
List Dreams-distilled learned plan templates. Filter by `--grade` (`pending` / `promoted` / `rejected`)

Usage: argo plan template list [OPTIONS]

Options:
      --grade <GRADE>  Filter by lifecycle state
      --limit <LIMIT>  Cap on rows returned (default 50, hard ceiling 500) [default: 50]
      --json           Output as a single JSON object
  -h, --help           Print help
```

##### `argo plan template promote`

```text
Promote a Pending template to Promoted so the TemplatePlanner can match it

Usage: argo plan template promote [OPTIONS] <ID>

Arguments:
  <ID>  Template id

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

##### `argo plan template show`

```text
Show one learned plan template — header + workflow body

Usage: argo plan template show [OPTIONS] <ID>

Arguments:
  <ID>  Template id

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

### `argo plugin`

```text
Usage: argo plugin <COMMAND>

Commands:
  list     List plugins configured under `[plugin].auto_load`
  install  
  enable   
  disable  
  remove   
  run      
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo plugin disable`

```text
Usage: argo plugin disable <NAME>

Arguments:
  <NAME>  

Options:
  -h, --help  Print help
```

#### `argo plugin enable`

```text
Usage: argo plugin enable <NAME>

Arguments:
  <NAME>  

Options:
  -h, --help  Print help
```

#### `argo plugin install`

```text
Usage: argo plugin install <SOURCE>

Arguments:
  <SOURCE>  

Options:
  -h, --help  Print help
```

#### `argo plugin list`

```text
List plugins configured under `[plugin].auto_load`

Usage: argo plugin list [OPTIONS]

Options:
      --json             Output as a JSON array of strings for scripting. Equivalent to `--format json`
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
  -h, --help             Print help
```

#### `argo plugin remove`

```text
Usage: argo plugin remove <NAME>

Arguments:
  <NAME>  

Options:
  -h, --help  Print help
```

#### `argo plugin run`

```text
Usage: argo plugin run <NAME>

Arguments:
  <NAME>  

Options:
  -h, --help  Print help
```

### `argo policy`

```text
Declarative org policies (Stream `claude-policy-as-code`): list, show, validate, apply YAML files at `<workspace>/policies/<name>.yaml` that constrain the auto-approval gate via the resolution chain action → worker → roster → workspace-file → default

Usage: argo policy <COMMAND>

Commands:
  list      List policy files registered to the workspace (queries the `policy_files` table). Pass `--dir <path>` to list a directory of YAML files instead (filesystem view)
  show      Render one policy file (parsed + validated) for inspection. `<path>` is a path to a YAML file on disk
  validate  Parse + validate one policy YAML file. Exits non-zero on any parse/validation error. CI-friendly: pair with `find` to validate a whole policy directory in one shot
  apply     Register a policy YAML file to the workspace. Validates the file, copies it into `<workspace>/policies/<name>.yaml`, then records (workspace_id, name, sha256, applied_at) in the `policy_files` table
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo policy apply`

```text
Register a policy YAML file to the workspace. Validates the file, copies it into `<workspace>/policies/<name>.yaml`, then records (workspace_id, name, sha256, applied_at) in the `policy_files` table

Usage: argo policy apply [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to the source YAML file

Options:
      --json  Emit JSON instead of the human-readable confirmation
  -h, --help  Print help
```

#### `argo policy list`

```text
List policy files registered to the workspace (queries the `policy_files` table). Pass `--dir <path>` to list a directory of YAML files instead (filesystem view)

Usage: argo policy list [OPTIONS]

Options:
      --dir <DIR>  Show files in the given directory instead of the registered set. Useful for previewing a folder before `argo policy apply`
      --json       Emit a JSON array instead of the human-readable table
  -h, --help       Print help
```

#### `argo policy show`

```text
Render one policy file (parsed + validated) for inspection. `<path>` is a path to a YAML file on disk

Usage: argo policy show [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to a YAML file. Reads + parses + validates, then prints the structured result

Options:
      --json  Emit a single JSON object instead of YAML pretty-print
  -h, --help  Print help
```

#### `argo policy validate`

```text
Parse + validate one policy YAML file. Exits non-zero on any parse/validation error. CI-friendly: pair with `find` to validate a whole policy directory in one shot

Usage: argo policy validate [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to a YAML file on disk

Options:
      --json  Emit `{ok, name, errors}` as JSON instead of text
  -h, --help  Print help
```

### `argo provider`

```text
Usage: argo provider <COMMAND>

Commands:
  list          List provider manifests under `manifests/providers/`
  show          Render one provider manifest's full details (header, auth, transport, capabilities, models, discovery)
  validate      
  validate-all  Validate every manifest under `manifests/providers/`. Exits non-zero if any fails parsing — wire into CI to catch drift early
  test          
  cache-stats   Print prompt-cache hit-rate metrics aggregated per provider+model (mission-prompt-cache-hit-rate-observability). Numbers come from the in-process counters incremented on every Anthropic response; the gateway also exposes them under the standard `/metrics` endpoint
  help          Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo provider cache-stats`

```text
Print prompt-cache hit-rate metrics aggregated per provider+model (mission-prompt-cache-hit-rate-observability). Numbers come from the in-process counters incremented on every Anthropic response; the gateway also exposes them under the standard `/metrics` endpoint

Usage: argo provider cache-stats [OPTIONS]

Options:
      --provider <PROVIDER>  Filter to one provider id (e.g. `anthropic`)
      --model <MODEL>        Filter to one model id (e.g. `claude-opus-4-7`)
      --json                 Emit a JSON array instead of human-readable text
  -h, --help                 Print help
```

#### `argo provider list`

```text
List provider manifests under `manifests/providers/`

Usage: argo provider list [OPTIONS]

Options:
      --json             Output as a JSON array of `{id, family, base_url}` rows. Equivalent to `--format json`
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
  -h, --help             Print help
```

#### `argo provider show`

```text
Render one provider manifest's full details (header, auth, transport, capabilities, models, discovery)

Usage: argo provider show [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --json  Output the parsed manifest as a single JSON object
  -h, --help  Print help
```

#### `argo provider test`

```text
Usage: argo provider test <PROVIDER>

Arguments:
  <PROVIDER>  

Options:
  -h, --help  Print help
```

#### `argo provider validate`

```text
Usage: argo provider validate <MANIFEST>

Arguments:
  <MANIFEST>  

Options:
  -h, --help  Print help
```

#### `argo provider validate-all`

```text
Validate every manifest under `manifests/providers/`. Exits non-zero if any fails parsing — wire into CI to catch drift early

Usage: argo provider validate-all

Options:
  -h, --help  Print help
```

### `argo recover`

```text
LLM-assisted post-mortem for a failed session (Stream `claude-error-recovery-suggestions`). Walks the audit chain, calls Anthropic with the chain summary, returns 3 ranked fix suggestions. Cached per (session_id, audit_chain_hash) so re-runs are free. Advisory only — no auto-apply

Usage: argo recover [OPTIONS] <SESSION_ID>

Arguments:
  <SESSION_ID>  Session id (UUID) whose chain to analyze

Options:
      --json  Output as a single JSON object
  -h, --help  Print help

Examples:
  argo recover 01J9XYZ...
  argo recover 01J9XYZ... --json
```

### `argo recover-cache`

```text
Inspect or invalidate cached suggestions written by `argo recover` (Stream `codex-recover-cache-cli`). Pure CRUD over the `recovery_cache` table — no LLM calls

Usage: argo recover-cache <COMMAND>

Commands:
  list   List recent recovery cache entries newest-first
  show   Print cached suggestions for a session without re-calling the LLM. Operates fully offline
  clear  Invalidate cache entries — by session, by age, or wipe all (with typed confirmation)
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo recover-cache clear`

```text
Invalidate cache entries — by session, by age, or wipe all (with typed confirmation)

Usage: argo recover-cache clear [OPTIONS] [SESSION_ID]

Arguments:
  [SESSION_ID]  Session id whose cache entries to remove. Mutually exclusive with `--all` / `--older-than`

Options:
      --all                      Wipe every entry. Requires `--confirm yes-i-mean-it`
      --older-than <OLDER_THAN>  Evict entries older than this duration (e.g. `7d`, `30d`, `12h`)
      --confirm <CONFIRM>        Typed confirmation token. Required for `--all`
      --json                     Emit JSON envelope
  -h, --help                     Print help
```

#### `argo recover-cache list`

```text
List recent recovery cache entries newest-first

Usage: argo recover-cache list [OPTIONS]

Options:
      --since <SINCE>  Filter to entries cached within the past `<duration>` (e.g. `7d`, `12h`, `30m`). Default: no filter
      --limit <LIMIT>  Maximum number of rows to return (default: 25, max: 1000)
      --json           Emit JSON envelope
  -h, --help           Print help
```

#### `argo recover-cache show`

```text
Print cached suggestions for a session without re-calling the LLM. Operates fully offline

Usage: argo recover-cache show [OPTIONS] <SESSION_ID>

Arguments:
  <SESSION_ID>  Session id (UUID) whose cached suggestions to print

Options:
      --json  Emit JSON envelope
  -h, --help  Print help
```

### `argo replay`

```text
Time-travel session replay surface. `export` emits a self-contained tarball of one session (audit chain, attached skills, per-action bundles, manifest with checksum) for offline analysis or support hand-off (`mission-replay-cli-bundle-export`)

Usage: argo replay <COMMAND>

Commands:
  export  Export a session as a self-contained tar/tar.gz bundle for offline replay or support hand-off. Bundle includes the audit chain in JSONL, a verify-chain summary, attached skill ids, per-action bundles, a manifest with a content checksum, and a short README. With `--sanitize`, JSON payloads are scrubbed of provider keys, JWTs, emails, and phone numbers before write
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo replay export`

```text
Export a session as a self-contained tar/tar.gz bundle for offline replay or support hand-off. Bundle includes the audit chain in JSONL, a verify-chain summary, attached skill ids, per-action bundles, a manifest with a content checksum, and a short README. With `--sanitize`, JSON payloads are scrubbed of provider keys, JWTs, emails, and phone numbers before write

Usage: argo replay export [OPTIONS] --out <OUT> <SESSION_ID>

Arguments:
  <SESSION_ID>  Session UUID to export

Options:
      --out <OUT>     Output path (tar.gz by default; tar with `--uncompressed`)
      --uncompressed  Emit a plain tar archive instead of gzip-compressed
      --sanitize      Redact provider keys, JWTs, emails, and phone numbers in JSON payloads before bundling. The manifest records `sanitized = true` so downstream consumers can distinguish
  -h, --help          Print help

Examples:
  argo replay export 01J9XYZ... --out /tmp/s.tar.gz
  argo replay export 01J9XYZ... --out /tmp/s.tar --uncompressed
  argo replay export 01J9XYZ... --out /tmp/s.tar.gz --sanitize
```

### `argo restore`

```text
Restore a workspace backup tarball

Usage: argo restore [OPTIONS] <PATH>

Arguments:
  <PATH>  

Options:
      --into <INTO>  Restore into this workspace directory
      --dry-run      List files without extracting them
      --json         Emit a JSON envelope
  -h, --help         Print help

Examples:
  argo restore .argo/backups/2026-05-13.tar.gz
  argo restore ./snap.tar.gz --dry-run --json
```

### `argo resume`

```text
Re-render a recorded session: prints its header and every message in order. Read-only — for the live "continue this session against an LLM" flow, see the daemon's /sessions/<id>/runs binding (separate PR)

Usage: argo resume <ID>

Arguments:
  <ID>  Session id (full UUID)

Options:
  -h, --help  Print help

Examples:
  argo resume 01J9XYZ...
```

### `argo roster`

```text
Worker rosters: group-level policy, channels, budget cap (`claude-worker-roster`)

Usage: argo roster <COMMAND>

Commands:
  create         Create a new roster (group of workers + shared policy)
  list           List rosters in a tenant
  show           Show a roster (with members hydrated)
  add-member     Add a worker to a roster
  remove-member  Remove a worker from a roster
  help           Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo roster add-member`

```text
Add a worker to a roster

Usage: argo roster add-member [OPTIONS] --tenant <TENANT> <ROSTER> <WORKER>

Arguments:
  <ROSTER>  Roster name OR uuid
  <WORKER>  Worker uuid to add

Options:
      --tenant <TENANT>  Tenant id this roster belongs to
      --json             Emit JSON instead of the human-friendly confirmation
  -h, --help             Print help
```

#### `argo roster create`

```text
Create a new roster (group of workers + shared policy)

Usage: argo roster create [OPTIONS] --tenant <TENANT> <NAME>

Arguments:
  <NAME>  Operator-friendly group name (unique per tenant)

Options:
      --tenant <TENANT>
          Tenant id this roster belongs to
      --policy <POLICY>
          Path to a JSON file describing the AutoApprovalPolicy. Falls back to the disabled default when omitted
      --channels <CHANNELS>
          Path to a JSON file describing the NotifyChannelRef list. Falls back to an empty list when omitted
      --budget-cap-usd <BUDGET_CAP_USD>
          Optional group-level spend ceiling in whole USD
      --json
          Emit JSON instead of the human-friendly recap
  -h, --help
          Print help
```

#### `argo roster list`

```text
List rosters in a tenant

Usage: argo roster list [OPTIONS] --tenant <TENANT>

Options:
      --tenant <TENANT>  Tenant id to filter by
      --json             Emit JSON instead of the table view
  -h, --help             Print help
```

#### `argo roster remove-member`

```text
Remove a worker from a roster

Usage: argo roster remove-member [OPTIONS] --tenant <TENANT> <ROSTER> <WORKER>

Arguments:
  <ROSTER>  Roster name OR uuid
  <WORKER>  Worker uuid to remove

Options:
      --tenant <TENANT>  Tenant id this roster belongs to
      --json             Emit JSON instead of the human-friendly confirmation
  -h, --help             Print help
```

#### `argo roster show`

```text
Show a roster (with members hydrated)

Usage: argo roster show [OPTIONS] --tenant <TENANT> <ROSTER>

Arguments:
  <ROSTER>  Roster name OR uuid

Options:
      --tenant <TENANT>  Tenant id this roster belongs to
      --json             Emit JSON instead of the multi-line view
  -h, --help             Print help
```

### `argo route`

```text
Usage: argo route <COMMAND>

Commands:
  inspect  
  bench    
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo route bench`

```text
Usage: argo route bench

Options:
  -h, --help  Print help
```

#### `argo route inspect`

```text
Usage: argo route inspect <SESSION_ID>

Arguments:
  <SESSION_ID>  

Options:
  -h, --help  Print help
```

### `argo sandbox`

```text
Cloud-sandbox lifecycle ops (Stream `mission-e2b-reap-idle`). `e2b list` / `e2b reap-idle` for out-of-band cleanup of leaked E2B sandboxes — orthogonal to the CU executor's in-band Drop path

Usage: argo sandbox <COMMAND>

Commands:
  e2b   E2B managed-sandbox ops
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo sandbox e2b`

```text
E2B managed-sandbox ops

Usage: argo sandbox e2b <COMMAND>

Commands:
  list       JSON-line list of all sandboxes the configured E2B API key owns
  reap-idle  One-shot reap of sandboxes idle longer than `--max-age` seconds. Emits a single JSON summary object
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo sandbox e2b list`

```text
JSON-line list of all sandboxes the configured E2B API key owns

Usage: argo sandbox e2b list

Options:
  -h, --help  Print help
```

##### `argo sandbox e2b reap-idle`

```text
One-shot reap of sandboxes idle longer than `--max-age` seconds. Emits a single JSON summary object

Usage: argo sandbox e2b reap-idle [OPTIONS]

Options:
      --max-age <MAX_AGE>  Idle-time threshold in seconds. Default 600 (10 minutes) [default: 600]
      --dry-run            List the would-be-reaped sandboxes without closing them
  -h, --help               Print help
```

### `argo security`

```text
Coordinated security operations. Subcommands: - `audit` — vuln-scan wrapper around `cargo audit` + `cargo deny check advisories` (Mission `mission-vuln-management-workflow`). See `docs/skycot/security/vuln-management.md`. - `rotate-keys` — key-rotation coordinator across DB encryption / credentials master / gateway bearer / channel HMAC / per-connector OAuth (Mission `mission-automated-key-rotation`). See the "Key rotation" section of `docs/skycot/RUNBOOK.md`

Usage: argo security <COMMAND>

Commands:
  audit         Run `cargo audit` + `cargo deny check advisories`, merge their results, and print a single JSON envelope of `{vulnerabilities[], total_count_by_severity}`. Exit codes: `0` = no unmitigated rows above floor, `1` = at least one unmitigated row above floor, `2` = tooling error (missing `cargo-audit`/`cargo-deny`, network failure pulling the RustSec feed, malformed JSON output)
  scan-secrets  Scan one or more paths for likely credentials (OpenAI / Anthropic / AWS / Stripe / Slack / GitHub / SSH private keys / JWTs / Argo bearer tokens). CI parity for the pre-commit hook installed by `argo doctor install-hooks`. Exits non-zero when any unsuppressed match is found
  rotate-keys   Coordinated key-rotation operations across the workspace (Mission `mission-automated-key-rotation`)
  help          Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo security audit`

```text
Run `cargo audit` + `cargo deny check advisories`, merge their results, and print a single JSON envelope of `{vulnerabilities[], total_count_by_severity}`. Exit codes: `0` = no unmitigated rows above floor, `1` = at least one unmitigated row above floor, `2` = tooling error (missing `cargo-audit`/`cargo-deny`, network failure pulling the RustSec feed, malformed JSON output)

Usage: argo security audit [OPTIONS]

Options:
      --severity-floor <SEVERITY_FLOOR>
          Drop rows below this severity from `vulnerabilities[]`. One of `low` (default), `medium`, `high`, `critical`. Below-floor rows still count toward `total_count_by_severity` [default: low]
      --compact
          Emit single-line compact JSON instead of pretty-printed. Useful for piping into `jq` / ops dashboards
  -h, --help
          Print help
```

#### `argo security rotate-keys`

```text
Coordinated key-rotation operations across the workspace (Mission `mission-automated-key-rotation`)

Usage: argo security rotate-keys <COMMAND>

Commands:
  list           Enumerate every named key with its last-rotated date and age in days. Default output is a Markdown table; `--json` emits a JSON array suitable for scripting
  plan           Show the rotation plan for `<key-name>`: which subsystem owns it, what re-encryption / re-keying steps run, and any operator follow-ups (config edits, restarts). Read-only
  commit         Execute the rotation plan for `<key-name>`. Idempotent — re-running with the same `--new-key-source` after a successful rotation is a no-op. Always writes a `security.key_rotated` audit row
  audit-key-age  Exit non-zero when any registered key is older than `--threshold` days (default 90). Designed for cron-driven monitoring. Exit codes: 0 = all fresh, 1 = ≥1 stale, 2 = config error
  help           Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo security rotate-keys audit-key-age`

```text
Exit non-zero when any registered key is older than `--threshold` days (default 90). Designed for cron-driven monitoring. Exit codes: 0 = all fresh, 1 = ≥1 stale, 2 = config error

Usage: argo security rotate-keys audit-key-age [OPTIONS]

Options:
      --threshold <THRESHOLD>  Age threshold in days. Keys older than this exit non-zero [default: 90]
      --json                   Emit a single JSON envelope (`{stale, fresh, threshold_days}`) instead of human-readable status
  -h, --help                   Print help
```

##### `argo security rotate-keys commit`

```text
Execute the rotation plan for `<key-name>`. Idempotent — re-running with the same `--new-key-source` after a successful rotation is a no-op. Always writes a `security.key_rotated` audit row

Usage: argo security rotate-keys commit [OPTIONS] <KEY_NAME>

Arguments:
  <KEY_NAME>  Registered key name. Use `list` to enumerate

Options:
      --new-key-source <NEW_KEY_SOURCE>
          Optional source for the new key material. Accepted values: `random` (default — OS RNG), `prompt` (read from stdin without echo), or `env:<VAR>` (read from an env var). Never logged [default: random]
      --dry-run
          Skip the audit row + last-rotated bump; print the plan only. Equivalent to `argo security rotate-keys plan <name>`
      --json
          Emit a single JSON envelope instead of human-readable status
  -h, --help
          Print help
```

##### `argo security rotate-keys list`

```text
Enumerate every named key with its last-rotated date and age in days. Default output is a Markdown table; `--json` emits a JSON array suitable for scripting

Usage: argo security rotate-keys list [OPTIONS]

Options:
      --json             Equivalent to `--format json`
      --format <FORMAT>  Output format: `markdown` (default) or `json` [default: markdown]
  -h, --help             Print help
```

##### `argo security rotate-keys plan`

```text
Show the rotation plan for `<key-name>`: which subsystem owns it, what re-encryption / re-keying steps run, and any operator follow-ups (config edits, restarts). Read-only

Usage: argo security rotate-keys plan [OPTIONS] <KEY_NAME>

Arguments:
  <KEY_NAME>  Registered key name. Use `list` to enumerate

Options:
      --json  Equivalent to `--format json`
  -h, --help  Print help
```

#### `argo security scan-secrets`

```text
Scan one or more paths for likely credentials (OpenAI / Anthropic / AWS / Stripe / Slack / GitHub / SSH private keys / JWTs / Argo bearer tokens). CI parity for the pre-commit hook installed by `argo doctor install-hooks`. Exits non-zero when any unsuppressed match is found

Usage: argo security scan-secrets [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  One or more file or directory paths to scan. Directories are walked recursively, skipping `.git/` and files whose first 1 KiB contains a NUL byte (binary heuristic)

Options:
      --allowlist <ALLOWLIST>  Path to the operator allowlist. Defaults to `.skycot/secret-scan-allowlist.toml`. Missing file is OK (treated as empty allowlist); malformed TOML is fatal
      --json                   Emit findings as one JSON array on stdout. Default is a human-readable `file:line: [id] human -> redacted` per match
  -h, --help                   Print help
```

### `argo self-update`

```text
Verified binary self-upgrade (Stream `codex-self-update-cli`, V1.4). Queries GitHub Releases for a newer build, downloads the platform asset + detached Ed25519 signature, verifies against the embedded release pubkey, and atomically replaces the running binary. Best-effort gateway daemon restart

Usage: argo self-update [OPTIONS]

Options:
      --check              Report the latest available version without installing
      --channel <CHANNEL>  Release channel: `stable` (default) or `beta` (allows pre-release tags) [default: stable]
      --force              Skip the "already on latest" short-circuit and reinstall. Never bypasses signature verification
      --json               Emit a JSON envelope instead of human-readable status
  -h, --help               Print help

Examples:
  argo self-update --check
  argo self-update
  argo self-update --channel beta --force --json
```

### `argo session`

```text
Usage: argo session <COMMAND>

Commands:
  start             Create a session row directly (orchestrator path)
  list              List sessions with rich filters (status, provider, model, title-substring, spend window, etc.)
  count             
  watch             Stream a session's new messages live (poll loop)
  messages          Render the message log for one session
  attach-skill      
  detach-skill      
  skills            
  clear-skills      
  show              Print the session header (provider, model, status, spend)
  transcript        
  export            
  search            Full-text search across session messages
  delete            Delete a session row + its messages (use `--force` to skip the prompt)
  replay            
  diff              
  title             
  end               
  reopen            
  end-all           
  bisect            
  clone             
  branch            
  stats             
  turns             
  by-day            
  by-provider       
  by-model          
  audit-by-kind     
  audit-by-tool     
  messages-by-role  
  tool-errors       
  approve           
  revoke-approval   
  list-grants       
  capability-gaps   
  help              Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo session approve`

```text
Usage: argo session approve [OPTIONS] --tool <TOOL> <SESSION_ID>

Arguments:
  <SESSION_ID>  

Options:
      --tool <TOOL>  
      --note <NOTE>  
      --once         
      --json         
  -h, --help         Print help
```

#### `argo session attach-skill`

```text
Usage: argo session attach-skill <ID> <SKILL>

Arguments:
  <ID>     
  <SKILL>  

Options:
  -h, --help  Print help
```

#### `argo session audit-by-kind`

```text
Usage: argo session audit-by-kind [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --since-minutes <SINCE_MINUTES>  
      --json                           
  -h, --help                           Print help
```

#### `argo session audit-by-tool`

```text
Usage: argo session audit-by-tool [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --since-minutes <SINCE_MINUTES>  
      --json                           
  -h, --help                           Print help
```

#### `argo session bisect`

```text
Usage: argo session bisect <ID> <HYPOTHESIS>

Arguments:
  <ID>          
  <HYPOTHESIS>  

Options:
  -h, --help  Print help
```

#### `argo session branch`

```text
Usage: argo session branch [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --through-seq <THROUGH_SEQ>  
  -h, --help                       Print help
```

#### `argo session by-day`

```text
Usage: argo session by-day [OPTIONS]

Options:
      --days <DAYS>  [default: 30]
      --json         
  -h, --help         Print help
```

#### `argo session by-model`

```text
Usage: argo session by-model [OPTIONS]

Options:
      --since-minutes <SINCE_MINUTES>  
      --json                           
  -h, --help                           Print help
```

#### `argo session by-provider`

```text
Usage: argo session by-provider [OPTIONS]

Options:
      --since-minutes <SINCE_MINUTES>  
      --json                           
  -h, --help                           Print help
```

#### `argo session capability-gaps`

```text
Usage: argo session capability-gaps [OPTIONS] <SESSION_ID>

Arguments:
  <SESSION_ID>  

Options:
      --since-minutes <SINCE_MINUTES>  [default: 0]
      --threshold <THRESHOLD>          [default: 3]
      --json                           
  -h, --help                           Print help
```

#### `argo session clear-skills`

```text
Usage: argo session clear-skills <ID>

Arguments:
  <ID>  

Options:
  -h, --help  Print help
```

#### `argo session clone`

```text
Usage: argo session clone <ID>

Arguments:
  <ID>  

Options:
  -h, --help  Print help
```

#### `argo session count`

```text
Usage: argo session count [OPTIONS]

Options:
      --status <STATUS>                          [default: all]
      --since-minutes <SINCE_MINUTES>            
      --older-than-minutes <OLDER_THAN_MINUTES>  
      --provider <PROVIDER>                      
      --model <MODEL>                            
      --title-contains <TITLE_CONTAINS>          
      --spent-min <SPENT_MIN>                    
      --spent-max <SPENT_MAX>                    
      --json                                     
  -h, --help                                     Print help
```

#### `argo session delete`

```text
Delete a session row + its messages (use `--force` to skip the prompt)

Usage: argo session delete [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --force  
  -h, --help   Print help

Examples:
  argo session delete 01J9XYZ...
  argo session delete 01J9XYZ... --force
```

#### `argo session detach-skill`

```text
Usage: argo session detach-skill <ID> <SKILL>

Arguments:
  <ID>     
  <SKILL>  

Options:
  -h, --help  Print help
```

#### `argo session diff`

```text
Usage: argo session diff [OPTIONS] <ID1> <ID2>

Arguments:
  <ID1>  
  <ID2>  

Options:
      --json             
      --format <FORMAT>  [default: text]
  -h, --help             Print help
```

#### `argo session end`

```text
Usage: argo session end <ID>

Arguments:
  <ID>  

Options:
  -h, --help  Print help
```

#### `argo session end-all`

```text
Usage: argo session end-all [OPTIONS]

Options:
      --older-than-minutes <OLDER_THAN_MINUTES>  
  -h, --help                                     Print help
```

#### `argo session export`

```text
Usage: argo session export [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --out <OUT>        
      --include-audit    
      --format <FORMAT>  [default: jsonl]
  -h, --help             Print help
```

#### `argo session list`

```text
List sessions with rich filters (status, provider, model, title-substring, spend window, etc.)

Usage: argo session list [OPTIONS]

Options:
      --tenant <TENANT>                          
      --limit <LIMIT>                            [default: 25]
      --json                                     
      --format <FORMAT>                          [default: text]
      --since-minutes <SINCE_MINUTES>            
      --older-than-minutes <OLDER_THAN_MINUTES>  
      --spent-min <SPENT_MIN>                    
      --spent-max <SPENT_MAX>                    
      --provider <PROVIDER>                      
      --model <MODEL>                            
      --status <STATUS>                          [default: all]
      --title-contains <TITLE_CONTAINS>          
      --sort-by <SORT_BY>                        [default: started-at]
      --vertical <VERTICAL>                      
  -h, --help                                     Print help

Examples:
  argo session list
  argo session list --status active --limit 50
  argo session list --since-minutes 60 --provider anthropic --json
```

#### `argo session list-grants`

```text
Usage: argo session list-grants [OPTIONS] <SESSION_ID>

Arguments:
  <SESSION_ID>  

Options:
      --json  
  -h, --help  Print help
```

#### `argo session messages`

```text
Render the message log for one session

Usage: argo session messages [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --json                                 
      --format <FORMAT>                      [default: text]
      --limit <LIMIT>                        
      --tail <TAIL>                          
      --role <ROLE>                          
      --since-minutes <SINCE_MINUTES>        
      --seq-min <SEQ_MIN>                    
      --seq-max <SEQ_MAX>                    
      --content-contains <CONTENT_CONTAINS>  
  -h, --help                                 Print help

Examples:
  argo session messages 01J9XYZ...
  argo session messages 01J9XYZ... --role assistant --tail 20
  argo session messages 01J9XYZ... --json
```

#### `argo session messages-by-role`

```text
Usage: argo session messages-by-role [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --since-minutes <SINCE_MINUTES>  
      --json                           
  -h, --help                           Print help
```

#### `argo session reopen`

```text
Usage: argo session reopen <ID>

Arguments:
  <ID>  

Options:
  -h, --help  Print help
```

#### `argo session replay`

```text
Usage: argo session replay [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --model <MODEL>          
      --provider <PROVIDER>    
      --tools <TOOLS>          
      --workspace <WORKSPACE>  
      --stream                 
  -h, --help                   Print help
```

#### `argo session revoke-approval`

```text
Usage: argo session revoke-approval [OPTIONS] --tool <TOOL> <SESSION_ID>

Arguments:
  <SESSION_ID>  

Options:
      --tool <TOOL>  
      --json         
  -h, --help         Print help
```

#### `argo session search`

```text
Full-text search across session messages

Usage: argo session search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  

Options:
      --limit <LIMIT>      [default: 20]
      --session <SESSION>  
      --role <ROLE>        
      --json               
      --format <FORMAT>    [default: text]
      --count              
  -h, --help               Print help

Examples:
  argo session search "timeout"
  argo session search "approval denied" --role assistant --limit 5 --json
  argo session search "oom" --count
```

#### `argo session show`

```text
Print the session header (provider, model, status, spend)

Usage: argo session show [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --tenant <TENANT>  
      --json             
      --format <FORMAT>  [default: text]
  -h, --help             Print help

Examples:
  argo session show 01J9XYZ...
  argo session show 01J9XYZ... --json
```

#### `argo session skills`

```text
Usage: argo session skills <ID>

Arguments:
  <ID>  

Options:
  -h, --help  Print help
```

#### `argo session start`

```text
Create a session row directly (orchestrator path)

Usage: argo session start [OPTIONS] --title <TITLE>

Options:
      --tenant <TENANT>  
      --title <TITLE>    
      --worker <WORKER>  
      --json             
  -h, --help             Print help

Examples:
  argo session start --title "morning standup notes"
  argo session start --title "design review" --worker scout --json
```

#### `argo session stats`

```text
Usage: argo session stats [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --json  
  -h, --help  Print help
```

#### `argo session title`

```text
Usage: argo session title [OPTIONS] <ID> [NEW]

Arguments:
  <ID>   
  [NEW]  

Options:
      --clear  
  -h, --help   Print help
```

#### `argo session tool-errors`

```text
Usage: argo session tool-errors [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --since-minutes <SINCE_MINUTES>  
      --json                           
  -h, --help                           Print help
```

#### `argo session transcript`

```text
Usage: argo session transcript [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --json  
  -h, --help  Print help
```

#### `argo session turns`

```text
Usage: argo session turns [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --json  
  -h, --help  Print help
```

#### `argo session watch`

```text
Stream a session's new messages live (poll loop)

Usage: argo session watch <ID>

Arguments:
  <ID>  

Options:
  -h, --help  Print help

Examples:
  argo session watch 01J9XYZ...
```

### `argo setup`

```text
First-time interactive setup. Mirrors the `curl … | bash && argo setup` Hermes-style two-step install (#646). Idempotent: re-runnable to verify configuration or pick a different provider

Usage: argo setup [OPTIONS]

Options:
      --check              Verification mode — skip the interactive walkthrough and exit non-zero if no provider key is loadable from the process environment. Designed for CI / production- launch checklists; pairs with the `provider_keys.md` rotation runbook
      --non-interactive    Run the setup wizard without prompts. Intended for CI and scripted bootstrap flows
      --json               Emit `{steps, ready}` as JSON instead of the checklist table
      --require <ENV_VAR>  Require this specific env var to be set (repeatable). When provided alongside `--check`, the probe demands every named var resolves to a non-empty value; otherwise the existing "at least one" contract applies. Use this when your vertical declares both a primary AND a fallback provider — the launch checklist in `provider_keys.md` says BOTH should be loadable so the fallback actually works on 503
  -h, --help               Print help

Examples:
  argo setup
  argo setup --check
  argo setup --check --require ANTHROPIC_API_KEY --require OPENAI_API_KEY
```

### `argo skill`

```text
Usage: argo skill <COMMAND>

Commands:
  list               
  count              Print just the count of active skills. Default: one int per line; `--json` emits `{count}`
  view               
  explain            Explain one skill's provenance, version, Forge, trust, and usage history
  verify             Verify a skill's body matches its frontmatter `provenance.hash`. Pass `--all` to verify every active skill in the workspace
  create             Create a new skill from CLI flags or stdin. Inserts a fresh row via `SkillStore::install`; rejects duplicate names (use `argo skill refine` to bump an existing skill instead)
  search             Search the cross-tenant OCI skill registry
  catalog            Browse marketplace skills with local-install status marks (mission-cli-skill-catalog). Unlike `skill list` (local-only) and `skill search` (query-driven), this prints every marketplace entry and joins it against the locally-installed set so operators can see at a glance what is installable. `--vertical` is matched against the marketplace `tenant` field (no separate vertical column exists yet on `IndexedSkill`); `--tag` is a substring match against the marketplace entry description
  publish            Publish a local skill to the marketplace registry
  unpublish          Remove a mutable marketplace tag while preserving blobs
  tags               List marketplace tags for one skill
  runs               Show chronological skill invocation history
  versions           `argo skill versions <name>` — list every version of `<name>` installed in this workspace, newest first
  ab-test            `argo skill ab-test <name> --variants vN,vM --runs N` — create a new A/B test run pitting two existing versions of `<name>` against each other
  ab-status          `argo skill ab-status <run-id>` — print the run row + per-variant counts + the chi-square verdict at this moment
  ab-promote         `argo skill ab-promote <run-id>` — apply a run's winner to the `skill` table by stamping the loser's `superseded_by`. If the run hasn't auto-completed, the operator must supply `--winner <version>` explicitly
  ab-snapshot        Print the most recent A/B run's per-variant counts + significance for one skill name (operator inspection; added by mission-skill-select-variant-runtime-wire)
  sign               Sign a local skill manifest with a cosign-compatible ed25519 key
  verify-sig         Verify a published skill against its detached cosign signature. Uses a hyphenated subcommand name because `verify` is already occupied by the body/provenance hash check (`SkillCmd::Verify`)
  keygen             Generate a cosign-style ed25519 keypair for testing/onboarding
  review             Drive the learned-skill review gate (#681 lifecycle): list pending proposals + approve / reject them. Without this surface the lifecycle gate is theoretical — agents can propose skills via `propose_for_review`, but operators have no way to act on the queue
  conflicts          Inspect + resolve workspace ↔ library skill name collisions surfaced by the H-4b layered lookup (Stream `claude-skill-conflict-resolution`)
  refine             Append a hint to the active skill's body and supersede it with a fresh version. Manual mutator — the score-driven version (run bench, accept only when the new score wins) lands once `forge optimize` learns to mutate
  rollback           Promote a prior version back to active
  install            
  update             Mutate metadata on an active skill in place. Currently only the skill-side trust gate (`--min-trust-grade`) is wired — future flag pass-throughs extend this command instead of adding fresh subcommands. M6 precursor 1/3 (Stream `mission-skill-min-trust-grade-column`)
  distill-now        Run cohort-based Dreams distillation over recent trajectories and emit a per-cohort summary. Reads `TrajectoryInput` JSON array from stdin (the producer is out of v1 scope — a nightly tick will land later). Brief: `claude-dreams-skill-distillation`
  export             Export one active skill (or an explicit `name:version`) into a portable `.skill.tar.gz` artifact
  export-all         Export every active skill into a directory of portable `.skill.tar.gz` artifacts
  import             Import a portable `.skill.tar.gz` artifact into the local skill store
  lock               
  unlock             
  quality            Show the latest quality score (0..=100) + inputs for one skill (`claude-skill-quality-judge` / `claude-skill-quality-score-vis`). With `--limit N` the command renders the last N scores as a history block. The `--format markdown` flag emits the human-readable per-section view; `--json` (or `--format json`) wins for scripting; the legacy text default is preserved when neither is passed for backward compatibility
  deps               Show the dependency DAG of one skill: which skills it `argo skill run`s, references via `\`skill:NAME\``, etc. (`claude-skill-dependency-graph`). `--reverse` flips the arrows so you can see "what would break if I rolled back X?" Markdown tree by default; `--json` emits the resolved edges
  quality-list       List every scored skill, worst first. `--threshold` keeps rows with score ≤ threshold; `--worst` keeps only the lowest-scoring N
  quality-trend      ASCII sparkline of the past `--days` daily quality scores for one skill (gaps render as `·`)
  rate               Rate a marketplace-installed skill (operator thumbs -> publisher's skill_rating table; Stream `mission-skill-ratings-publisher`). The skill must be installed from the marketplace (`provenance.source` of the form `published:<grade>:<publisher>`); non-published sources return an explicit "only published skills can be rated" error
  ratings            Show aggregated up/down counts + score + recent comments for one marketplace-installed skill (Stream `mission-skill-ratings-publisher`). Counts roll up across every rater tenant; `comments` are the most-recent 10
  set-flag           Recompute the quality score for one or every active skill from the audit log (mission-skill-quality-cron). For each skill: walks `tool_result` audit rows, derives the four `QualityInputs`, runs the existing `compute_score` primitive, and persists the result via the standard skill-quality store. Output is one line per skill: `<skill_id> <old_score> -> <new_score> (<delta>)`. Designed to be cron-driven; `--dry-run` is the rehearsal switch. Set a per-skill operator flag (mission-forge-operator-feedback-eval-signal). Today the only consumer is `argo forge` — the `--no-forge-feedback` flag tells the Forge ingestion path to skip operator-vote eval cases for this skill. The underlying `skill_flag` table is generic, so future per-skill toggles piggyback on the same surface
  quality-recompute  
  migrate            Record a publisher-side `skill_version_migration` row linking `--from` to `--to`. Tenant workspaces with the `--from` version installed will see this when they run `argo skill upgrade <name>` or `argo skill list-migrations`. Idempotent on the composite PK. (`mission-skill-version-migration-path`)
  upgrade            Apply a pending `skill_version_migration` to the locally- installed version of `<name>`. With `--dry-run`, prints what would happen without superseding. Without `--yes`, only auto-upgrade migrations are applied (others are surfaced for operator confirmation). (`mission-skill-version-migration-path`)
  list-migrations    List every pending migration across the tenant's installed skills (for each active skill, the row whose `from_version` matches its installed version). Operator dashboard input. (`mission-skill-version-migration-path`)
  ir                 Canonical-IR maintenance ops (Track L). One-shot tools the operator runs against the local skill store — currently just `backfill`, which synthesises a minimal `SkillGraph` for every installed skill that lacks one (brief `L3-skill-ir-backfill`)
  pack               `argo skill pack <sub>` tree
  help               Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo skill ab-promote`

```text
`argo skill ab-promote <run-id>` — apply a run's winner to the `skill` table by stamping the loser's `superseded_by`. If the run hasn't auto-completed, the operator must supply `--winner <version>` explicitly

Usage: argo skill ab-promote [OPTIONS] <RUN_ID>

Arguments:
  <RUN_ID>  Run id

Options:
      --winner <WINNER>
          Override the auto-detected winner (required for runs that haven't reached significance yet)
      --quality-weight <QUALITY_WEIGHT>
          Fraction (0.0..=1.0) of the auto-promote winner score contributed by the latest cached `QualityScore` per variant. `0.0` keeps today's success-rate-only behavior [default: 0.3]
      --max-score-age-days <MAX_SCORE_AGE_DAYS>
          Quality scores older than this are treated as missing (the variant falls through to success-rate-only logic) [default: 7]
      --min-samples <MIN_SAMPLES>
          Minimum samples per variant before auto-promote will run the significance gate. Default 30 [default: 30]
      --json
          Emit JSON
  -h, --help
          Print help
```

#### `argo skill ab-snapshot`

```text
Print the most recent A/B run's per-variant counts + significance for one skill name (operator inspection; added by mission-skill-select-variant-runtime-wire)

Usage: argo skill ab-snapshot [OPTIONS] <NAME>

Arguments:
  <NAME>  Skill name to inspect

Options:
      --min-samples <MIN_SAMPLES>  Minimum samples per variant before significance is computed. Default 30 (matches `ab-status`) [default: 30]
      --json                       Emit a JSON envelope instead of the human-readable summary
  -h, --help                       Print help
```

#### `argo skill ab-status`

```text
`argo skill ab-status <run-id>` — print the run row + per-variant counts + the chi-square verdict at this moment

Usage: argo skill ab-status [OPTIONS] <RUN_ID>

Arguments:
  <RUN_ID>  Run id (UUIDv7) returned by `argo skill ab-test`

Options:
      --min-samples <MIN_SAMPLES>  Minimum samples per variant before significance is computed. Default 30 (per the brief) [default: 30]
      --json                       Emit JSON instead of a human-readable summary
  -h, --help                       Print help
```

#### `argo skill ab-test`

```text
`argo skill ab-test <name> --variants vN,vM --runs N` — create a new A/B test run pitting two existing versions of `<name>` against each other

Usage: argo skill ab-test [OPTIONS] --variants <VARIANTS> <NAME>

Arguments:
  <NAME>  Skill name

Options:
      --variants <VARIANTS>  Comma-separated variant pair: `<vA>,<vB>`. Both must already exist as `(name, version)` rows
      --runs <RUNS>          Target invocation count before the run is allowed to auto-complete on significance. Default 100 [default: 100]
      --json                 Emit JSON (the new run row) instead of a one-line summary
  -h, --help                 Print help
```

#### `argo skill catalog`

```text
Browse marketplace skills with local-install status marks (mission-cli-skill-catalog). Unlike `skill list` (local-only) and `skill search` (query-driven), this prints every marketplace entry and joins it against the locally-installed set so operators can see at a glance what is installable. `--vertical` is matched against the marketplace `tenant` field (no separate vertical column exists yet on `IndexedSkill`); `--tag` is a substring match against the marketplace entry description

Usage: argo skill catalog [OPTIONS]

Options:
      --vertical <VERTICAL>  Filter to marketplace entries whose tenant (vertical analog) matches this id (case-insensitive). Unknown values surface a `vertical <id> not found` error
      --tag <TAG>            Substring match (case-insensitive) against the marketplace entry description
      --installed-only       Show only locally-installed marketplace entries
      --not-installed        Show only entries that are NOT locally installed
      --all                  Explicit no-filter flag. Present for parity with the brief; the default behaviour is already to show all entries
      --json                 Emit a JSON array `[{ id, name, vertical, tags, installed, version, installed_version? }]` instead of the human-readable table
  -h, --help                 Print help
```

#### `argo skill conflicts`

```text
Inspect + resolve workspace ↔ library skill name collisions surfaced by the H-4b layered lookup (Stream `claude-skill-conflict-resolution`)

Usage: argo skill conflicts <COMMAND>

Commands:
  list     List every workspace ↔ library skill-name collision
  show     Show the unified diff between the two layers' bodies for one collision
  resolve  Resolve a collision by archiving the loser. `--keep workspace` archives the library row; `--keep library` archives the workspace row
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo skill conflicts list`

```text
List every workspace ↔ library skill-name collision

Usage: argo skill conflicts list [OPTIONS]

Options:
      --json  Output as a JSON array
  -h, --help  Print help
```

##### `argo skill conflicts resolve`

```text
Resolve a collision by archiving the loser. `--keep workspace` archives the library row; `--keep library` archives the workspace row

Usage: argo skill conflicts resolve [OPTIONS] --keep <KEEP> <NAME>

Arguments:
  <NAME>  The colliding skill name

Options:
      --keep <KEEP>  Which layer to keep — `workspace` or `library`
      --json         Output as a single JSON object
  -h, --help         Print help
```

##### `argo skill conflicts show`

```text
Show the unified diff between the two layers' bodies for one collision

Usage: argo skill conflicts show [OPTIONS] <NAME>

Arguments:
  <NAME>  The colliding skill name

Options:
      --json  Output as a single JSON object (`{name, workspace, library, diff}`)
  -h, --help  Print help
```

#### `argo skill count`

```text
Print just the count of active skills. Default: one int per line; `--json` emits `{count}`

Usage: argo skill count [OPTIONS]

Options:
      --include-superseded  Include superseded versions in the count too
      --json                
  -h, --help                Print help
```

#### `argo skill create`

```text
Create a new skill from CLI flags or stdin. Inserts a fresh row via `SkillStore::install`; rejects duplicate names (use `argo skill refine` to bump an existing skill instead)

Usage: argo skill create [OPTIONS] <NAME>

Arguments:
  <NAME>  

Options:
      --description <DESCRIPTION>  One-line description for the frontmatter. Defaults to a stub (`<name> — created via argo skill create`)
      --body <BODY>                Skill body text. Mutually exclusive with `--from-file`; without either flag, body is read from stdin until EOF
      --from-file <FROM_FILE>      Read the skill body from a path on disk
      --new-version <NEW_VERSION>  Version label for the new row. Defaults to "0.1" [default: 0.1]
  -h, --help                       Print help
```

#### `argo skill deps`

```text
Show the dependency DAG of one skill: which skills it `argo skill run`s, references via `\`skill:NAME\``, etc. (`claude-skill-dependency-graph`). `--reverse` flips the arrows so you can see "what would break if I rolled back X?" Markdown tree by default; `--json` emits the resolved edges

Usage: argo skill deps [OPTIONS] <NAME>

Arguments:
  <NAME>  Skill name (active version is used)

Options:
      --json                   Emit JSON instead of the markdown tree
      --reverse                Show skills that depend ON this one, not the reverse
      --max-depth <MAX_DEPTH>  Maximum recursion depth (default 5; 0 → default) [default: 5]
  -h, --help                   Print help
```

#### `argo skill distill-now`

```text
Run cohort-based Dreams distillation over recent trajectories and emit a per-cohort summary. Reads `TrajectoryInput` JSON array from stdin (the producer is out of v1 scope — a nightly tick will land later). Brief: `claude-dreams-skill-distillation`

Usage: argo skill distill-now [OPTIONS]

Options:
      --worker <WORKER>  Worker id to charge against the per-day Dreams cap shared with `argo memory dream-now`. Defaults to `skill-distillation` so the operator can run off-band without picking a specific worker [default: skill-distillation]
      --json             Emit a JSON envelope
  -h, --help             Print help
```

#### `argo skill explain`

```text
Explain one skill's provenance, version, Forge, trust, and usage history

Usage: argo skill explain [OPTIONS] <NAME>

Arguments:
  <NAME>  

Options:
      --history-depth <HISTORY_DEPTH>  Limit history sections to N rows [default: 20]
      --json                           Output as a single JSON object for scripting
  -h, --help                           Print help
```

#### `argo skill export`

```text
Export one active skill (or an explicit `name:version`) into a portable `.skill.tar.gz` artifact

Usage: argo skill export [OPTIONS] --out <OUT> <NAME>

Arguments:
  <NAME>  Skill name, optionally suffixed as `name:version`

Options:
      --out <OUT>  Output `.skill.tar.gz` path
      --json       Emit JSON
  -h, --help       Print help
```

#### `argo skill export-all`

```text
Export every active skill into a directory of portable `.skill.tar.gz` artifacts

Usage: argo skill export-all [OPTIONS] --out <OUT>

Options:
      --out <OUT>            Output directory
      --vertical <VERTICAL>  Filter to skills bundled by this vertical id
      --json                 Emit JSON
  -h, --help                 Print help
```

#### `argo skill import`

```text
Import a portable `.skill.tar.gz` artifact into the local skill store

Usage: argo skill import [OPTIONS] <PATH>

Arguments:
  <PATH>  Artifact path

Options:
      --name <NAME>            Rename on import; required to avoid duplicate-name conflicts when the original name is already installed
      --workspace <WORKSPACE>  Resolve the target skill store as if running from this workspace path
      --dry-run                Validate and print what would be installed without writing the store
      --json                   Emit JSON
  -h, --help                   Print help
```

#### `argo skill install`

```text
Usage: argo skill install <SOURCE>

Arguments:
  <SOURCE>  

Options:
  -h, --help  Print help
```

#### `argo skill ir`

```text
Canonical-IR maintenance ops (Track L). One-shot tools the operator runs against the local skill store — currently just `backfill`, which synthesises a minimal `SkillGraph` for every installed skill that lacks one (brief `L3-skill-ir-backfill`)

Usage: argo skill ir <COMMAND>

Commands:
  backfill  One-shot: synthesise a minimal IR (single `action` node with the skill body as instruction) for every installed skill missing an `skill_ir` row. Idempotent — re-running produces zero writes. `--dry-run` reports counts without persisting
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo skill ir backfill`

```text
One-shot: synthesise a minimal IR (single `action` node with the skill body as instruction) for every installed skill missing an `skill_ir` row. Idempotent — re-running produces zero writes. `--dry-run` reports counts without persisting

Usage: argo skill ir backfill [OPTIONS]

Options:
      --dry-run  Walk the store, classify each skill, but write nothing
      --json     Emit a JSON envelope `{backfilled, skipped, entries: [...]}` instead of the human-readable summary
  -h, --help     Print help
```

#### `argo skill keygen`

```text
Generate a cosign-style ed25519 keypair for testing/onboarding

Usage: argo skill keygen [OPTIONS]

Options:
      --out <OUT>  Directory to write `cosign.key` + `cosign.pub` into. Defaults to the current working directory
      --json       Emit a JSON envelope
  -h, --help       Print help
```

#### `argo skill list`

```text
Usage: argo skill list [OPTIONS]

Options:
      --json                           Output as JSON (one object per skill) for scripting. Equivalent to `--format json`
      --format <FORMAT>                Output format: `text` (default), `json`, or `markdown` (renders a markdown table). `--json` wins if both are passed [default: text]
      --sort-by <SORT_BY>              Sort order: `created-at` (default, descending), `name` (lexical, ascending), or `trust` (most-trusted first — Builtin/PublishedVerified ahead of LearnedUnreviewed) [default: created-at]
      --limit <LIMIT>                  Truncate to the first N rows (after sorting)
      --include-superseded             Include superseded versions (default keeps only the active skill per name). Mirrors `?include-superseded`
      --name-contains <NAME_CONTAINS>  Case-insensitive substring filter on `frontmatter.name`. Mirrors `?name-contains=…`
      --lock <LOCK>                    Filter by lock status: `none` / `user-protected` / `content-locked`. Mirrors `?lock=…`
      --body-contains <BODY_CONTAINS>  Case-insensitive substring filter on the skill body. Mirrors `?body-contains=…`
      --quality-min <QUALITY_MIN>      Filter to skills whose latest quality score is at least this value (`0..=100`). Skills with no score row are excluded from the filtered list
  -h, --help                           Print help
```

#### `argo skill list-migrations`

```text
List every pending migration across the tenant's installed skills (for each active skill, the row whose `from_version` matches its installed version). Operator dashboard input. (`mission-skill-version-migration-path`)

Usage: argo skill list-migrations [OPTIONS]

Options:
      --json  Emit a JSON envelope
  -h, --help  Print help
```

#### `argo skill lock`

```text
Usage: argo skill lock <NAME>

Arguments:
  <NAME>  

Options:
  -h, --help  Print help
```

#### `argo skill migrate`

```text
Record a publisher-side `skill_version_migration` row linking `--from` to `--to`. Tenant workspaces with the `--from` version installed will see this when they run `argo skill upgrade <name>` or `argo skill list-migrations`. Idempotent on the composite PK. (`mission-skill-version-migration-path`)

Usage: argo skill migrate [OPTIONS] --from <FROM> --to <TO> <SKILL_ID>

Arguments:
  <SKILL_ID>  Skill id (32-char hex from `argo skill versions --json`, or dashed UUID)

Options:
      --from <FROM>
          Version the migration applies FROM
      --to <TO>
          Version the migration upgrades TO
      --auto-upgrade
          Mark the migration as auto-applicable: tenants running `argo skill upgrade <name>` WITHOUT `--yes` will still apply this row
      --reason <REASON>
          Free-text rationale (truncated on insert)
      --publisher-tenant <PUBLISHER_TENANT>
          Publisher tenant id (UUID). Defaults to a fresh per-call id when omitted; publisher-ownership enforcement is a separate brief
      --json
          Emit a JSON envelope
  -h, --help
          Print help
```

#### `argo skill pack`

```text
`argo skill pack <sub>` tree

Usage: argo skill pack <COMMAND>

Commands:
  lint  Lint a vertical-template pack directory before publish — validates `pack.toml`, Skill IRs, memory schema, eval fixtures, naming, semver, and the 500-LOC file limit
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo skill pack lint`

```text
Lint a vertical-template pack directory before publish — validates `pack.toml`, Skill IRs, memory schema, eval fixtures, naming, semver, and the 500-LOC file limit

Usage: argo skill pack lint [OPTIONS] <PATH>

Arguments:
  <PATH>  Pack root directory to lint

Options:
      --strict  Treat warnings as errors (exit code 2)
      --json    Emit machine-readable JSON instead of the text-formatted finding list
  -h, --help    Print help
```

#### `argo skill publish`

```text
Publish a local skill to the marketplace registry

Usage: argo skill publish [OPTIONS] <NAME>

Arguments:
  <NAME>  Local active skill name to publish

Options:
      --registry <REGISTRY>  Registry reference. `mock://path` writes to a filesystem registry [default: mock://.argo/skill-registry]
      --tag <TAG>            Marketplace tag. Defaults to the source skill version
      --manifest <MANIFEST>  Read a SKILL.md file instead of the active local skill row
      --dry-run              Print the bundle plan without writing files or calling a registry
      --json                 Emit a JSON envelope
  -h, --help                 Print help
```

#### `argo skill quality`

```text
Show the latest quality score (0..=100) + inputs for one skill (`claude-skill-quality-judge` / `claude-skill-quality-score-vis`). With `--limit N` the command renders the last N scores as a history block. The `--format markdown` flag emits the human-readable per-section view; `--json` (or `--format json`) wins for scripting; the legacy text default is preserved when neither is passed for backward compatibility

Usage: argo skill quality [OPTIONS] <NAME>

Arguments:
  <NAME>  Skill name (active version is used)

Options:
      --json             Emit JSON. Equivalent to `--format json`
      --format <FORMAT>  Output format: `text` (default), `markdown`, or `json`. `--json` wins if both are passed [default: text]
      --limit <LIMIT>    Render history of the last N scores rather than only the latest. Default 1 (latest only) [default: 1]
  -h, --help             Print help
```

#### `argo skill quality-list`

```text
List every scored skill, worst first. `--threshold` keeps rows with score ≤ threshold; `--worst` keeps only the lowest-scoring N

Usage: argo skill quality-list [OPTIONS]

Options:
      --threshold <THRESHOLD>  Optional ceiling — only rows with `score <= threshold` are emitted
      --worst <WORST>          Truncate to the lowest-scoring N rows
      --json                   Emit JSON
  -h, --help                   Print help
```

#### `argo skill quality-recompute`

```text
Usage: argo skill quality-recompute [OPTIONS]

Options:
      --skill <SKILL>
          Recompute one named skill. Mutually exclusive with `--all`.
          
          (Sensible deviation from the brief, which specified `<skill_id>`: CLI commands consistently take a skill name, so this flag accepts the active skill's name. The output still prints the underlying `SkillId`.)

      --all
          Recompute every active skill in the local store

      --since <SINCE>
          Only consider audit rows newer than this cutoff. Accepts an RFC-3339 timestamp (`2026-01-01T00:00:00Z`) or a relative duration (`30m`, `24h`, `7d`)

      --dry-run
          Compute + print but do NOT persist the new score

  -h, --help
          Print help (see a summary with '-h')
```

#### `argo skill quality-trend`

```text
ASCII sparkline of the past `--days` daily quality scores for one skill (gaps render as `·`)

Usage: argo skill quality-trend [OPTIONS] <NAME>

Arguments:
  <NAME>  Skill name (active version is used)

Options:
      --days <DAYS>  Days to render (default 30, max 365)
      --json         Emit JSON envelope (per-day buckets + sparkline)
  -h, --help         Print help
```

#### `argo skill rate`

```text
Rate a marketplace-installed skill (operator thumbs -> publisher's skill_rating table; Stream `mission-skill-ratings-publisher`). The skill must be installed from the marketplace (`provenance.source` of the form `published:<grade>:<publisher>`); non-published sources return an explicit "only published skills can be rated" error

Usage: argo skill rate [OPTIONS] --vote <VOTE> <NAME>

Arguments:
  <NAME>  Active local skill name to rate

Options:
      --version <VERSION>  Skill version label being rated. Defaults to the active version of `name` if omitted
      --vote <VOTE>        `up` or `down`
      --comment <COMMENT>  Optional free-form comment (truncated at 500 chars)
      --json               Emit a JSON envelope
  -h, --help               Print help
```

#### `argo skill ratings`

```text
Show aggregated up/down counts + score + recent comments for one marketplace-installed skill (Stream `mission-skill-ratings-publisher`). Counts roll up across every rater tenant; `comments` are the most-recent 10

Usage: argo skill ratings [OPTIONS] <NAME>

Arguments:
  <NAME>  Skill name

Options:
      --version <VERSION>  Skill version. Defaults to the active version
      --json               Emit JSON
  -h, --help               Print help
```

#### `argo skill refine`

```text
Append a hint to the active skill's body and supersede it with a fresh version. Manual mutator — the score-driven version (run bench, accept only when the new score wins) lands once `forge optimize` learns to mutate

Usage: argo skill refine [OPTIONS] --hint <HINT> <NAME>

Arguments:
  <NAME>  

Options:
      --hint <HINT>                Text to append to the skill body, under a new `## Refinement` section
      --new-version <NEW_VERSION>  Version label for the refined skill. Defaults to bumping the existing version's last numeric segment (e.g. `0.1` → `0.2`). `--version` would clash with clap's auto-version flag, hence the rename
  -h, --help                       Print help
```

#### `argo skill review`

```text
Drive the learned-skill review gate (#681 lifecycle): list pending proposals + approve / reject them. Without this surface the lifecycle gate is theoretical — agents can propose skills via `propose_for_review`, but operators have no way to act on the queue

Usage: argo skill review <COMMAND>

Commands:
  list     List skills awaiting human review (lifecycle_state = pending_review). Default text mode prints id/name/ vertical/learned-in/created-at; `--json` emits a machine-readable envelope
  show     Show one pending-review skill's body, trajectory, provenance, and originating session
  approve  Approve a pending-review skill — flips lifecycle_state to active. The next agent turn that consults the skill lookup will see it
  reject   Reject a pending-review skill — flips lifecycle_state to rejected. The skill row stays in the table for audit but the lookup never returns it again
  promote  Promote a memory-backed skill candidate into a real `pending_review` skill row (#697). Reads the candidate from `MEMORY.md` (vertical-namespaced; isolation preserved), constructs a SKILL.md from its fields, and installs it via `propose_for_review`. The skill remains `pending_review` until the operator runs `argo skill review approve <new-skill-id>` — promote never auto-activates. Emits a `skill.candidate.promoted` audit row
  export   Export a stored skill to an offline distribution file
  import   Import a skill distribution file into the local skill store
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo skill review approve`

```text
Approve a pending-review skill — flips lifecycle_state to active. The next agent turn that consults the skill lookup will see it

Usage: argo skill review approve [OPTIONS] <SKILL_ID>

Arguments:
  <SKILL_ID>  The skill's id (UUID v7)

Options:
      --note <NOTE>  Optional reviewer note recorded in the audit row
      --json         
  -h, --help         Print help
```

##### `argo skill review export`

```text
Export a stored skill to an offline distribution file

Usage: argo skill review export [OPTIONS] --output <OUTPUT> <SKILL_ID>

Arguments:
  <SKILL_ID>  The skill's id (UUID v7)

Options:
      --output <OUTPUT>  Destination file path
      --format <FORMAT>  Output format: md (default), toml, or json [default: md]
  -h, --help             Print help
```

##### `argo skill review import`

```text
Import a skill distribution file into the local skill store

Usage: argo skill review import [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to a md, toml, or json skill distribution

Options:
      --lock <LOCK>  Optional lock override: none, content_locked, or user_protected
  -h, --help         Print help
```

##### `argo skill review list`

```text
List skills awaiting human review (lifecycle_state = pending_review). Default text mode prints id/name/ vertical/learned-in/created-at; `--json` emits a machine-readable envelope

Usage: argo skill review list [OPTIONS]

Options:
      --limit <LIMIT>        Truncate both persisted pending skills and memory-backed candidates after filtering
      --vertical <VERTICAL>  Restrict the review queue to one vertical id
      --json                 
  -h, --help                 Print help
```

##### `argo skill review promote`

```text
Promote a memory-backed skill candidate into a real `pending_review` skill row (#697). Reads the candidate from `MEMORY.md` (vertical-namespaced; isolation preserved), constructs a SKILL.md from its fields, and installs it via `propose_for_review`. The skill remains `pending_review` until the operator runs `argo skill review approve <new-skill-id>` — promote never auto-activates. Emits a `skill.candidate.promoted` audit row

Usage: argo skill review promote [OPTIONS] <MEMORY_ID>

Arguments:
  <MEMORY_ID>  The memory entry's UUID. Use `argo skill review list --json` to discover candidate ids

Options:
      --feedback-action <ACTION_ID>  Require the latest operator feedback for this action to be an up-vote before promoting the candidate
      --json                         
  -h, --help                         Print help
```

##### `argo skill review reject`

```text
Reject a pending-review skill — flips lifecycle_state to rejected. The skill row stays in the table for audit but the lookup never returns it again

Usage: argo skill review reject [OPTIONS] <SKILL_ID>

Arguments:
  <SKILL_ID>  The skill's id

Options:
      --note <NOTE>  Optional reviewer note recorded in the audit row
      --json         
  -h, --help         Print help
```

##### `argo skill review show`

```text
Show one pending-review skill's body, trajectory, provenance, and originating session

Usage: argo skill review show [OPTIONS] <SKILL_ID>

Arguments:
  <SKILL_ID>  The skill's id (UUID v7)

Options:
      --json  
  -h, --help  Print help
```

#### `argo skill rollback`

```text
Promote a prior version back to active

Usage: argo skill rollback [OPTIONS] --to <TO> <NAME>

Arguments:
  <NAME>  Skill name

Options:
      --to <TO>          Target version to promote
      --dry-run          Preview the transition without changing rows
      --reason <REASON>  Optional audit reason
      --json             Emit JSON
  -h, --help             Print help
```

#### `argo skill runs`

```text
Show chronological skill invocation history

Usage: argo skill runs [OPTIONS] [skill-name]

Arguments:
  [skill-name]  Skill name. Omit when using `--tenant` for cross-skill history

Options:
      --tenant <TENANT>  Cross-skill tenant view
      --worker <WORKER>  Filter to one worker id
      --since <SINCE>    Wall-clock relative filter such as `24h`, `30m`, or `7d`
      --limit <LIMIT>    Maximum number of rows to return [default: 20]
      --include-failed   Include failed and skipped invocations. Defaults to successes only
      --json             Emit a JSON envelope
  -h, --help             Print help
```

#### `argo skill search`

```text
Search the cross-tenant OCI skill registry

Usage: argo skill search [OPTIONS] [QUERY]

Arguments:
  [QUERY]  Search terms matched against skill name, description, tenant, and OCI ref. Optional when `--vertical <id>` is set — listing all skills in a vertical doesn't need a search term [default: ""]

Options:
      --trust-min <TRUST_MIN>     Minimum trust grade: builtin, published_verified, tenant, published_community, or learned_unreviewed
      --tenant <TENANT>           Restrict results to one tenant or publisher namespace
      --limit <LIMIT>             Maximum number of ranked rows to return [default: 20]
      --json                      Output a JSON array
      --vertical <VERTICAL>       Filter to skills whose `vertical_id` column matches the given vertical id. Switches to a store-side query (mission claim 4 — skills as a per-vertical library)
      --include-used-by-sessions  Pair with `--vertical <id>`: also surface skills attached to sessions in that vertical (via `session_skill`), regardless of how the skill row tagged itself. No-op without `--vertical`
  -h, --help                      Print help
```

#### `argo skill set-flag`

```text
Recompute the quality score for one or every active skill from the audit log (mission-skill-quality-cron). For each skill: walks `tool_result` audit rows, derives the four `QualityInputs`, runs the existing `compute_score` primitive, and persists the result via the standard skill-quality store. Output is one line per skill: `<skill_id> <old_score> -> <new_score> (<delta>)`. Designed to be cron-driven; `--dry-run` is the rehearsal switch. Set a per-skill operator flag (mission-forge-operator-feedback-eval-signal). Today the only consumer is `argo forge` — the `--no-forge-feedback` flag tells the Forge ingestion path to skip operator-vote eval cases for this skill. The underlying `skill_flag` table is generic, so future per-skill toggles piggyback on the same surface

Usage: argo skill set-flag [OPTIONS] <NAME>

Arguments:
  <NAME>  Skill name to flag

Options:
      --no-forge-feedback        Set `no-forge-feedback=1` for this skill so the Forge ingestion path skips its operator votes
      --clear-no-forge-feedback  Clear `no-forge-feedback` for this skill (set value=0)
      --json                     Emit a JSON envelope `{skill, flag, value}` instead of the human-readable confirmation line
  -h, --help                     Print help
```

#### `argo skill sign`

```text
Sign a local skill manifest with a cosign-compatible ed25519 key

Usage: argo skill sign [OPTIONS] [NAME]

Arguments:
  [NAME]  Active local skill name to sign. Omit when `--manifest` is set

Options:
      --key <KEY>            PEM private key file produced by `argo skill keygen`. Required for v1; `--keyless` is deferred follow-up
      --keyless              Sigstore Fulcio keyless signing (deferred to follow-up)
      --manifest <MANIFEST>  Read SKILL.md bytes from a path instead of the active skill row
      --tag <TAG>            Override the published tag the signature is filed under (defaults to the skill's frontmatter `version`)
      --registry <REGISTRY>  Mock OCI registry reference (must match `argo skill publish`) [default: mock://.argo/skill-registry]
      --json                 Emit a JSON envelope
  -h, --help                 Print help
```

#### `argo skill tags`

```text
List marketplace tags for one skill

Usage: argo skill tags [OPTIONS] <NAME>

Arguments:
  <NAME>  Skill name whose marketplace tags should be listed

Options:
      --registry <REGISTRY>  Registry reference. `mock://path` reads a filesystem registry [default: mock://.argo/skill-registry]
      --limit <LIMIT>        Maximum number of tags to list [default: 20]
      --json                 Emit a JSON envelope
  -h, --help                 Print help
```

#### `argo skill unlock`

```text
Usage: argo skill unlock <NAME>

Arguments:
  <NAME>  

Options:
  -h, --help  Print help
```

#### `argo skill unpublish`

```text
Remove a mutable marketplace tag while preserving blobs

Usage: argo skill unpublish [OPTIONS] <REFERENCE>

Arguments:
  <REFERENCE>  Marketplace reference in `<name>:<tag>` form

Options:
      --registry <REGISTRY>  Registry reference. `mock://path` reads a filesystem registry [default: mock://.argo/skill-registry]
      --json                 Emit a JSON envelope
  -h, --help                 Print help
```

#### `argo skill update`

```text
Mutate metadata on an active skill in place. Currently only the skill-side trust gate (`--min-trust-grade`) is wired — future flag pass-throughs extend this command instead of adding fresh subcommands. M6 precursor 1/3 (Stream `mission-skill-min-trust-grade-column`)

Usage: argo skill update [OPTIONS] <NAME>

Arguments:
  <NAME>  Skill name (active version is mutated)

Options:
      --min-trust-grade <MIN_TRUST_GRADE>
          Minimum [`argo_skill::trust::TrustGrade`] required to surface the skill for a worker. Accepts the snake_case JSON serialization (`builtin`, `published_verified`, `tenant`, `published_community`, `learned_unreviewed`) or the literal `none` / `null` to clear the column back to "no restriction" (the v1 default). Without this flag the command is a no-op so future flags compose cleanly
      --json
          Emit JSON `{name, min_trust_grade}` instead of the human-readable line
  -h, --help
          Print help
```

#### `argo skill upgrade`

```text
Apply a pending `skill_version_migration` to the locally- installed version of `<name>`. With `--dry-run`, prints what would happen without superseding. Without `--yes`, only auto-upgrade migrations are applied (others are surfaced for operator confirmation). (`mission-skill-version-migration-path`)

Usage: argo skill upgrade [OPTIONS] <NAME>

Arguments:
  <NAME>  Skill name (the active version's `from_version` is the lookup key)

Options:
      --dry-run  Print what would change without writing
      --yes      Apply even when the migration row's `auto_upgrade=false`
      --json     Emit a JSON envelope
  -h, --help     Print help
```

#### `argo skill verify`

```text
Verify a skill's body matches its frontmatter `provenance.hash`. Pass `--all` to verify every active skill in the workspace

Usage: argo skill verify [OPTIONS] [NAME]

Arguments:
  [NAME]  

Options:
      --all   Verify every active skill instead of one named skill
  -h, --help  Print help
```

#### `argo skill verify-sig`

```text
Verify a published skill against its detached cosign signature. Uses a hyphenated subcommand name because `verify` is already occupied by the body/provenance hash check (`SkillCmd::Verify`)

Usage: argo skill verify-sig [OPTIONS] <REFERENCE>

Arguments:
  <REFERENCE>  Published reference as `<name>:<tag>` (e.g. `foo:0.1.0`)

Options:
      --key <KEY>            PEM public key file. Required until keyless verify lands
      --registry <REGISTRY>  Mock OCI registry reference [default: mock://.argo/skill-registry]
      --json                 Emit a JSON envelope
  -h, --help                 Print help
```

#### `argo skill versions`

```text
`argo skill versions <name>` — list every version of `<name>` installed in this workspace, newest first

Usage: argo skill versions [OPTIONS] <NAME>

Arguments:
  <NAME>  Skill name (the family identifier, NOT a version string)

Options:
      --json  Emit JSON (one object per row) instead of a table
  -h, --help  Print help
```

#### `argo skill view`

```text
Usage: argo skill view [OPTIONS] <NAME>

Arguments:
  <NAME>  

Options:
      --json  Output as a single JSON object for scripting
  -h, --help  Print help
```

### `argo sla`

```text
Availability/SLA dashboards backed by canonical audit rows

Usage: argo sla <COMMAND>

Commands:
  report  Availability % + latency p50/p95/p99 from `audit_record` request rows
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo sla report`

```text
Availability % + latency p50/p95/p99 from `audit_record` request rows

Usage: argo sla report [OPTIONS]

Options:
      --window <WINDOW>  Time window to inspect, e.g. `30m`, `1h`, or `24h` (default) [default: 24h]
      --tenant <TENANT>  Optional tenant id filter
      --json             Emit a JSON envelope instead of a table
  -h, --help             Print help
```

### `argo steer`

```text
Append an out-of-band user message to a live session. Used to nudge a running agent without waiting for the next prompt

Usage: argo steer <SESSION_ID> <TEXT>

Arguments:
  <SESSION_ID>  
  <TEXT>        

Options:
  -h, --help  Print help

Examples:
  argo steer 01J9XYZ... "pause and summarise progress"
```

### `argo storage`

```text
Usage: argo storage <COMMAND>

Commands:
  list-tenants    List tenant-specific database pools and their health
  migrate-tenant  Run migrations for one tenant-specific database
  health          Check tenant-specific database health
  help            Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo storage health`

```text
Check tenant-specific database health

Usage: argo storage health [OPTIONS]

Options:
      --json  Output as JSON
  -h, --help  Print help
```

#### `argo storage list-tenants`

```text
List tenant-specific database pools and their health

Usage: argo storage list-tenants [OPTIONS]

Options:
      --json  Output as JSON
  -h, --help  Print help
```

#### `argo storage migrate-tenant`

```text
Run migrations for one tenant-specific database

Usage: argo storage migrate-tenant [OPTIONS] <TENANT_ID>

Arguments:
  <TENANT_ID>  

Options:
      --json  Output as JSON
  -h, --help  Print help
```

### `argo subtask`

```text
Inspect the persistent subtask ledger (Stream `claude-subtask-ledger-store`). One row per node the Planner emits during decomposition; durable across restarts so operators can read a worker's plan after the fact

Usage: argo subtask <COMMAND>

Commands:
  list    List subtask rows. Filter by `--session`, `--worker`, `--parent-goal`, or `--state`. Newest-first, capped at `--limit` (default 50)
  show    Show one row in full
  tree    Render the subtask tree for one parent session id, oldest row first. Useful for "what plan did the worker generate for session X?"
  budget  Render the per-delegation budget tree rooted at a subtask id (Stream `mission-delegation-budget-tree`). Aggregated spend rolls up to each node via the recursive walk_tree CTE; declared budget is taken from each row's `budget_usd_cents` (NULL = unbounded)
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo subtask budget`

```text
Render the per-delegation budget tree rooted at a subtask id (Stream `mission-delegation-budget-tree`). Aggregated spend rolls up to each node via the recursive walk_tree CTE; declared budget is taken from each row's `budget_usd_cents` (NULL = unbounded)

Usage: argo subtask budget [OPTIONS] <ROOT_ID>

Arguments:
  <ROOT_ID>  Root subtask id whose descendants to walk

Options:
      --json  Output as a single JSON object instead of a text tree
  -h, --help  Print help
```

#### `argo subtask list`

```text
List subtask rows. Filter by `--session`, `--worker`, `--parent-goal`, or `--state`. Newest-first, capped at `--limit` (default 50)

Usage: argo subtask list [OPTIONS]

Options:
      --session <SESSION>          Restrict to one parent session id
      --worker <WORKER>            Restrict to one worker id
      --parent-goal <PARENT_GOAL>  Restrict to an exact parent goal string
      --state <STATE>              Restrict to one lifecycle state: `pending`, `running`, `done`, `failed`, `skipped`
      --limit <LIMIT>              Cap on returned rows. Default 50, hard cap 500 [default: 50]
      --json                       Output the JSON envelope (`{rows, count, filters}`) instead of a text table
  -h, --help                       Print help
```

#### `argo subtask show`

```text
Show one row in full

Usage: argo subtask show [OPTIONS] <ID>

Arguments:
  <ID>  Subtask id (UUID)

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

#### `argo subtask tree`

```text
Render the subtask tree for one parent session id, oldest row first. Useful for "what plan did the worker generate for session X?"

Usage: argo subtask tree [OPTIONS] <SESSION>

Arguments:
  <SESSION>  Parent session id

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

### `argo team`

```text
Org-chart-aware teams: the operator-facing bridge between `argo org` (the tree) and `argo roster` (group policy) (Stream `claude-worker-team-roster-cli`). Each `team` is a single id that resolves uniquely in both `org_unit` (name + parent link) and `roster` (default policy bundle)

Usage: argo team <COMMAND>

Commands:
  list    List all teams in a tenant + active member counts
  show    Show one team's metadata and active members
  add     Add a worker to a team. Idempotent: re-adding an active member updates only the role; re-adding a removed member reactivates the row
  remove  Soft-delete a worker's membership. Sets `removed_at` so audit / KPI history rows that reference the (team, worker) pair stay valid
  rename  Rename a team. Updates both the org-chart row and the matching roster row
  create  Create a new team (org-chart unit + matching roster row)
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo team add`

```text
Add a worker to a team. Idempotent: re-adding an active member updates only the role; re-adding a removed member reactivates the row

Usage: argo team add [OPTIONS] --tenant <TENANT> <TEAM_ID> <WORKER_ID>

Arguments:
  <TEAM_ID>    Team id (uuid)
  <WORKER_ID>  Worker id (uuid) to add

Options:
      --tenant <TENANT>  Tenant uuid this team belongs to
      --role <ROLE>      Optional free-form role label (`lead`, `clerk`, …)
      --json             Emit `{ok, team_id, worker_id, role}` as JSON
  -h, --help             Print help
```

#### `argo team create`

```text
Create a new team (org-chart unit + matching roster row)

Usage: argo team create [OPTIONS] --tenant <TENANT> <NAME>

Arguments:
  <NAME>  Operator-friendly team name (must be unique per tenant)

Options:
      --tenant <TENANT>  Tenant uuid this team belongs to
      --parent <PARENT>  Optional parent team id; the parent must already exist
      --json             Emit `{ok, team_id, name, parent}` as JSON
  -h, --help             Print help
```

#### `argo team list`

```text
List all teams in a tenant + active member counts

Usage: argo team list [OPTIONS] --tenant <TENANT>

Options:
      --tenant <TENANT>  Tenant uuid this team set belongs to
      --format <FORMAT>  Output format: `markdown` (default) or `json` [default: markdown]
  -h, --help             Print help
```

#### `argo team remove`

```text
Soft-delete a worker's membership. Sets `removed_at` so audit / KPI history rows that reference the (team, worker) pair stay valid

Usage: argo team remove [OPTIONS] --tenant <TENANT> <TEAM_ID> <WORKER_ID>

Arguments:
  <TEAM_ID>    Team id (uuid)
  <WORKER_ID>  Worker id (uuid) to remove

Options:
      --tenant <TENANT>  Tenant uuid this team belongs to
      --json             Emit `{ok, team_id, worker_id}` as JSON
  -h, --help             Print help
```

#### `argo team rename`

```text
Rename a team. Updates both the org-chart row and the matching roster row

Usage: argo team rename [OPTIONS] --tenant <TENANT> <TEAM_ID> <NEW_NAME>

Arguments:
  <TEAM_ID>   Team id (uuid)
  <NEW_NAME>  New display name

Options:
      --tenant <TENANT>  Tenant uuid this team belongs to
      --json             Emit `{ok, team_id, name}` as JSON
  -h, --help             Print help
```

#### `argo team show`

```text
Show one team's metadata and active members

Usage: argo team show [OPTIONS] --tenant <TENANT> <TEAM_ID>

Arguments:
  <TEAM_ID>  Team id (uuid)

Options:
      --tenant <TENANT>  Tenant uuid this team belongs to
      --format <FORMAT>  Output format: `markdown` (default) or `json` [default: markdown]
  -h, --help             Print help
```

### `argo tenant`

```text
Usage: argo tenant <COMMAND>

Commands:
  list        List tenants known to this workspace store
  info        Show tenant workers, sessions, and recent activity
  show        Show tenant metadata, roster, activity, channels, and rules
  create      Register a new tenant name in the current workspace store
  rename      Rename a tenant id, or use --to to rename its display name
  archive     Soft-delete a tenant from default listings while preserving audit history
  transfer    Change the operator of record for a tenant
  quota       Per-tenant resource quotas and period-to-date headroom
  rate-limit  Per-tenant gateway rate-limit overrides (Stream `mission-per-tenant-rate-limiting`). Missing override → use the `[gateway.rate_limit.tenant]` config default
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo tenant archive`

```text
Soft-delete a tenant from default listings while preserving audit history

Usage: argo tenant archive [OPTIONS] <ID>

Arguments:
  <ID>  Tenant id (UUID)

Options:
      --json  Emit JSON instead of a human-readable summary
  -h, --help  Print help
```

#### `argo tenant create`

```text
Register a new tenant name in the current workspace store

Usage: argo tenant create [OPTIONS] <NAME>

Arguments:
  <NAME>  Human-readable tenant/workspace name

Options:
      --json  Emit JSON instead of a human-readable summary
  -h, --help  Print help
```

#### `argo tenant info`

```text
Show tenant workers, sessions, and recent activity

Usage: argo tenant info [OPTIONS] <ID>

Arguments:
  <ID>  Tenant id (UUID)

Options:
      --json  Emit JSON instead of a human-readable summary
  -h, --help  Print help
```

#### `argo tenant list`

```text
List tenants known to this workspace store

Usage: argo tenant list [OPTIONS]

Options:
      --include-archived  Include tenants that were archived for audit retention
      --json              Emit JSON instead of a human-readable table
  -h, --help              Print help
```

#### `argo tenant quota`

```text
Per-tenant resource quotas and period-to-date headroom

Usage: argo tenant quota <COMMAND>

Commands:
  set    Set or update quota caps for one tenant
  show   Show quota, usage, and remaining headroom for one tenant
  list   List all configured tenant quotas
  clear  Clear a tenant quota row or a single quota field
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo tenant quota clear`

```text
Clear a tenant quota row or a single quota field

Usage: argo tenant quota clear [OPTIONS] <TENANT>

Arguments:
  <TENANT>
          Tenant id

Options:
      --field <FIELD>
          Optional field to clear. Omit to remove the whole quota row

          Possible values:
          - max-usd:     Clear the USD cap
          - max-actions: Clear the action-count cap
          - max-workers: Clear the worker-count cap

      --json
          Emit a JSON envelope

  -h, --help
          Print help (see a summary with '-h')
```

##### `argo tenant quota list`

```text
List all configured tenant quotas

Usage: argo tenant quota list [OPTIONS]

Options:
      --json  Emit a JSON envelope
  -h, --help  Print help
```

##### `argo tenant quota set`

```text
Set or update quota caps for one tenant

Usage: argo tenant quota set [OPTIONS] <TENANT>

Arguments:
  <TENANT>
          Tenant id

Options:
      --max-usd <MAX_USD>
          Maximum period spend in USD

      --max-actions <MAX_ACTIONS>
          Maximum period action count

      --max-workers <MAX_WORKERS>
          Maximum active worker count

      --period <PERIOD>
          Quota window for spend and action counts

          Possible values:
          - daily:   Daily quota window
          - monthly: Monthly quota window

      --json
          Emit a JSON envelope

  -h, --help
          Print help (see a summary with '-h')
```

##### `argo tenant quota show`

```text
Show quota, usage, and remaining headroom for one tenant

Usage: argo tenant quota show [OPTIONS] <TENANT>

Arguments:
  <TENANT>  Tenant id

Options:
      --json  Emit a JSON envelope
  -h, --help  Print help
```

#### `argo tenant rate-limit`

```text
Per-tenant gateway rate-limit overrides (Stream `mission-per-tenant-rate-limiting`). Missing override → use the `[gateway.rate_limit.tenant]` config default

Usage: argo tenant rate-limit <COMMAND>

Commands:
  set     Set or replace one tenant's per-minute budget
  get     Show one tenant's override, if any
  list    List every stored override
  delete  Clear one tenant's override; falls back to the config default
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo tenant rate-limit delete`

```text
Clear one tenant's override; falls back to the config default

Usage: argo tenant rate-limit delete [OPTIONS] <TENANT>

Arguments:
  <TENANT>  Tenant id (UUID)

Options:
      --json  Emit JSON
  -h, --help  Print help
```

##### `argo tenant rate-limit get`

```text
Show one tenant's override, if any

Usage: argo tenant rate-limit get [OPTIONS] <TENANT>

Arguments:
  <TENANT>  Tenant id (UUID)

Options:
      --json  Emit JSON
  -h, --help  Print help
```

##### `argo tenant rate-limit list`

```text
List every stored override

Usage: argo tenant rate-limit list [OPTIONS]

Options:
      --json  Emit JSON
  -h, --help  Print help
```

##### `argo tenant rate-limit set`

```text
Set or replace one tenant's per-minute budget

Usage: argo tenant rate-limit set [OPTIONS] --rpm <RPM> <TENANT>

Arguments:
  <TENANT>  Tenant id (UUID)

Options:
      --rpm <RPM>  Per-minute budget (`> 0`)
      --json       Emit JSON instead of a one-line summary
  -h, --help       Print help
```

#### `argo tenant rename`

```text
Rename a tenant id, or use --to to rename its display name

Usage: argo tenant rename [OPTIONS] <ID> [NEW_ID]

Arguments:
  <ID>      Tenant id (UUID)
  [NEW_ID]  New stable tenant id (UUID)

Options:
      --to <NEW_NAME>  New human-readable tenant name (legacy display-name rename)
      --json           Emit JSON instead of a human-readable summary
  -h, --help           Print help
```

#### `argo tenant show`

```text
Show tenant metadata, roster, activity, channels, and rules

Usage: argo tenant show [OPTIONS] <ID>

Arguments:
  <ID>  Tenant id (UUID)

Options:
      --json  Emit JSON instead of a human-readable summary
  -h, --help  Print help
```

#### `argo tenant transfer`

```text
Change the operator of record for a tenant

Usage: argo tenant transfer [OPTIONS] --owner <NEW_OPERATOR_EMAIL> <ID>

Arguments:
  <ID>  Tenant id (UUID)

Options:
      --owner <NEW_OPERATOR_EMAIL>  New operator email
      --json                        Emit JSON instead of a human-readable summary
  -h, --help                        Print help
```

### `argo tools`

```text
List the in-core tools the agent can call. Mirrors `GET /tools`

Usage: argo tools [COMMAND]

Commands:
  list  List in-core tools
  show  Show one tool's full JSON-schema parameter shape by name
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help

Examples:
  argo tools
  argo tools list --format markdown
  argo tools show web_search --json
```

#### `argo tools list`

```text
List in-core tools

Usage: argo tools list [OPTIONS]

Options:
      --json                           Equivalent to `--format json`
      --format <FORMAT>                Output format: `text` (default), `json`, or `markdown` [default: text]
      --name-contains <NAME_CONTAINS>  Case-insensitive substring filter on tool name
  -h, --help                           Print help

Examples:
  argo tools list
  argo tools list --format markdown
  argo tools list --name-contains search
```

#### `argo tools show`

```text
Show one tool's full JSON-schema parameter shape by name

Usage: argo tools show [OPTIONS] <NAME>

Arguments:
  <NAME>  

Options:
      --json  Output as a single JSON object for scripting
  -h, --help  Print help

Examples:
  argo tools show web_search
  argo tools show web_search --json
```

### `argo trace`

```text
Inspect session traces. Without a subcommand, renders the audit-log timeline (`--session`) or one turn's span tree (`--turn`); with `explore <session>`, walks the local OTLP NDJSON sink and renders a markdown or JSON tree

Usage: argo trace [OPTIONS] [COMMAND]

Commands:
  explore  Walk a session's local OTLP trace tree from `.skycot/traces/<session>.ndjson` and render it as markdown (default) or nested JSON. Read-only consumer of the local NDJSON sink emitted alongside the OTLP/HTTP exporter (#938)
  help     Print this message or the help of the given subcommand(s)

Options:
      --turn <TURN>        
      --session <SESSION>  
  -h, --help               Print help

Examples:
  argo trace --session 01J9XYZ...
  argo trace --turn 01J9TURN...
  argo trace explore 01J9XYZ... --format json
```

#### `argo trace explore`

```text
Walk a session's local OTLP trace tree from `.skycot/traces/<session>.ndjson` and render it as markdown (default) or nested JSON. Read-only consumer of the local NDJSON sink emitted alongside the OTLP/HTTP exporter (#938)

Usage: argo trace explore [OPTIONS] <SESSION>

Arguments:
  <SESSION>  Session id (matches the NDJSON filename stem)

Options:
      --format <FORMAT>  Output format: `markdown` (default) or `json` [default: markdown]
      --path <PATH>      Override the default `.skycot/traces/` lookup directory
  -h, --help             Print help
```

### `argo training`

```text
Export successful action trajectories as fine-tuning JSONL

Usage: argo training <COMMAND>

Commands:
  export  Export completed action trajectories for one vertical
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo training export`

```text
Export completed action trajectories for one vertical

Usage: argo training export [OPTIONS] --vertical <VERTICAL> --start-date <START_DATE>

Options:
      --vertical <VERTICAL>      Vertical id to export
      --start-date <START_DATE>  Inclusive lower bound: YYYY-MM-DD or RFC3339
      --format <FORMAT>          Output format. Only jsonl is supported [default: jsonl]
      --output <OUTPUT>          Write JSONL to this path instead of stdout
      --json                     Print an export summary envelope instead of the JSONL bytes
  -h, --help                     Print help
```

### `argo trajectory`

```text
Export action trajectories for fine-tuning data pipelines (claude-trajectory-training-export)

Usage: argo trajectory <COMMAND>

Commands:
  export  Walk every action bundle under `.skycot/outputs/` and emit the captured trajectories in one of three formats: * `anthropic` — `[{role, content, tool_use?}, ...]` * `openai`    — `[{role, content, tool_calls?}, ...]` * `jsonl`     — one raw trajectory object per line
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo trajectory export`

```text
Walk every action bundle under `.skycot/outputs/` and emit the captured trajectories in one of three formats: * `anthropic` — `[{role, content, tool_use?}, ...]` * `openai`    — `[{role, content, tool_calls?}, ...]` * `jsonl`     — one raw trajectory object per line

Use `--filter passed` to drop trajectories whose action state isn't `completed`. Pass `--include-screenshots` to bundle the per-step PNGs inline (base64) in the output; without it the trajectory carries `screenshot_relpath` pointers only.

Usage: argo trajectory export [OPTIONS]

Options:
      --since <SINCE>
          RFC3339 timestamp or `YYYY-MM-DD` (UTC midnight) — drop trajectories whose `created_at` is older than this

      --until <UNTIL>
          RFC3339 timestamp or `YYYY-MM-DD` (UTC end-of-day) — drop trajectories whose `created_at` is newer than this

      --format <FORMAT>
          Output format. `anthropic` (default) / `openai` / `jsonl`
          
          [default: anthropic]

      --filter <FILTER>
          Trajectory filter. `passed` (default — only `completed` actions), `all` (every state including failed/blocked)
          
          [default: passed]

      --include-screenshots
          Inline per-step screenshots as base64-encoded PNG. Off by default — the path-manifest stays in `screenshot_relpath` so callers can fetch lazily

      --gdpr-redact-emails
          Apply the same email-redaction rules as `argo audit export --gdpr-redact-emails`

      --output <OUTPUT>
          Write the export to this path. When omitted, prints to stdout

      --json
          Emit a JSON envelope to stdout describing the export (rows, format, sha256). The export bytes still go to `--output` (or stdout if `--output` was omitted)

  -h, --help
          Print help (see a summary with '-h')
```

### `argo trust`

```text
Cross-tenant trust-graph queries (Stream `mission-cross-tenant-trust-graph`, MISSION_BACKLOG Section 3 Claim 4 #81). `argo trust graph --from <t>` prints outgoing edges; `argo trust composed --from <a> --to <c>` prints the composed score and BFS path

Usage: argo trust <COMMAND>

Commands:
  graph     Print neighbors (outgoing trust edges) for a tenant
  composed  Print the composed trust score from `--from` to `--to` plus the winning BFS path
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo trust composed`

```text
Print the composed trust score from `--from` to `--to` plus the winning BFS path

Usage: argo trust composed [OPTIONS] --from <FROM> --to <TO>

Options:
      --from <FROM>            Trusting tenant id
      --to <TO>                Trusted tenant id
      --max-hops <MAX_HOPS>    Maximum hops to compose across. Default 3; values above 3 are honoured but discouraged (signal decays) [default: 3]
      --days-back <DAYS_BACK>  How many days of observations to materialise the graph from. Default 90 [default: 90]
      --json                   Emit one JSON `{from, to, max_hops, score, path}` object instead of the human-readable status line
  -h, --help                   Print help
```

#### `argo trust graph`

```text
Print neighbors (outgoing trust edges) for a tenant

Usage: argo trust graph [OPTIONS] --from <FROM>

Options:
      --from <FROM>            Tenant id to print outgoing edges for
      --days-back <DAYS_BACK>  How many days of observations to materialise the graph from. Default 90 — matches the brief's decay window [default: 90]
      --json                   Emit one JSON array of `{to_tenant, weight, observation_count, last_updated_at_rfc3339}` rows instead of the human-readable table
  -h, --help                   Print help
```

### `argo tui`

```text
Open the interactive TUI (read-only session viewer for now)

Usage: argo tui

Options:
  -h, --help  Print help

Examples:
  argo tui
```

### `argo update`

```text
Placeholder for an in-place upgrade command. Not yet wired in this build — see `argo self-update` for the working signed-release upgrade flow

Usage: argo update

Options:
  -h, --help  Print help

Examples:
  argo update                # stub today, exits non-zero
  argo self-update --check   # use this instead
```

### `argo vertical`

```text
Vertical-template discovery surface (#650). Read-only — `list`, `show`, `validate`. Install / eval / run land in later PRs

Usage: argo vertical <COMMAND>

Commands:
  list       List loaded verticals discovered from `./verticals` (or the path in `$ARGO_VERTICALS_DIR`). Prints id / name / version per row, plus any per-vertical load errors
  archive    Soft-delete a vertical so it no longer appears in default listings and cannot bind new sessions or workers
  unarchive  Re-activate a previously archived vertical
  delete     Permanently remove an archived vertical from disk
  show       Show one vertical's manifest summary by id
  describe   Render a one-paragraph summary of an installed vertical, or the full manifest summary with `--json` (persona, action list, executor matrix, eval seeds, bundled skills — same shape as `show --json`)
  validate   Re-run the cross-reference validator on one vertical and exit non-zero on any error. Mirrors what the loader does at startup; useful as a CI gate while authoring a vertical
  eval       Walk every `[evaluation].suites` JSONL file declared in the manifest, validate each fixture row is well-formed, and tabulate fixture counts per suite. Useful as a CI gate that the suite hasn't drifted into bad shape (#656)
  workflow   Run a named workflow declared by a vertical (#659). Walks the workflow's steps in declaration order. Today only `tool_call` steps run end-to-end — `skill_invoke` and `prompt` need an LLM transport and are recorded with `status="deferred_no_llm"` so the operator can see what would have run. Capability + per-vertical deny lists are enforced. Pass `--session <id>` (#668) to bind the workflow's tool calls to an existing session's audit chain so `argo audit show --session <id>` shows the activity alongside agent turns
  install    Install a vertical from a source directory into the discovery dir (#661). Validates the source manifest first, then copies the whole tree to `<target>/<vertical-id>/`. Refuses to overwrite an existing vertical unless `--force` is passed
  uninstall  Remove an installed vertical from the discovery dir (#661). Refuses by default unless `--yes` is passed; this is a destructive on-disk operation so the confirmation is the safety boundary
  lint       Run best-practice hygiene checks against a vertical beyond `validate`'s structural rules (#671). Catches orphan escalation entries, typo'd tool names against the runtime's built-in set, empty bundles / suites, and other things the operator probably didn't mean
  doctor     Composite "is this vertical OK?" health check (#673): validate + eval + lint in one report. Exits non-zero if any check fails. Useful as a single CI gate
  pack       Pack an installed vertical into a tar archive (#675). Inverse of `install` from a tarball — gives operators a single artifact they can ship to CI / share with a team / stash before editing
  publish    Publish an installed vertical to an OCI registry as a versioned artifact (Stream `claude-marketplace-vertical-publish`). Packs every file under `verticals/<id>/` into a deterministic tar via `argo_marketplace::build_vertical_bundle`, then pushes to the registry. The deterministic tar means re-publishing the same on-disk state yields the same digest, so receivers can verify pulls byte-for-byte
  pull       Pull a vertical from an OCI registry into the local discovery dir (Stream `claude-marketplace-vertical-publish`). Verifies the tar's sha256 against the manifest's declared digest BEFORE writing any file — refuses to extract tampered bundles. Refuses if a vertical with the same id already exists locally; pass `--rename <new-id>` to pull under a different local id
  run        One-shot "from zero to running vertical" composite (#669). Creates a fresh session bound to the vertical, attaches every skill the manifest declares, then runs the named `--workflow` against that session so the workflow's tool calls land in the session's audit chain. Emits one combined JSON document with both the session metadata and the workflow report
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo vertical archive`

```text
Soft-delete a vertical so it no longer appears in default listings and cannot bind new sessions or workers

Usage: argo vertical archive [OPTIONS] <ID>

Arguments:
  <ID>  The vertical's id

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

#### `argo vertical delete`

```text
Permanently remove an archived vertical from disk

Usage: argo vertical delete [OPTIONS] <ID>

Arguments:
  <ID>  The vertical's id

Options:
      --force  Confirm permanent deletion. The vertical must already be archived so existing audit/session rows remain the record of prior work
      --json   Output as a single JSON object
  -h, --help   Print help
```

#### `argo vertical describe`

```text
Render a one-paragraph summary of an installed vertical, or the full manifest summary with `--json` (persona, action list, executor matrix, eval seeds, bundled skills — same shape as `show --json`)

Usage: argo vertical describe [OPTIONS] <ID>

Arguments:
  <ID>  The vertical's id (matches `[vertical].id` in `vertical.toml`)

Options:
      --json  Output the full manifest summary as a single JSON object
  -h, --help  Print help
```

#### `argo vertical doctor`

```text
Composite "is this vertical OK?" health check (#673): validate + eval + lint in one report. Exits non-zero if any check fails. Useful as a single CI gate

Usage: argo vertical doctor [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --json  
  -h, --help  Print help
```

#### `argo vertical eval`

```text
Walk every `[evaluation].suites` JSONL file declared in the manifest, validate each fixture row is well-formed, and tabulate fixture counts per suite. Useful as a CI gate that the suite hasn't drifted into bad shape (#656).

Default mode: structural validation only (fixture counts + row-shape errors). Pass `--score` to switch to deterministic rule-based scoring (#693) — each fixture's `expected` block (must_include / must_not_include / abstain / hallucination / pattern_detected / skill_proposal / external_mutation / must_refuse_or_defer / personalization_quality) is checked against an `actual` field on the same row. Rows without an `actual` are reported as `no_actual` and don't count toward pass/fail. No LLM dependency; an LLM-judge layer can be added later as an opt-in extension.

Usage: argo vertical eval [OPTIONS] <ID>

Arguments:
  <ID>
          The vertical's id

Options:
      --json
          Output as a single JSON object. Structural mode shape: `{id, suites:[{path, fixture_count, errors:[…]}]}`. Scored-mode shape: `{id, mode:"scored", suites:[{path, fixture_count, scored_count, no_actual, malformed, passed, failed, pass_rate, unknown_actual_ids, fixtures:[…]}]}`

      --score
          Switch to scored mode (#693). Off by default so CI gates that already depend on the structural shape stay stable

      --actuals <ACTUALS>
          Path to an actuals JSONL file (#694). Each row is `{"id": "<fixture_id>", "actual"|"response": "..."}`. The scorer joins by `id`; external actuals override any `actual` embedded in the canonical fixture row, so the canonical eval suite stays untouched. Ignored without `--score`. Errors loudly on duplicate ids, missing fields, or invalid JSON; warns on unknown ids that don't match any fixture

      --judge <JUDGE>
          Optional judge layer (#698). Off by default — without this flag, scoring is purely rule-based. Two modes: `--judge heuristic` aggregates the rule check outcomes into one verdict per fixture (no LLM, no API key); `--judge <provider>/<model>` is the seam for a live LLM judge — the wiring lands in PR #698b (today the verdict carries a `not_yet_wired` reason that mirrors the rule-engine outcome). Ignored without `--score`. Reject empty / malformed specs

  -h, --help
          Print help (see a summary with '-h')
```

#### `argo vertical install`

```text
Install a vertical from a source directory into the discovery dir (#661). Validates the source manifest first, then copies the whole tree to `<target>/<vertical-id>/`. Refuses to overwrite an existing vertical unless `--force` is passed

Usage: argo vertical install [OPTIONS] <SOURCE>

Arguments:
  <SOURCE>  Path to the source directory containing `vertical.toml`

Options:
      --target <TARGET>  Target verticals dir. Defaults to `argo_vertical::default_dir()` (i.e., `./verticals` or `$ARGO_VERTICALS_DIR`)
      --force            Replace an existing installation of the same id
      --json             Output as a single JSON object `{ok, id, name, version, root}`
  -h, --help             Print help
```

#### `argo vertical lint`

```text
Run best-practice hygiene checks against a vertical beyond `validate`'s structural rules (#671). Catches orphan escalation entries, typo'd tool names against the runtime's built-in set, empty bundles / suites, and other things the operator probably didn't mean

Usage: argo vertical lint [OPTIONS] <ID>

Arguments:
  <ID>  The vertical's id

Options:
      --json  Output as a single JSON object `{ok, id, findings:[{severity, code, message}]}`
  -h, --help  Print help
```

#### `argo vertical list`

```text
List loaded verticals discovered from `./verticals` (or the path in `$ARGO_VERTICALS_DIR`). Prints id / name / version per row, plus any per-vertical load errors

Usage: argo vertical list [OPTIONS]

Options:
      --tenant <TENANT>   Optional tenant id to echo in discovery output. Local vertical installation is workspace-scoped today; the tenant flag is carried for operator scripts that pair this with tenant-scoped worker/session commands
      --source <SOURCE>   Discovery source: workspace, library, or both [default: both] [possible values: workspace, library, both]
      --include-archived  Include soft-archived verticals in the listing
      --json              Output as a single JSON object `{ok, loaded:[…], errors:[…]}`
  -h, --help              Print help
```

#### `argo vertical pack`

```text
Pack an installed vertical into a tar archive (#675). Inverse of `install` from a tarball — gives operators a single artifact they can ship to CI / share with a team / stash before editing

Usage: argo vertical pack [OPTIONS] <ID>

Arguments:
  <ID>  The vertical's id

Options:
      --out <OUT>        Output path; defaults to `<id>.tar` in the cwd
      --target <TARGET>  Source verticals dir. Defaults to the discovery dir
      --json             Output as a single JSON object `{ok, id, path, bytes}`
  -h, --help             Print help
```

#### `argo vertical publish`

```text
Publish an installed vertical to an OCI registry as a versioned artifact (Stream `claude-marketplace-vertical-publish`). Packs every file under `verticals/<id>/` into a deterministic tar via `argo_marketplace::build_vertical_bundle`, then pushes to the registry. The deterministic tar means re-publishing the same on-disk state yields the same digest, so receivers can verify pulls byte-for-byte

Usage: argo vertical publish [OPTIONS] <ID>

Arguments:
  <ID>  The vertical's id

Options:
      --registry <REGISTRY>  OCI registry base URL (e.g. `https://ghcr.io` or `http://localhost:5000`). When omitted, uses the in-memory mock path for backward compatibility
      --tag <TAG>            Tag to publish under. Defaults to the vertical's `[vertical].version` when omitted
      --repo-org <REPO_ORG>  Optional org/namespace prefix on the registry side (e.g. `argo-agents` → `<host>/argo-agents/<id>:<tag>`). When omitted, the bare vertical id is used as the repo
      --token <AUTH_TOKEN>   Optional registry token. Takes precedence over credentials.toml and Docker config
      --json                 Output as a single JSON object
  -h, --help                 Print help
```

#### `argo vertical pull`

```text
Pull a vertical from an OCI registry into the local discovery dir (Stream `claude-marketplace-vertical-publish`). Verifies the tar's sha256 against the manifest's declared digest BEFORE writing any file — refuses to extract tampered bundles. Refuses if a vertical with the same id already exists locally; pass `--rename <new-id>` to pull under a different local id

Usage: argo vertical pull [OPTIONS] <REFERENCE>

Arguments:
  <REFERENCE>  OCI reference, e.g. `ghcr.io/argo-agents/marketplace-seller:1.0.0`

Options:
      --registry <REGISTRY>  Override the registry HTTPS base. When omitted, derived from the reference (HTTP for `localhost`/`127.0.0.1`, HTTPS otherwise)
      --rename <RENAME>      Pull under a different local id when one already exists
      --token <AUTH_TOKEN>   Optional registry token. Takes precedence over credentials.toml and Docker config
      --json                 Output as a single JSON object
  -h, --help                 Print help
```

#### `argo vertical run`

```text
One-shot "from zero to running vertical" composite (#669). Creates a fresh session bound to the vertical, attaches every skill the manifest declares, then runs the named `--workflow` against that session so the workflow's tool calls land in the session's audit chain. Emits one combined JSON document with both the session metadata and the workflow report

Usage: argo vertical run [OPTIONS] --workflow <WORKFLOW> <VERTICAL_ID>

Arguments:
  <VERTICAL_ID>  The vertical's id

Options:
      --workflow <WORKFLOW>  The workflow to run inside the freshly-created session
      --title <TITLE>        Optional human-readable session title
      --provider <PROVIDER>  Provider id; defaults to `mock` so the command works without paid credentials in CI / demo flows [default: mock]
      --model <MODEL>        Model id; defaults to `mock` [default: mock]
      --json                 Output as a single combined JSON document
  -h, --help                 Print help
```

#### `argo vertical show`

```text
Show one vertical's manifest summary by id

Usage: argo vertical show [OPTIONS] <ID>

Arguments:
  <ID>  The vertical's id (matches `[vertical].id` in `vertical.toml`)

Options:
      --json             Output as a single JSON object
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
  -h, --help             Print help
```

#### `argo vertical unarchive`

```text
Re-activate a previously archived vertical

Usage: argo vertical unarchive [OPTIONS] <ID>

Arguments:
  <ID>  The vertical's id

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

#### `argo vertical uninstall`

```text
Remove an installed vertical from the discovery dir (#661). Refuses by default unless `--yes` is passed; this is a destructive on-disk operation so the confirmation is the safety boundary

Usage: argo vertical uninstall [OPTIONS] <ID>

Arguments:
  <ID>  The vertical's id

Options:
      --target <TARGET>  Target verticals dir. Defaults to `argo_vertical::default_dir()`
      --yes              Confirm the deletion. Without this flag the command prints what would happen and exits non-zero
      --json             Output as a single JSON object `{ok, id, removed_path}`
  -h, --help             Print help
```

#### `argo vertical validate`

```text
Re-run the cross-reference validator on one vertical and exit non-zero on any error. Mirrors what the loader does at startup; useful as a CI gate while authoring a vertical

Usage: argo vertical validate [OPTIONS] <ID>

Arguments:
  <ID>  The vertical's id

Options:
      --json  Output as a single JSON object `{id, valid, error?}`
  -h, --help  Print help
```

#### `argo vertical workflow`

```text
Run a named workflow declared by a vertical (#659). Walks the workflow's steps in declaration order. Today only `tool_call` steps run end-to-end — `skill_invoke` and `prompt` need an LLM transport and are recorded with `status="deferred_no_llm"` so the operator can see what would have run. Capability + per-vertical deny lists are enforced. Pass `--session <id>` (#668) to bind the workflow's tool calls to an existing session's audit chain so `argo audit show --session <id>` shows the activity alongside agent turns

Usage: argo vertical workflow [OPTIONS] <VERTICAL_ID> <WORKFLOW_NAME>

Arguments:
  <VERTICAL_ID>    The vertical's id (`[vertical].id` in `vertical.toml`)
  <WORKFLOW_NAME>  The workflow's name (`workflow.name` field of the matching file in `[workflows].files`)

Options:
      --session <SESSION>  Bind the workflow's tool calls to this session's audit chain (#668). Same audit shape as the agent runner so the chain verifier accepts them
      --json               Output as a single JSON object `{vertical_id, workflow, ok, total_ms, steps:[…]}`
  -h, --help               Print help
```

### `argo voice`

```text
Local voice TUI for approval flow (`claude-voice-local-tui-whisper`). Mic capture is opt-in: `--features whisper-native`

Usage: argo voice [OPTIONS]

Options:
      --mock-script <PATH>  Drive the loop from a JSON file (`{"script": [...]}`)
      --json                Emit one TickOutcome per line as JSON
  -h, --help                Print help

Examples:
  argo voice
  argo voice --mock-script fixtures/voice-mock.json --json
```

### `argo wizard`

```text
End-to-end operator setup wizard (Mission `mission-operator-onboarding-wizard`). `argo wizard onboard` replaces the five-command first-time bootstrap (detect → provider key → operator identity → vertical → first worker) with a single guided flow

Usage: argo wizard <COMMAND>

Commands:
  onboard  Walk a fresh operator through detection → provider key → operator identity → first vertical → first worker. Runs interactively against stdin/stdout by default; non-interactive mode reads pre-populated answers from a TOML file (`--answers <path>`). Idempotent: a populated `[operator]` block in `argo.toml` prompts the operator to overwrite (default No)
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo wizard onboard`

```text
Walk a fresh operator through detection → provider key → operator identity → first vertical → first worker. Runs interactively against stdin/stdout by default; non-interactive mode reads pre-populated answers from a TOML file (`--answers <path>`). Idempotent: a populated `[operator]` block in `argo.toml` prompts the operator to overwrite (default No)

Usage: argo wizard onboard [OPTIONS]

Options:
      --non-interactive  Skip prompts; read every answer from `--answers <path>`. Errors out if any required answer is missing or invalid
      --config <PATH>    Override the default `<cwd>/argo.toml` path. Useful for tests and for operators with multiple workspaces on one machine
      --answers <PATH>   TOML file with pre-populated answers for `--non-interactive`. Schema: flat keys mirroring the prompt order — see `docs/skycot/runbook/wizard-onboard.md`
  -h, --help             Print help
```

### `argo work`

```text
Work queue v0 (#716). `add` enqueues a unit of work for a vertical-bound worker; `serve` runs a persistent loop that processes the queue using existing session + memory + chat primitives. Default provider `mock` works without an API key

Usage: argo work <COMMAND>

Commands:
  add    Queue a unit of work for a vertical-bound worker. Validates the vertical exists, then appends a `pending_jobs` memory entry under the vertical's namespace. Returns the job id (the memory entry's UUID)
  serve  Run a persistent worker loop. Reads the oldest unprocessed `pending_jobs` for the vertical, creates a fresh session, runs one turn, writes a `completed_jobs` memory entry, and repeats
  logs   Surface one job's artifact bundle from disk without requiring the operator to remember the `.skycot/outputs/<job_id>/...` layout. Looks up the job's `output_dir` from its `completed_jobs` memory row, then prints (or writes to stdout) the requested file
  jobs   List queued + completed worker jobs across the workspace. Walks the same `pending_jobs` / `completed_jobs` memory rows the queue runner uses, so what you see here is exactly what the worker would process / has processed
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo work add`

```text
Queue a unit of work for a vertical-bound worker. Validates the vertical exists, then appends a `pending_jobs` memory entry under the vertical's namespace. Returns the job id (the memory entry's UUID)

Usage: argo work add [OPTIONS] --vertical <VERTICAL> --prompt <PROMPT>

Options:
      --vertical <VERTICAL>  Vertical id (e.g. `gtm-engineer`). Must be discoverable via `argo vertical list`
      --prompt <PROMPT>      Free-form prompt describing the unit of work. Passed verbatim as the user message on the worker's next turn
      --json                 Emit JSON instead of human-readable
  -h, --help                 Print help

Examples:
  argo work add --vertical gtm-engineer --prompt "draft cold outreach for ACME"
  argo work add --vertical gtm-engineer --prompt "summarise inbox" --json
```

#### `argo work jobs`

```text
List queued + completed worker jobs across the workspace. Walks the same `pending_jobs` / `completed_jobs` memory rows the queue runner uses, so what you see here is exactly what the worker would process / has processed

Usage: argo work jobs [OPTIONS]

Options:
      --state <STATE>        Filter by job state. One of `pending`, `completed`, or `all`. Defaults to `all` [default: all]
      --vertical <VERTICAL>  Filter to a specific vertical's namespace. When omitted, walks every loaded vertical
      --limit <LIMIT>        Cap the number of rows printed. Default 50 [default: 50]
      --json                 Emit JSON-lines (one row per job) instead of a human-readable table
  -h, --help                 Print help

Examples:
  argo work jobs
  argo work jobs --state pending --vertical gtm-engineer
  argo work jobs --state completed --limit 20 --json
```

#### `argo work logs`

```text
Surface one job's artifact bundle from disk without requiring the operator to remember the `.skycot/outputs/<job_id>/...` layout. Looks up the job's `output_dir` from its `completed_jobs` memory row, then prints (or writes to stdout) the requested file

Usage: argo work logs [OPTIONS] <JOB_ID>

Arguments:
  <JOB_ID>  The `job_id` (UUIDv7) of a previously-served job. Use `argo work jobs --json` to find a recent one

Options:
      --kind <KIND>  Which file to surface. Defaults to `transcript`. `all` prints a header per file followed by its contents, in canonical reading order [default: transcript] [possible values: transcript, assistant, approval_checklist, execution_plan, all]
  -h, --help         Print help

Examples:
  argo work logs 01J9JOB...
  argo work logs 01J9JOB... --kind assistant
  argo work logs 01J9JOB... --kind all
```

#### `argo work serve`

```text
Run a persistent worker loop. Reads the oldest unprocessed `pending_jobs` for the vertical, creates a fresh session, runs one turn, writes a `completed_jobs` memory entry, and repeats.

Mock provider (default) produces deterministic synthetic output without dispatching the LLM — no API key required. Switch `--provider` / `--model` to drive a real provider (e.g. `--provider anthropic --model claude-sonnet-4-6`, requires `ANTHROPIC_API_KEY` in env).

Usage: argo work serve [OPTIONS] --vertical <VERTICAL>

Options:
      --vertical <VERTICAL>
          Vertical id this worker serves

      --interval-seconds <INTERVAL_SECONDS>
          Sleep between iterations when no work is pending
          
          [default: 5]

      --provider <PROVIDER>
          Provider for new sessions. Default `mock` short-circuits to deterministic synthetic output without an API key
          
          [default: mock]

      --model <MODEL>
          Model for new sessions
          
          [default: mock]

      --once
          Process at most one job batch then exit. Test affordance — production deployments should omit this

      --max-steps <MAX_STEPS>
          Maximum number of multi-step worker turns per job (#722). One "step" is one full agent turn (i.e. one or more LLM rounds with tool dispatch, ending in an assistant text). The loop stops early when the model emits one of `[STOP: TASK_COMPLETE]`, `[STOP: APPROVAL_REQUIRED]`, or `[STOP: NEED_INFO ...]`. Setting `--max-steps 1` reverts to the single-turn Worker v1 behaviour and skips the research-tools pre-flight check
          
          [default: 5]

      --max-tool-rounds <MAX_TOOL_ROUNDS>
          Per-step ceiling on internal tool-dispatch rounds (#724). One "tool round" is one provider call plus its tool dispatch — research-heavy work (web_search ×N, then web_extract ×N) can hit the cap mid-turn. When that happens the worker no longer aborts with a transport error; it persists the partial transcript, ends the step cleanly, and continues to the next outer step. Default 8
          
          [default: 8]

      --json
          Emit JSON per job instead of human-readable

  -h, --help
          Print help (see a summary with '-h')

Examples:
  argo work serve --vertical gtm-engineer
  argo work serve --vertical gtm-engineer --provider anthropic --model claude-sonnet-4-6
  argo work serve --vertical gtm-engineer --once --json
```

### `argo worker`

```text
Worker v3 execution-mode action ledger (#728). Inspect (and in later PRs: approve / execute / complete) the actions the worker has proposed. Distinct from `argo work` — this surface is the noun (the worker's output), not the verb (running the worker)

Usage: argo worker <COMMAND>

Commands:
  create      Create a persistent worker instance from a vertical template, persona name, and delegated role charter
  onboard     Create a worker from a single TOML persona/role spec
  show        Show the full worker identity plus the resolved policy for one tenant-scoped worker instance
  list        List worker instances for one tenant, optionally filtered by persona name
  costs       Show cost attribution from the worker budget ledger
  fleet       Show live worker presence ordered by `last_active_at DESC`, with workers idle >30 days flagged `idle` and never-seen rows rendered as `(never seen)` (Stream `claude-worker-presence-tracking`)
  portrait    Render a paste-friendly one-page worker portrait
  policy      Inspect resolved worker auto-approval policy documents
  update      Update an existing worker's auto-approval policy from a JSON file (Stream `claude-worker-instance-policy`). The file must deserialise into `argo_identity::AutoApprovalPolicy`; when the worker's policy is `enabled = true`, it overrides the workspace default at gate time
  resume      Resume a worker whose circuit breaker is open. Moves it to half-open so the next action is a supervised probe
  actions     Manage actions the worker has proposed / approved / executed. Backed by `worker_action` memory rows in the originating job's vertical namespace
  items       Inventory view: aggregated per-`item_id` summary derived from the action ledger. Operators running a multi-day worker (e.g. selling skates) use this to answer "what's in inventory, what's sold, what needs my attention?" in one shot. Read-only — never writes to the ledger
  doctor      Print a one-shot SKYCOT health check across the workspace: verticals loaded (with action-section status), action-ledger counts by state, recent failures + blocked rows, and the last-refresh time of the workspace ledger files. Operators run this to answer "is the worker setup healthy?" without scripting 5 commands together
  anomalies   List recent behaviour anomaly events for one worker (Stream `claude-anomaly-detector`). Each row shows `dimension / observed / z-score` so the operator can spot drift the circuit breaker doesn't see (subtler latency spikes, retry-count creep, etc.)
  charter     Manage the per-worker charter (markdown delegation contract, ≤ 16 KB) and KPIs (free-form JSON, ≤ 8 KB) — backs mission claim 2 ("Workers operate under a written charter")
  archive     Sleep / wake a worker — freezes the full identity into a `worker_archive` row and removes the live persona/role/duty rows. Audit + handoff history stay intact (Stream `claude-worker-archive-rehydrate`, mission claim 1)
  rehydrate   Inverse of `worker archive` — restore a previously-archived worker by re-creating its persona/role/duty rows from the frozen JSON snapshot
  delete      Permanently purge an archived worker, cascading per-tenant memory rows and tombstoning audit references (Stream `mission-identity-delete-worker`, mission claim 1 closure). Refuses while the worker is still live (archive first) and refuses inside the `ARGO_WORKER_DELETE_GRACE_DAYS` window (default 30d) unless `--force` is set for the compliance path
  channels    Add, remove, or list the worker's notification channels (`Stream claude-worker-channels-cli`). Channels live as JSON inside the worker row — no schema migration. Cap of 16 per worker; `add` is idempotent on `<kind>:<identifier>` dup
  budget      Set or inspect the worker's daily / monthly spend caps (Stream `claude-worker-budget-cli`). Caps land on the worker's auto-approval policy JSON; period-to-date spend rolls up the existing `worker_budget` ledger
  kpi         Record / list / aggregate observed KPI values for one worker (Stream `claude-worker-kpi-tracker`, mission claim 2 runtime evidence). Charter storage records KPI *definitions* on the worker row; this is the time-series store of observed numeric values
  okr         Set / show / score quarterly OKRs for one worker (Stream `claude-worker-okr-cli`, mission claim 2 goals layer). KPI tracker records observed values; OKRs declare intent against named KPIs and score progress
  review      Quarterly / monthly / weekly review packet bundling KPI deltas, success rate, operator feedback, handoffs, and anomalies for one worker (Stream `claude-worker-review-cycle-cli`, mission claim 2). Read-only aggregator — no new schema
  deps        Manage declared inter-worker dependencies with per-kind cycle detection (Stream `claude-worker-dependency-tracking`, V1.1). Distinct from `handoff_routing` runtime weights; this is *declared intent* — what the operator says one worker structurally needs from another
  success     Per-worker auto-approval success rollup over typed `AuditKind` approval kinds. Read-only join over `audit_record` + `session.worker_id` (mission claim 2 KPI seed)
  promote     Promote a worker up the `entry → junior → senior → lead` tenure ladder (Stream `claude-worker-promotion-workflow`, mission claim 2). Without `--to`, advances to the next level. `--force` overrides the criteria gate (sessions / success-rate / tenure-days); promotion to `lead` always requires `--force`
  level       Show the worker's current level and the numeric gap to the next promotion (e.g. "23 sessions short, 4d tenure short"). Read-only
  restarts    List recent OOM / signal / hung-worker restart events (mission-oom-restart-handler). Read-only view over `worker.oom_restart` audit-chain rows. JSON-lines output; pipe through `jq` for richer filtering
  skills      List the active skills one worker is permitted to surface (Stream `mission-skill-propagation-loop`). Calls `SkillStore::available_for_worker` so the operator can debug "why doesn't worker X see skill Y?" without poking at SQL. Read-only
  elasticity  Worker-pool elasticity status + configuration (`mission-worker-pool-elasticity`). Surfaces the most recent recommender tick and the active `[worker_pool.elasticity]` bounds. Read-only in v1; AutoScale flip is via `argo.toml`
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo worker actions`

```text
Manage actions the worker has proposed / approved / executed. Backed by `worker_action` memory rows in the originating job's vertical namespace

Usage: argo worker actions <COMMAND>

Commands:
  list           List actions, with optional state/job/vertical/type/ platform filters. The "current state" of an action is the latest persisted row matching its `action_id`
  show           Show the full latest snapshot of one action
  approve        Approve an approval-gated action: `pending → approved`. Records `approved_by` + `approved_at`. Refuses if the action isn't currently `pending` or doesn't require approval (per the manifest classification)
  approve-batch  Apply one decision (approve, or deny via `--deny`) to every listed `action_id`. Each id processes independently; one failure doesn't abort the rest. Confirms before writing unless `--confirm` (`-y`) is set, or `--dry-run` lists the ids without writing (`claude-approval-batch-decisions-cli`)
  execute        Mark an approved (or autonomous-pending) action as `executing`. The actual executor logic ships in #731 — for now this is just a state stamp so an operator can signal "I started doing this offline"
  complete       Mark an executing action as `completed`. Records `executed_at` and stores any operator-supplied result
  fail           Mark an executing action as `failed`. Records the reason into `result.failure` for downstream retry / triage
  cancel         Cancel a pending or approved action (Stream `claude-action-cancel-flow`). Records the operator + note into `result.cancellation`. Refused for executing / completed / failed / blocked actions — those are terminal from cancel's standpoint. Cancelled is itself terminal
  status         Show the full state-row history for one action — every `worker_action` row that shares the `action_id`, in time order. Useful to audit who approved when
  render         Render workspace-level per-state ledger artifacts under `.skycot/`. Writes `approval_queue.md`, `completed_actions.md`, and `blocked_actions.md` so the operator has a single review surface across all jobs
  retry          Retry a failed action: emit a NEW pending action with the same payload + platform + item_id, with the failed action_id referenced in the new action's `evidence`. The new action goes through the OS classification fresh — if the manifest still says `requires_approval = true`, it lands `pending` awaiting approval. The failed action stays terminal in the ledger; nothing is mutated retroactively
  help           Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo worker actions approve`

```text
Approve an approval-gated action: `pending → approved`. Records `approved_by` + `approved_at`. Refuses if the action isn't currently `pending` or doesn't require approval (per the manifest classification)

Usage: argo worker actions approve [OPTIONS] <ACTION_ID>

Arguments:
  <ACTION_ID>  `action_id` (UUIDv7)

Options:
      --approved-by <APPROVED_BY>  Operator identifier recorded into `approved_by`. Defaults to `$USER` when omitted
      --note <NOTE>                Optional free-form note appended into the row's `result` field for audit context
      --json                       
  -h, --help                       Print help
```

##### `argo worker actions approve-batch`

```text
Apply one decision (approve, or deny via `--deny`) to every listed `action_id`. Each id processes independently; one failure doesn't abort the rest. Confirms before writing unless `--confirm` (`-y`) is set, or `--dry-run` lists the ids without writing (`claude-approval-batch-decisions-cli`)

Usage: argo worker actions approve-batch [OPTIONS] [ACTION_IDS]...

Arguments:
  [ACTION_IDS]...  Space-separated list of `action_id`s

Options:
      --approved-by <APPROVED_BY>  
      --note <NOTE>                
      --deny                       Cancel each pending action instead of approving
  -y, --confirm                    Skip the interactive confirmation prompt
      --dry-run                    Preview ids without writing
      --json                       
  -h, --help                       Print help
```

##### `argo worker actions cancel`

```text
Cancel a pending or approved action (Stream `claude-action-cancel-flow`). Records the operator + note into `result.cancellation`. Refused for executing / completed / failed / blocked actions — those are terminal from cancel's standpoint. Cancelled is itself terminal

Usage: argo worker actions cancel [OPTIONS] <ACTION_ID>

Arguments:
  <ACTION_ID>  `action_id` (UUIDv7) of the pending or approved action

Options:
      --cancelled-by <CANCELLED_BY>  Operator identifier recorded into the cancellation payload. Defaults to `$USER` when omitted
      --note <NOTE>                  Optional free-form note recorded alongside the cancellation timestamp
      --json                         
  -h, --help                         Print help
```

##### `argo worker actions complete`

```text
Mark an executing action as `completed`. Records `executed_at` and stores any operator-supplied result

Usage: argo worker actions complete [OPTIONS] <ACTION_ID>

Arguments:
  <ACTION_ID>  

Options:
      --note <NOTE>                Free-form one-liner stored under `result.summary`
      --result-json <RESULT_JSON>  Optional structured result blob. Must be valid JSON
      --json                       
  -h, --help                       Print help
```

##### `argo worker actions execute`

```text
Mark an approved (or autonomous-pending) action as `executing`. The actual executor logic ships in #731 — for now this is just a state stamp so an operator can signal "I started doing this offline"

Usage: argo worker actions execute [OPTIONS] <ACTION_ID>

Arguments:
  <ACTION_ID>  

Options:
      --dry-run  Preview execution without changing state or writing an output bundle
      --json     
  -h, --help     Print help
```

##### `argo worker actions fail`

```text
Mark an executing action as `failed`. Records the reason into `result.failure` for downstream retry / triage

Usage: argo worker actions fail [OPTIONS] --reason <REASON> <ACTION_ID>

Arguments:
  <ACTION_ID>  

Options:
      --reason <REASON>  Free-form failure summary (required)
      --json             
  -h, --help             Print help
```

##### `argo worker actions list`

```text
List actions, with optional state/job/vertical/type/ platform filters. The "current state" of an action is the latest persisted row matching its `action_id`

Usage: argo worker actions list [OPTIONS]

Options:
      --state <STATE>              Filter by action state. One of `pending`, `approved`, `executing`, `completed`, `failed`, `blocked`, `all`. Defaults to `all` [default: all]
      --job-id <JOB_ID>            Filter to actions originating from this job_id
      --vertical <VERTICAL>        Filter to actions in a specific vertical's namespace. When omitted, walks every loaded vertical
      --item-id <ITEM_ID>          Filter by `item_id` — the inventory tag stamped onto each action at emission time. Lets the operator scope a long-running multi-day worker to "what's happened for skate-pair-A" without mentally diffing the full ledger. Matches actions whose `item_id` field equals the given string exactly; rows with no `item_id` are excluded
      --action-type <ACTION_TYPE>  Filter by `action_type` (e.g. `post_listing`, `verify_price`). Matches the canonical name; see `handlers::worker_actions::ActionType` for the full list
      --platform <PLATFORM>        Filter by destination platform (e.g. `facebook_marketplace`, `ebay`, `gumtree`). Optional — most action types are platform-agnostic
      --limit <LIMIT>              Cap the number of rows printed. Default 50 [default: 50]
      --json                       Emit JSON-lines (one row per action) instead of a human-readable table
  -h, --help                       Print help
```

##### `argo worker actions render`

```text
Render workspace-level per-state ledger artifacts under `.skycot/`. Writes `approval_queue.md`, `completed_actions.md`, and `blocked_actions.md` so the operator has a single review surface across all jobs

Usage: argo worker actions render [OPTIONS]

Options:
      --vertical <VERTICAL>  Optional vertical filter — when omitted, walks every loaded vertical
  -h, --help                 Print help
```

##### `argo worker actions retry`

```text
Retry a failed action: emit a NEW pending action with the same payload + platform + item_id, with the failed action_id referenced in the new action's `evidence`. The new action goes through the OS classification fresh — if the manifest still says `requires_approval = true`, it lands `pending` awaiting approval. The failed action stays terminal in the ledger; nothing is mutated retroactively.

Retry policy (#755): the bound vertical's manifest can declare `[actions.<name>.retry] max_attempts = N`. The OS counts the retry chain from the source's `evidence` and refuses retries past `max_attempts` unless `--force` is passed. `--force` overrides log to stderr for audit.

Usage: argo worker actions retry [OPTIONS] <ACTION_ID>

Arguments:
  <ACTION_ID>
          `action_id` (UUIDv7) of the failed action

Options:
      --force
          Bypass the manifest's `max_attempts` ceiling. Logged to stderr so audit reviewers see when an operator overrode the policy. Use sparingly — typically the fix is upstream (different prompt, different vertical, different time-of-day) rather than another blind retry

      --json
          

  -h, --help
          Print help (see a summary with '-h')
```

##### `argo worker actions show`

```text
Show the full latest snapshot of one action

Usage: argo worker actions show [OPTIONS] <ACTION_ID>

Arguments:
  <ACTION_ID>  `action_id` (UUIDv7) — the stable identifier across all state transitions

Options:
      --json  Emit JSON instead of the human-readable preview
  -h, --help  Print help
```

##### `argo worker actions status`

```text
Show the full state-row history for one action — every `worker_action` row that shares the `action_id`, in time order. Useful to audit who approved when

Usage: argo worker actions status [OPTIONS] <ACTION_ID>

Arguments:
  <ACTION_ID>  

Options:
      --json  
  -h, --help  Print help
```

#### `argo worker anomalies`

```text
List recent behaviour anomaly events for one worker (Stream `claude-anomaly-detector`). Each row shows `dimension / observed / z-score` so the operator can spot drift the circuit breaker doesn't see (subtler latency spikes, retry-count creep, etc.)

Usage: argo worker anomalies [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7 or `nil` for the platform-default worker that runs unbound work)

Options:
      --since-minutes <SINCE_MINUTES>  Filter to events observed within this many recent minutes. When unset, returns the most-recent N rows
      --limit <LIMIT>                  Cap the number of rows printed. Default 100 (max 1000)
      --json                           Emit JSON-lines (one event per row) instead of a human-readable table
  -h, --help                           Print help
```

#### `argo worker archive`

```text
Sleep / wake a worker — freezes the full identity into a `worker_archive` row and removes the live persona/role/duty rows. Audit + handoff history stay intact (Stream `claude-worker-archive-rehydrate`, mission claim 1)

Usage: argo worker archive <COMMAND>

Commands:
  add   Archive one live worker. Removes its persona/role/duty rows from the live tables and stores a single frozen-JSON row in `worker_archive`. Brief shorthand was `argo worker archive <id>`; clap can't have a positional alongside subcommands at the same level, so the verb form is `argo worker archive add <id>`
  list  List archived workers for one tenant, newest first
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo worker archive add`

```text
Archive one live worker. Removes its persona/role/duty rows from the live tables and stores a single frozen-JSON row in `worker_archive`. Brief shorthand was `argo worker archive <id>`; clap can't have a positional alongside subcommands at the same level, so the verb form is `argo worker archive add <id>`

Usage: argo worker archive add [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7) to archive

Options:
      --tenant <TENANT>        Tenant id whose worker namespace owns the row
      --reason <REASON>        Optional free-text reason recorded alongside the archive row (e.g. "campaign over", "customer churned")
      --principal <PRINCIPAL>  Operator principal recorded as `archived_by_principal`. Defaults to `$USER` when omitted
      --json                   Emit JSON instead of a human-readable summary
  -h, --help                   Print help
```

##### `argo worker archive list`

```text
List archived workers for one tenant, newest first

Usage: argo worker archive list [OPTIONS] --tenant <TENANT>

Options:
      --tenant <TENANT>  Tenant id whose archive should be listed
      --json             Emit JSON-lines (one row per archive entry) instead of a human-readable table
  -h, --help             Print help
```

#### `argo worker budget`

```text
Set or inspect the worker's daily / monthly spend caps (Stream `claude-worker-budget-cli`). Caps land on the worker's auto-approval policy JSON; period-to-date spend rolls up the existing `worker_budget` ledger

Usage: argo worker budget <COMMAND>

Commands:
  set   Set or update the per-worker spend caps. At least one of `--daily`, `--monthly`, or `--clear` is required. Caps are stored as USD micros on the worker's auto-approval policy JSON; the dispatch gate already honours `--daily` (the monthly cap is recorded for `argo worker budget show` / future gate work — see PR notes)
  show  Print the worker's current caps + period-to-date spend
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo worker budget set`

```text
Set or update the per-worker spend caps. At least one of `--daily`, `--monthly`, or `--clear` is required. Caps are stored as USD micros on the worker's auto-approval policy JSON; the dispatch gate already honours `--daily` (the monthly cap is recorded for `argo worker budget show` / future gate work — see PR notes)

Usage: argo worker budget set [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --daily <DAILY>      Daily ceiling in USD (decimal dollars; e.g. `5`, `5.50`, `0` to deny all). Stored as integer micros
      --monthly <MONTHLY>  Monthly ceiling in USD (decimal dollars)
      --clear              Drop both cap fields and fall back to the tenant default
      --json               Emit JSON instead of a human-readable summary
  -h, --help               Print help
```

##### `argo worker budget show`

```text
Print the worker's current caps + period-to-date spend

Usage: argo worker budget show [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --json  Emit JSON instead of a human-readable summary
  -h, --help  Print help
```

#### `argo worker channels`

```text
Add, remove, or list the worker's notification channels (`Stream claude-worker-channels-cli`). Channels live as JSON inside the worker row — no schema migration. Cap of 16 per worker; `add` is idempotent on `<kind>:<identifier>` dup

Usage: argo worker channels <COMMAND>

Commands:
  add     Append a `<kind>:<identifier>` channel spec. Idempotent on exact (kind, target) match — second add prints a warning and makes no write. Refused past the per-worker cap of 16
  remove  Remove a channel by `<kind>:<identifier>`. Errors if no matching channel is present on the worker
  list    Print the worker's channel list
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo worker channels add`

```text
Append a `<kind>:<identifier>` channel spec. Idempotent on exact (kind, target) match — second add prints a warning and makes no write. Refused past the per-worker cap of 16

Usage: argo worker channels add [OPTIONS] --tenant <TENANT> <WORKER_ID> <SPEC>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)
  <SPEC>       `<kind>:<identifier>` (e.g. `email:ops@example.com`, `slack:C0123`, `whatsapp:+15551234567`)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace owns the row
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

##### `argo worker channels list`

```text
Print the worker's channel list

Usage: argo worker channels list [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace should be queried
      --json             Emit JSON instead of a markdown bullet list
  -h, --help             Print help
```

##### `argo worker channels remove`

```text
Remove a channel by `<kind>:<identifier>`. Errors if no matching channel is present on the worker

Usage: argo worker channels remove [OPTIONS] --tenant <TENANT> <WORKER_ID> <CHANNEL_ID>

Arguments:
  <WORKER_ID>   Worker id (UUIDv7)
  <CHANNEL_ID>  `<kind>:<identifier>` to remove (must match an existing channel on the worker). `risk_level` is ignored for matching purposes

Options:
      --tenant <TENANT>  Tenant id whose worker namespace owns the row
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

#### `argo worker charter`

```text
Manage the per-worker charter (markdown delegation contract, ≤ 16 KB) and KPIs (free-form JSON, ≤ 8 KB) — backs mission claim 2 ("Workers operate under a written charter")

Usage: argo worker charter <COMMAND>

Commands:
  set    Set or replace the worker's charter and/or KPIs
  get    Print the worker's charter and/or KPIs
  clear  Null out the charter and KPIs for one worker
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo worker charter clear`

```text
Null out the charter and KPIs for one worker

Usage: argo worker charter clear [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace owns the row
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

##### `argo worker charter get`

```text
Print the worker's charter and/or KPIs

Usage: argo worker charter get [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace should be queried
      --md               Render the charter markdown to stdout (default)
      --json             Emit `{charter, kpis}` JSON
  -h, --help             Print help
```

##### `argo worker charter set`

```text
Set or replace the worker's charter and/or KPIs

Usage: argo worker charter set [OPTIONS] --tenant <TENANT> --file <FILE> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace owns the row
      --file <FILE>      Path to a markdown file holding the charter body
      --kpis <KPIS>      Optional path to a JSON file holding the KPIs object
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

#### `argo worker costs`

```text
Show cost attribution from the worker budget ledger

Usage: argo worker costs [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>
          Worker id whose spend ledger should be queried

Options:
      --tenant <TENANT>
          Tenant id to scope the query. When omitted, all rows for the worker id are included

      --since <SINCE>
          Only include charges at or after this RFC3339 timestamp

      --breakdown <BREAKDOWN>
          Group totals by UTC day or action id

          Possible values:
          - day:    Group charges by UTC calendar day
          - action: Group charges by action id

      --limit <LIMIT>
          Number of recent charge rows to render in text/JSON
          
          [default: 10]

      --json
          Emit a stable JSON envelope instead of text

  -h, --help
          Print help (see a summary with '-h')
```

#### `argo worker create`

```text
Create a persistent worker instance from a vertical template, persona name, and delegated role charter

Usage: argo worker create [OPTIONS] --vertical <VERTICAL> --persona <PERSONA> --charter <CHARTER>

Options:
      --vertical <VERTICAL>
          Vertical template id to instantiate
      --persona <PERSONA>
          Public-facing persona name
      --charter <CHARTER>
          Role charter delegated to this worker
      --builder <BUILDER>
          Operator email or handle that built the worker
      --voice-preamble <VOICE_PREAMBLE>
          Override the default persona voice preamble
      --duty <DUTIES>
          Cron-driven duty: `<cron>::<entry_action>::<description>`
      --fs-root <FS_ROOT>
          Per-worker filesystem-executor sandbox root. Defaults to `/tmp/argo-fs/<worker_id>/` when omitted (V1.2 brief `claude-filesystem-executor`)
      --json
          Emit JSON instead of a human-readable summary
  -h, --help
          Print help
```

#### `argo worker delete`

```text
Permanently purge an archived worker, cascading per-tenant memory rows and tombstoning audit references (Stream `mission-identity-delete-worker`, mission claim 1 closure). Refuses while the worker is still live (archive first) and refuses inside the `ARGO_WORKER_DELETE_GRACE_DAYS` window (default 30d) unless `--force` is set for the compliance path

Usage: argo worker delete [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7) of the archived worker to purge

Options:
      --tenant <TENANT>  Tenant id whose worker namespace owns the archive row
      --force            Skip the grace window (compliance-driven purges only — `argo compliance delete --tenant` calls through here)
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

#### `argo worker deps`

```text
Manage declared inter-worker dependencies with per-kind cycle detection (Stream `claude-worker-dependency-tracking`, V1.1). Distinct from `handoff_routing` runtime weights; this is *declared intent* — what the operator says one worker structurally needs from another

Usage: argo worker deps <COMMAND>

Commands:
  add     Declare that worker `<a>` depends on worker `<b>` for relation `<kind>`. Same-kind cycles are rejected; cross-kind cycles are permitted. Idempotent on `(a, b, kind)`
  remove  Remove the declared dependency `<a> → <b>` for one `<kind>`
  list    List dependencies touching one worker
  graph   Dump the entire workspace dependency graph
  impact  Transitive closure of workers downstream-impacted by `<id>`, sorted by depth
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo worker deps add`

```text
Declare that worker `<a>` depends on worker `<b>` for relation `<kind>`. Same-kind cycles are rejected; cross-kind cycles are permitted. Idempotent on `(a, b, kind)`

Usage: argo worker deps add [OPTIONS] --kind <KIND> <A> <B>

Arguments:
  <A>  The dependent (`<a>` depends on `<b>`)
  <B>  The dependency (`<a>` depends on `<b>`)

Options:
      --kind <KIND>  One of `data|approval|handoff|skill`
      --note <NOTE>  Optional operator note recorded with the row
      --json         Emit JSON instead of a human-readable confirmation
  -h, --help         Print help
```

##### `argo worker deps graph`

```text
Dump the entire workspace dependency graph

Usage: argo worker deps graph [OPTIONS]

Options:
      --format <FORMAT>  Output format: `markdown` (default), `dot`, or `json` [default: markdown]
  -h, --help             Print help
```

##### `argo worker deps impact`

```text
Transitive closure of workers downstream-impacted by `<id>`, sorted by depth

Usage: argo worker deps impact [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id whose downstream impact to compute

Options:
      --json  Emit JSON instead of a human-readable list
  -h, --help  Print help
```

##### `argo worker deps list`

```text
List dependencies touching one worker

Usage: argo worker deps list [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id to query

Options:
      --direction <DIRECTION>  `out` (default) — what this worker depends on. `in` — what depends on this worker. `both` — union of the two [default: out]
      --format <FORMAT>        Output format: `markdown` (default) or `json` [default: markdown]
  -h, --help                   Print help
```

##### `argo worker deps remove`

```text
Remove the declared dependency `<a> → <b>` for one `<kind>`

Usage: argo worker deps remove [OPTIONS] --kind <KIND> <A> <B>

Arguments:
  <A>  The dependent
  <B>  The dependency

Options:
      --kind <KIND>  One of `data|approval|handoff|skill`
      --json         Emit JSON instead of a human-readable confirmation
  -h, --help         Print help
```

#### `argo worker doctor`

```text
Print a one-shot SKYCOT health check across the workspace: verticals loaded (with action-section status), action-ledger counts by state, recent failures + blocked rows, and the last-refresh time of the workspace ledger files. Operators run this to answer "is the worker setup healthy?" without scripting 5 commands together

Usage: argo worker doctor [OPTIONS]

Options:
      --vertical <VERTICAL>  Filter the ledger view to a single vertical
      --json                 Emit JSON instead of the human-readable report
  -h, --help                 Print help
```

#### `argo worker elasticity`

```text
Worker-pool elasticity status + configuration (`mission-worker-pool-elasticity`). Surfaces the most recent recommender tick and the active `[worker_pool.elasticity]` bounds. Read-only in v1; AutoScale flip is via `argo.toml`

Usage: argo worker elasticity <COMMAND>

Commands:
  status  Current pool snapshot + most recent recommendation as JSON
  config  Show the resolved elasticity config (mode + bounds + cooldowns)
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo worker elasticity config`

```text
Show the resolved elasticity config (mode + bounds + cooldowns)

Usage: argo worker elasticity config [OPTIONS]

Options:
      --text  Render as human-readable text instead of JSON
  -h, --help  Print help
```

##### `argo worker elasticity status`

```text
Current pool snapshot + most recent recommendation as JSON

Usage: argo worker elasticity status [OPTIONS]

Options:
      --text  Render as human-readable text instead of JSON
  -h, --help  Print help
```

#### `argo worker fleet`

```text
Show live worker presence ordered by `last_active_at DESC`, with workers idle >30 days flagged `idle` and never-seen rows rendered as `(never seen)` (Stream `claude-worker-presence-tracking`)

Usage: argo worker fleet [OPTIONS]

Options:
      --tenant <TENANT>  Optional tenant id filter. Omitted means all tenants
      --limit <LIMIT>    Cap the number of rows printed. Default 50 [default: 50]
      --since <SINCE>    Only include workers active since this duration (`7d`, `12h`, `30m`, or `60s`)
      --json             Emit JSON instead of a markdown table
  -h, --help             Print help
```

#### `argo worker items`

```text
Inventory view: aggregated per-`item_id` summary derived from the action ledger. Operators running a multi-day worker (e.g. selling skates) use this to answer "what's in inventory, what's sold, what needs my attention?" in one shot. Read-only — never writes to the ledger

Usage: argo worker items <COMMAND>

Commands:
  list  List every `item_id` the worker has ever touched in the action ledger, with per-item rollups: sold / active state, total actions, count per action-state, latest activity. Operator's "what's in inventory?" view
  show  Show the full action history for one item, in time order. Unlike `actions list --item-id <id>` (which shows the latest snapshot per action), `items show` walks every `worker_action` memory row tagged with the given `item_id` so the operator can audit who approved what, when it executed, and what the result was. Designed for "why is skates-A still in inventory?" forensics
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo worker items list`

```text
List every `item_id` the worker has ever touched in the action ledger, with per-item rollups: sold / active state, total actions, count per action-state, latest activity. Operator's "what's in inventory?" view

Usage: argo worker items list [OPTIONS]

Options:
      --vertical <VERTICAL>  Filter to a single vertical's namespace
      --state <STATE>        Filter by sold status. `sold` shows items with a completed `mark_item_sold` row. `active` shows items with no completed mark_item_sold (i.e. still in inventory). `all` (default) shows both [default: all]
      --limit <LIMIT>        Cap the number of rows printed. Default 50 [default: 50]
      --json                 Emit JSON-lines (one row per item) instead of a human-readable table
  -h, --help                 Print help
```

##### `argo worker items show`

```text
Show the full action history for one item, in time order. Unlike `actions list --item-id <id>` (which shows the latest snapshot per action), `items show` walks every `worker_action` memory row tagged with the given `item_id` so the operator can audit who approved what, when it executed, and what the result was. Designed for "why is skates-A still in inventory?" forensics

Usage: argo worker items show [OPTIONS] <ITEM_ID>

Arguments:
  <ITEM_ID>  The `item_id` to inspect. Exact-match against the `item_id` field stamped at emission time

Options:
      --vertical <VERTICAL>  Filter to a single vertical's namespace. When the same item_id exists under multiple verticals (rare but possible), required to disambiguate
      --json                 Emit JSON (full row dump) instead of the human- readable timeline. JSON output is one object per row in chronological order
  -h, --help                 Print help
```

#### `argo worker kpi`

```text
Record / list / aggregate observed KPI values for one worker (Stream `claude-worker-kpi-tracker`, mission claim 2 runtime evidence). Charter storage records KPI *definitions* on the worker row; this is the time-series store of observed numeric values

Usage: argo worker kpi <COMMAND>

Commands:
  record  Append one observation of `kpi` for `worker_id`
  list    List observed KPI values for a worker, newest first
  series  Roll observations into time buckets and report avg/min/max/count
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo worker kpi list`

```text
List observed KPI values for a worker, newest first

Usage: argo worker kpi list [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7) to query

Options:
      --tenant <TENANT>  Tenant id whose worker namespace should be queried
      --kpi <KPI>        Filter to one KPI name. When omitted, all KPIs are returned
      --since <SINCE>    RFC3339 cutoff — only observations at or after this time are returned. Useful for "show me this week's csat" without counting rows
      --json             Emit JSON-lines (one row per observation) instead of a human-readable table
  -h, --help             Print help
```

##### `argo worker kpi record`

```text
Append one observation of `kpi` for `worker_id`

Usage: argo worker kpi record [OPTIONS] --tenant <TENANT> <WORKER_ID> <KPI> <VALUE>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7) the observation belongs to
  <KPI>        KPI name (e.g. `csat`, `reply_sla_hours`). Free-form; not validated against the worker's charter `kpi_json` definition list — that's a follow-up
  <VALUE>      Numeric value. Stored as SQLite REAL

Options:
      --tenant <TENANT>  Tenant id whose worker namespace owns the row
      --note <NOTE>      Optional free-text note recorded alongside the value (e.g. "post-incident review", "customer migrated")
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

##### `argo worker kpi series`

```text
Roll observations into time buckets and report avg/min/max/count

Usage: argo worker kpi series [OPTIONS] --tenant <TENANT> --bucket <BUCKET> <WORKER_ID> <KPI>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7) to aggregate
  <KPI>        KPI name to aggregate. Required — series is per-KPI

Options:
      --tenant <TENANT>  Tenant id whose worker namespace should be queried
      --bucket <BUCKET>  Bucket granularity: `day`, `week` (Monday-stamped), or `month` (`YYYY-MM`)
      --json             Emit JSON-lines (one row per bucket) instead of a human-readable table
  -h, --help             Print help
```

#### `argo worker level`

```text
Show the worker's current level and the numeric gap to the next promotion (e.g. "23 sessions short, 4d tenure short"). Read-only

Usage: argo worker level [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace owns the row
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

#### `argo worker list`

```text
List worker instances for one tenant, optionally filtered by persona name

Usage: argo worker list [OPTIONS] --tenant <TENANT>

Options:
      --tenant <TENANT>    Tenant id whose worker namespace should be queried
      --persona <PERSONA>  Filter to workers whose persona name exactly matches
      --json               Emit JSON instead of a human-readable table
  -h, --help               Print help
```

#### `argo worker okr`

```text
Set / show / score quarterly OKRs for one worker (Stream `claude-worker-okr-cli`, mission claim 2 goals layer). KPI tracker records observed values; OKRs declare intent against named KPIs and score progress

Usage: argo worker okr <COMMAND>

Commands:
  set       Declare an objective + its weighted key-results for one period
  show      List one worker's objectives, optionally filtered by period
  progress  Score each objective against the latest `worker_kpi_value` row
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo worker okr progress`

```text
Score each objective against the latest `worker_kpi_value` row

Usage: argo worker okr progress [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7) to score

Options:
      --tenant <TENANT>  Tenant id whose worker namespace should be queried
      --period <PERIOD>  Filter to one period (`YYYYQ[1-4]`)
      --json             Emit JSON instead of a human-readable scorecard
  -h, --help             Print help
```

##### `argo worker okr set`

```text
Declare an objective + its weighted key-results for one period

Usage: argo worker okr set [OPTIONS] --tenant <TENANT> --period <PERIOD> --title <TITLE> --kr <KR>... <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7) the objective belongs to

Options:
      --tenant <TENANT>            Tenant id whose worker namespace owns the row
      --period <PERIOD>            Period token (`YYYYQ[1-4]`, e.g. `2026Q2`)
      --title <TITLE>              Operator-facing objective title
      --description <DESCRIPTION>  Optional long-form description
      --kr <KR>...                 Repeatable `--kr kpi:op:target:weight`. `op` is `gte|lte|eq`, `weight > 0`. At least one is required
      --json                       Emit JSON instead of a human-readable summary
  -h, --help                       Print help
```

##### `argo worker okr show`

```text
List one worker's objectives, optionally filtered by period

Usage: argo worker okr show [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7) to query

Options:
      --tenant <TENANT>  Tenant id whose worker namespace should be queried
      --period <PERIOD>  Filter to one period (`YYYYQ[1-4]`)
      --format <FORMAT>  Output format: `markdown` (default) or `json` [default: markdown]
  -h, --help             Print help
```

#### `argo worker onboard`

```text
Create a worker from a single TOML persona/role spec

Usage: argo worker onboard [OPTIONS] --persona-spec <PERSONA_SPEC>

Options:
      --persona-spec <PERSONA_SPEC>  Path to the onboarding TOML spec
      --id <ID>                      Explicit worker id. Fails if it already exists
      --role-name <ROLE_NAME>        Optional role-name label consumed by `TargetSelector::RoleMatch(String)` routing-rule resolution (Tier M4). Omit to leave the worker unclassified
      --json                         Emit JSON instead of a markdown summary
  -h, --help                         Print help
```

#### `argo worker policy`

```text
Inspect resolved worker auto-approval policy documents

Usage: argo worker policy <COMMAND>

Commands:
  show  Print the resolved AutoApprovalPolicy JSON for one worker
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### `argo worker policy show`

```text
Print the resolved AutoApprovalPolicy JSON for one worker

Usage: argo worker policy show [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace should be queried
      --json             Emit JSON. Accepted for symmetry with every worker subcommand
  -h, --help             Print help
```

#### `argo worker portrait`

```text
Render a paste-friendly one-page worker portrait

Usage: argo worker portrait [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace should be queried
      --json             Emit JSON instead of markdown
  -h, --help             Print help
```

#### `argo worker promote`

```text
Promote a worker up the `entry → junior → senior → lead` tenure ladder (Stream `claude-worker-promotion-workflow`, mission claim 2). Without `--to`, advances to the next level. `--force` overrides the criteria gate (sessions / success-rate / tenure-days); promotion to `lead` always requires `--force`

Usage: argo worker promote [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace owns the row
      --to <TO>          Explicit target level (`junior` / `senior` / `lead`). When omitted, the worker is advanced to the next level above its current one
      --force            Bypass the criteria gate (records the override + reason in the audit row). Required for `--to lead` regardless of numeric thresholds
      --reason <REASON>  Operator note recorded in the promotion audit row
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

#### `argo worker rehydrate`

```text
Inverse of `worker archive` — restore a previously-archived worker by re-creating its persona/role/duty rows from the frozen JSON snapshot

Usage: argo worker rehydrate [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7) of the archived worker

Options:
      --tenant <TENANT>  Tenant id whose worker namespace owns the row
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

#### `argo worker restarts`

```text
List recent OOM / signal / hung-worker restart events (mission-oom-restart-handler). Read-only view over `worker.oom_restart` audit-chain rows. JSON-lines output; pipe through `jq` for richer filtering

Usage: argo worker restarts [OPTIONS]

Options:
      --worker <WORKER>  Filter to one worker id
      --since <SINCE>    `now - <duration>` filter; suffixes `s` / `m` / `h` / `d` (e.g. `1h`, `30d`)
  -h, --help             Print help
```

#### `argo worker resume`

```text
Resume a worker whose circuit breaker is open. Moves it to half-open so the next action is a supervised probe

Usage: argo worker resume [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace should be resumed
      --reason <REASON>  Operator note recorded in the circuit audit trail
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

#### `argo worker review`

```text
Quarterly / monthly / weekly review packet bundling KPI deltas, success rate, operator feedback, handoffs, and anomalies for one worker (Stream `claude-worker-review-cycle-cli`, mission claim 2). Read-only aggregator — no new schema

Usage: argo worker review [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --period <PERIOD>  Period preset: `quarterly` (default), `monthly`, `weekly` [default: quarterly]
      --since <SINCE>    Override the period start with an explicit RFC3339 cutoff
      --format <FORMAT>  `markdown` (default) or `json` [default: markdown]
  -h, --help             Print help
```

#### `argo worker show`

```text
Show the full worker identity plus the resolved policy for one tenant-scoped worker instance

Usage: argo worker show [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace should be queried
      --json             Emit JSON instead of a human-readable summary
  -h, --help             Print help
```

#### `argo worker skills`

```text
List the active skills one worker is permitted to surface (Stream `mission-skill-propagation-loop`). Calls `SkillStore::available_for_worker` so the operator can debug "why doesn't worker X see skill Y?" without poking at SQL. Read-only

Usage: argo worker skills [OPTIONS] --tenant <TENANT> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7)

Options:
      --tenant <TENANT>  Tenant id whose worker namespace owns the row
      --available-only   Restrict the listing to skills that carry a non-NULL `min_trust_grade` floor — i.e. the gated subset that actually depends on the worker's trust grade
      --json             Emit one JSON object per skill instead of the columnar text layout
  -h, --help             Print help
```

#### `argo worker success`

```text
Per-worker auto-approval success rollup over typed `AuditKind` approval kinds. Read-only join over `audit_record` + `session.worker_id` (mission claim 2 KPI seed)

Usage: argo worker success [OPTIONS] <WORKER_ID>

Arguments:
  <WORKER_ID>  

Options:
      --since <SINCE>    `now - <duration>` filter; suffixes `s`/`m`/`h`/`d`
      --bucket <BUCKET>  `none` (default), `day`, `week`, or `month` [default: none]
      --format <FORMAT>  `markdown` (default) or `json` [default: markdown]
  -h, --help             Print help
```

#### `argo worker update`

```text
Update an existing worker's auto-approval policy from a JSON file (Stream `claude-worker-instance-policy`). The file must deserialise into `argo_identity::AutoApprovalPolicy`; when the worker's policy is `enabled = true`, it overrides the workspace default at gate time

Usage: argo worker update [OPTIONS] --auto-approval-json <AUTO_APPROVAL_JSON> <WORKER_ID>

Arguments:
  <WORKER_ID>  Worker id (UUIDv7) to update

Options:
      --auto-approval-json <AUTO_APPROVAL_JSON>  Path to a JSON `AutoApprovalPolicy` document
      --json                                     Emit JSON instead of a human-readable summary
  -h, --help                                     Print help
```

### `argo workflow`

```text
Declarative DAG workflows across multiple workers (Stream `claude-workflow-dag`). YAML schema describing tasks + edges; runs each task as a Session with output binding from upstream tasks

Usage: argo workflow <COMMAND>

Commands:
  validate     Validate a workflow YAML file: parses, checks the DAG topology (no cycles, every edge endpoint declared), and reports the topological execution order
  run          Run a workflow. Without `--dry-run`, executes every task against the [mock] runner — production worker dispatch wiring is a follow-up. With `--dry-run`, prints the topological execution plan without invoking the runner
  status       Show the latest report for one workflow run id
  list-runs    List workflow runs in this workspace
  resume       Resume a previously-checkpointed workflow run. Replays `workflow_checkpoint` rows to restore prior outputs and runs only the still-pending tasks
  checkpoints  List every checkpoint persisted for one workflow run, in seq order
  rollback     Drop every checkpoint row strictly above `--to-checkpoint` for one run. Subsequent `argo workflow resume` will pick up from the rolled-back point
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo workflow checkpoints`

```text
List every checkpoint persisted for one workflow run, in seq order

Usage: argo workflow checkpoints [OPTIONS] <RUN_ID>

Arguments:
  <RUN_ID>  Workflow run id

Options:
      --json  Output as a single JSON array
  -h, --help  Print help
```

#### `argo workflow list-runs`

```text
List workflow runs in this workspace

Usage: argo workflow list-runs [OPTIONS]

Options:
      --limit <LIMIT>  Cap the number of rows printed. Default 50 [default: 50]
      --json           Output as a single JSON object
  -h, --help           Print help
```

#### `argo workflow resume`

```text
Resume a previously-checkpointed workflow run. Replays `workflow_checkpoint` rows to restore prior outputs and runs only the still-pending tasks

Usage: argo workflow resume [OPTIONS] <RUN_ID>

Arguments:
  <RUN_ID>  Workflow run id (UUIDv7) returned by `run`

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

#### `argo workflow rollback`

```text
Drop every checkpoint row strictly above `--to-checkpoint` for one run. Subsequent `argo workflow resume` will pick up from the rolled-back point

Usage: argo workflow rollback [OPTIONS] --to-checkpoint <TO_CHECKPOINT> <RUN_ID>

Arguments:
  <RUN_ID>  Workflow run id

Options:
      --to-checkpoint <TO_CHECKPOINT>  Target seq to roll back to (rows with seq > target are dropped)
      --json                           Output as a single JSON object
  -h, --help                           Print help
```

#### `argo workflow run`

```text
Run a workflow. Without `--dry-run`, executes every task against the [mock] runner — production worker dispatch wiring is a follow-up. With `--dry-run`, prints the topological execution plan without invoking the runner

Usage: argo workflow run [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to a workflow YAML file

Options:
      --dry-run  Print the execution plan without invoking the runner
      --json     Output as a single JSON object
  -h, --help     Print help
```

#### `argo workflow status`

```text
Show the latest report for one workflow run id

Usage: argo workflow status [OPTIONS] <RUN_ID>

Arguments:
  <RUN_ID>  Workflow run id (UUIDv7) returned by `run`

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

#### `argo workflow validate`

```text
Validate a workflow YAML file: parses, checks the DAG topology (no cycles, every edge endpoint declared), and reports the topological execution order

Usage: argo workflow validate [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to a workflow YAML file

Options:
      --json  Output as a single JSON object
  -h, --help  Print help
```

### `argo workspace`

```text
Usage: argo workspace <COMMAND>

Commands:
  init       Initialize a fresh workspace at `.` (writes argo.toml + the canonical bundle directories)
  status     One-line status summary (workspace + db + costs)
  verify     Verify workspace integrity: argo.toml parses, memory files + bundle dirs exist, agent persona files exist, manifests referenced from `[provider.profiles.*]` resolve
  fork       Pull a workspace bundle from an OCI ref and extract it into `--into <dir>` (defaults to a sibling directory named after the repo). The destination directory must not exist
  push       
  pull       
  artifacts  List recorded marketplace pushes (newest first). Each row shows the OCI reference, bundle digest, pushed-at timestamp, and `pushed_by` attribution
  diff       
  publish    
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `argo workspace artifacts`

```text
List recorded marketplace pushes (newest first). Each row shows the OCI reference, bundle digest, pushed-at timestamp, and `pushed_by` attribution

Usage: argo workspace artifacts [OPTIONS]

Options:
      --limit <LIMIT>  [default: 25]
      --json           Output as JSON (one object per row) for scripting
  -h, --help           Print help
```

#### `argo workspace diff`

```text
Usage: argo workspace diff <OTHER>

Arguments:
  <OTHER>  

Options:
  -h, --help  Print help
```

#### `argo workspace fork`

```text
Pull a workspace bundle from an OCI ref and extract it into `--into <dir>` (defaults to a sibling directory named after the repo). The destination directory must not exist

Usage: argo workspace fork [OPTIONS] <SOURCE>

Arguments:
  <SOURCE>  `<host>/<repo>:<tag>` reference, e.g. `ghcr.io/argo-agents/example:v1`

Options:
      --into <INTO>  Destination directory. Defaults to `./fork-<repo>-<tag>` next to the cwd
  -h, --help         Print help
```

#### `argo workspace init`

```text
Initialize a fresh workspace at `.` (writes argo.toml + the canonical bundle directories)

Usage: argo workspace init

Options:
  -h, --help  Print help

Examples:
  argo workspace init
```

#### `argo workspace publish`

```text
Usage: argo workspace publish

Options:
  -h, --help  Print help
```

#### `argo workspace pull`

```text
Usage: argo workspace pull <OCI_REF>

Arguments:
  <OCI_REF>  

Options:
  -h, --help  Print help
```

#### `argo workspace push`

```text
Usage: argo workspace push <OCI_REF>

Arguments:
  <OCI_REF>  

Options:
  -h, --help  Print help
```

#### `argo workspace status`

```text
One-line status summary (workspace + db + costs)

Usage: argo workspace status [OPTIONS]

Options:
      --json             Output as a single JSON object for scripting. Equivalent to `--format json`
      --format <FORMAT>  Output format: `text` (default), `json`, or `markdown`. `--json` wins if both are passed [default: text]
  -h, --help             Print help

Examples:
  argo workspace status
  argo workspace status --json
```

#### `argo workspace verify`

```text
Verify workspace integrity: argo.toml parses, memory files + bundle dirs exist, agent persona files exist, manifests referenced from `[provider.profiles.*]` resolve

Usage: argo workspace verify

Options:
  -h, --help  Print help

Examples:
  argo workspace verify
```

<!-- END AUTO-GENERATED CLI REFERENCE -->
