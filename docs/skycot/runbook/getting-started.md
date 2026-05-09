# SKYCOT Operator Runbook — Getting started

A walkthrough from `cargo install` to a bootstrapped workspace. For the
full table of contents, see [RUNBOOK.md](../RUNBOOK.md).

---

## 1. What you're getting

> The mission verbatim, from MISSION.md:
>
> AI workers that humans can bring into their team as a real
> teammate, that can be delegated a responsibility / task / work /
> role; the platform can build an AI worker for it, draw upon a
> shared skills library, or even learn and build its own new
> skills; it has memory; it works autonomously and alongside
> humans; AI workers can fully autonomously do real-world work;
> that work is any task a human can do on a PC/Mac; approval
> requests can flow via many different communication methods.

In v1 SKYCOT ships:

- **A vertical generator** (`argo genesis create`) that turns a
  one-line charter into a complete worker template — system
  prompt, action vocabulary, memory schema, eval seeds. 12
  archetypes ship out of the box (Sales SDR, Marketing Intern,
  Customer Support, etc.). Add `--refine` for an LLM-driven
  refinement pass on top of the deterministic floor.
- **A safety judge** built into Genesis: every action whose
  name contains a sensitive verb (`send`, `post`, `pay`,
  `publish`, `delete`, `share`, `message`) is automatically
  marked `requires_approval = true`, regardless of what the LLM
  proposes. The hard rule cannot be bypassed.
- **Two execution backends**:
  - `HumanExecutor` for human-carry-out actions (artifact
    bundles + `complete <id>` workflow).
  - `ComputerUseExecutor` driving an E2B Firecracker microVM
    + Anthropic computer-use loop for novel GUI work.
- **Auto-approval policy** with per-action / daily USD caps,
  time windows (with cross-midnight wrap), wake rules for
  urgent action types. Per-worker overrides via
  `argo worker update --auto-approval-json`.
- **Multi-channel notify** for approval gates: email
  (Postmark), Slack (Block Kit buttons), SMS (Twilio), push
  (Expo APNs/FCM), web inbox.
- **A worker-instance identity layer** — workers persist
  across sessions with stable ids, persona, role, memory
  namespace, channel preferences.
- **A 3-layer memory store** (recent / indexed / summary) with
  sqlite-vec + Voyage-3 embeddings.
- **A skill library** with trust grades (`builtin → tenant →
  published → learned-unreviewed`) and a learning loop that
  drafts SKILL.md proposals from successful CU trajectories.
- **An orchestrator** with Session + Handoff + EscalationChain
  primitives for multi-worker coordination.

This runbook walks you through using all of it.

---

## 2. Install

### 2.1 Prerequisites

| Requirement | Why |
|---|---|
| Rust 1.75+ | The CLI builds against the workspace's MSRV. |
| SQLite 3.40+ | The session store ships its own SQLite via `sqlx`; system version only matters if you want to inspect the DB with `sqlite3 .skycot/argo.db`. |
| `git` | Genesis writes a vertical tree — most operators commit it to a repo. |
| (optional) `ANTHROPIC_API_KEY` | Required for `argo chat`, `argo genesis create --refine`, and the Anthropic CU loop. |
| (optional) `OPENAI_API_KEY` | Alternate provider for chat. |
| (optional) `E2B_API_KEY` | Required for `ComputerUseExecutor` actions. |

### 2.2 From source

```bash
git clone https://github.com/mark-hallam/Argo-Agent skycot
cd skycot
cargo install --path crates/argo-cli
argo --version
```

Cargo installs the `argo` binary into `~/.cargo/bin/`. Verify it's on PATH:

```bash
which argo
# /Users/you/.cargo/bin/argo
```

### 2.3 Recommended directory layout

A workspace is just a directory with a `.skycot/` subdir. The
CLI auto-creates it on first run, but if you want a clean
layout up front:

```
my-workers/
├── .skycot/
│   ├── argo.db                 # Session store + identity + audit
│   ├── auto_approval.toml      # Workspace-level approval policy
│   └── outputs/                # Per-action artifact bundles
│       └── <job_id>/
│           └── actions/<action_id>/
├── verticals/
│   ├── marketing-intern-maya/
│   │   ├── vertical.toml
│   │   ├── worker.toml
│   │   ├── prompts/system.md
│   │   ├── prompts/persona.md
│   │   ├── memory/schema.toml
│   │   └── eval/seed.jsonl
│   └── sales-sdr-sam/
│       └── ...
└── manifests/
    └── executors/
        └── computer_use.toml   # CU profile registry
```

Genesis writes the per-vertical tree under `verticals/`.
Everything else under `.skycot/` is the runtime store.

### 2.4 Required env vars

The minimum viable workspace is **zero env vars** — the CLI's
read-side surface, Genesis (without `--refine`), and the human
executor all run offline.

For each capability you want:

| Capability | Env vars |
|---|---|
| `argo chat` (Anthropic) | `ANTHROPIC_API_KEY` |
| `argo chat` (OpenAI) | `OPENAI_API_KEY` |
| `argo genesis create --refine` | `ANTHROPIC_API_KEY` |
| `ComputerUseExecutor` | `E2B_API_KEY`, `ANTHROPIC_API_KEY` |
| Email approval channel | `POSTMARK_SERVER_TOKEN` |
| Slack approval channel | `SLACK_BOT_TOKEN`, `SLACK_SIGNING_SECRET` |
| SMS approval channel | `TWILIO_ACCOUNT_SID`, `TWILIO_AUTH_TOKEN`, `TWILIO_FROM_NUMBER` |
| Push approval channel | `EXPO_ACCESS_TOKEN` |

Missing env vars surface at the **first call site that needs
them**, never at startup. So you can install + bootstrap
without any keys, then add them as you wire up channels.

---

## 3. Bootstrap

```bash
cd my-workers
argo doctor
```

`argo doctor` is the first thing to run in a fresh workspace.
It walks every loaded vertical, checks the action ledger
schema, verifies memory layers, and reports any per-vertical
load errors.

Sample output:

```
argo doctor — workspace report

verticals (0 loaded):
  (no verticals — run `argo genesis create` to make one)

session store: ok (.skycot/argo.db, 41 KB, schema v15)
identity store: ok (0 workers)
memory layers: ok (0 entries across recent/indexed/summary)
audit log: ok (0 rows)

🟢 ready
```

If you see `🔴 errors`, the report names the offending vertical
or schema column. Common fixes:

| Symptom | Fix |
|---|---|
| `vertical 'X' failed to load: schema mismatch` | Bump the vertical's `schema_version` to match `argo --version`'s. |
| `no [actions] section` | The vertical's `vertical.toml` is missing the `[actions]` table — add at least one action declaration. |
| `migration v15 not applied` | Older workspace; run any `argo` command and the runtime auto-migrates on first connect. |

### 3.1 Verifying providers

```bash
argo provider list
argo doctor --json | jq '.providers'
```

If you've set `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`, they
show up here. If not, the list is empty (which is fine — you
just can't use chat / refine until you set one).

### 3.2 Verifying channels

Channels are registered via `manifests/channels/<name>.toml`
files in your workspace. The four shipped impls:

```bash
argo gateway routes --grep approve
```

This lists the gateway's HTTP endpoints that the channels
post decision callbacks to. If a channel has been configured
(toml + env vars), you'll see its decision route here.

---

Next: [Generate your first worker](workers.md).
