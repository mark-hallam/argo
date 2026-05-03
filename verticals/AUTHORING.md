# Authoring a vertical

Step-by-step walkthrough of building a new vertical agent on the
Argo platform. Aimed at someone who has the binary on `$PATH`,
clones this repo (or just the `verticals/` tree), and wants to
ship a working agent without touching the core platform crates.

> **Why "vertical-template"?** Argo's core platform is
> deliberately vertical-agnostic. A vertical is the **product**
> shape of the platform — a manifest + prompts + skills + eval
> fixtures + memory schema that, taken together, become a
> specific agent (front desk, PR reviewer, ticket triage, …).
> Two reference verticals ship today (`pr-checker`,
> `receptionist`) — copy either as a starting point.

## TL;DR (5 minutes)

```bash
# 1. Clone the closest reference vertical
cp -r verticals/receptionist verticals/my-vertical

# 2. Edit the manifest — at minimum, change [vertical].id, name, version
$EDITOR verticals/my-vertical/vertical.toml

# 3. Replace the persona + system prompt for your domain
$EDITOR verticals/my-vertical/prompts/{system,persona}.md

# 4. Replace the eval fixtures — JSONL with id / input / expected per row
$EDITOR verticals/my-vertical/eval/*.jsonl

# 5. Validate end-to-end
argo vertical validate my-vertical          # cross-ref check
argo vertical eval my-vertical              # fixture sanity check
argo vertical show my-vertical              # what the runtime sees

# 6. Wire a session against it (when ready)
argo new --provider anthropic --model claude-sonnet-4-5 \
  --vertical my-vertical --title "first run"
```

That's the whole loop. The rest of this document expands each
step + covers the boundaries you must respect.

## The architecture in one paragraph

The Argo binary stays the same across every vertical. What
*changes* per product is the **manifest** (`vertical.toml`) and
the files it references — system prompt, skill markdown,
evaluation fixtures, optional memory seed, optional workflow
declarations, escalation policy. The runtime reads those files
when a session binds to the vertical, applies them
declaratively, and audits every action against them. There is
**no place** in the codebase to add "if vertical_id == 'foo'"
branches — that's an invariant.

## The minimum manifest

```toml
schema_version = "1"

[vertical]
id      = "my-vertical"
name    = "My Vertical"
version = "0.1.0"
```

That alone parses + validates. Loaded sessions get the
platform's default behaviour (no role, no tool restrictions, no
escalation). You'll usually want more — read on.

## The seven sections

Every section below is optional. The validator (`argo vertical
validate`) tells you which file references resolve and which
don't.

### `[role]` — the agent's voice

```toml
[role]
system_prompt = "prompts/system.md"   # required if [role] is present
persona       = "prompts/persona.md"  # optional
```

`prompts/system.md` is the **only** place the platform looks for
the system message that gets prepended to every turn. Keep it
short, declarative, and structured. The runtime appends
skill-derived content after it — don't duplicate that here.

`prompts/persona.md` is auxiliary tone guidance that the
template author can reference from skills (it isn't auto-
prepended). Useful when multiple skills want to share voice
direction without re-stating it.

### `[provider]` — model defaults

```toml
[provider]
default   = "anthropic/claude-sonnet-4-5"
fallback  = "openai/gpt-5"
cost_tier = "balanced"     # one of: economy | balanced | premium
```

Carried for declarative completeness today; the model defaults
are advisory hints to the operator who runs `argo new
--vertical <id>`. The runtime uses whatever provider the session
was created with.

### `[tools]` — the security boundary

```toml
[tools]
allowed = ["file_read", "memory", "web_search"]
denied  = ["file_write", "file_patch", "terminal", "delegate_task"]
```

**Deny is the boundary**, not allow. The capability engine
(#655) treats every name in `denied` as a hard "never" — even if
the principal's role grants would otherwise permit the call,
deny wins. The agent receives a clear "denied by the bound
vertical's manifest" error so it can route around the boundary
instead of silently no-op'ing.

`allowed` is a discoverability hint to the agent (which tools
exist and are encouraged). It is **not** an "only these"
restriction — that's what `denied` is for.

When in doubt, **declare every state-mutating tool you don't
want** under `denied`. Defence-in-depth: if a future edit adds a
new mutating tool to the in-core set, your manifest still fails
closed.

### `[memory]` — typed seed data

```toml
[memory]
schema = "memory.schema.toml"
seed   = "memory/seed/"
```

`memory.schema.toml` declares the typed shape of the per-vertical
memory store. `memory/seed/` holds preloaded rows
(markdown / json / jsonl) that get installed when the session
binds to the vertical. The receptionist's `memory/seed/known_faqs.md`
is the canonical example.

### `[skills]` — auto-attached on session bind

```toml
[skills]
bundle = "skills/"
```

Every `*.skill.md` file in `skills/` gets installed (if not
already present) and attached to the session via `session_skill`
when `argo new --vertical <id>` runs. This is how a vertical
ships its specialized capabilities to the agent loop.

Skill markdown shape (frontmatter required):

```markdown
---
name: identify_intent
version: 1.0.0
description: Classify the visitor message into one of seven canonical intents.
provenance:
  source: receptionist/v0.1.0
  hash: <sha-256 of body>
---

# Body in markdown — the system-prompt fragment the agent gets when this skill is active.
```

### `[workflows]` — declarative state machines

```toml
[workflows]
files = ["workflows/check_invariants.workflow.toml"]
```

Workflow files declare a named, ordered sequence of steps. The
runner (`argo vertical workflow <vertical> <workflow-name>`)
walks them in declaration order. Today three step kinds are
supported:

- `tool_call` — runs end-to-end through the same dispatch path
  as the agent runner.
- `skill_invoke` — adds the named skill to the next agent turn
  (deferred — needs LLM transport, lands in a follow-up).
- `prompt` — injects a literal user message (deferred for the
  same reason).

Workflow files have their own schema; `argo vertical validate`
walks them.

### `[escalation]` — human approval gates

```toml
[escalation]
human_approval_required = ["delegate_task"]
escalate_after_failures = 2
escalate_to             = "logs"
```

Tools listed in `human_approval_required` **never dispatch**
without explicit operator approval — the runner emits an
`escalation.required` audit row + returns a clear error so the
agent surfaces the pause. Deny still wins over approval, so it's
safe to declare a tool in both lists as defence-in-depth.

`escalate_after_failures` + `escalate_to` are carried for
declarative completeness — the failure-count gate ships in a
follow-up.

### `[evaluation]` — the regression suite

```toml
[evaluation]
suites = ["eval/intent_classification.jsonl"]
```

Each suite is a JSONL file. Each line must be a JSON object
with at least `id`, `input`, and `expected` keys. `argo vertical
eval <id>` runs structural validation per row + tabulates fixture
counts so CI catches drift.

Live LLM-driven scoring (each fixture run through the agent +
graded against `expected`) is a separate command — out of scope
for the structural validator.

## The tooling tour

```bash
# Discovery — list everything in $ARGO_VERTICALS_DIR (or ./verticals)
argo vertical list
argo vertical list --json

# Show a single vertical's manifest + cross-refs
argo vertical show my-vertical
argo vertical show my-vertical --format markdown
argo vertical show my-vertical --json

# Validate cross-references (file references resolve, no
# duplicate skill names, etc.)
argo vertical validate my-vertical

# Eval-suite structural validation
argo vertical eval my-vertical
argo vertical eval my-vertical --json

# Run a workflow end-to-end (today: tool_call steps actually
# execute, skill_invoke + prompt steps record `deferred_no_llm`)
argo vertical workflow my-vertical greet --json

# Lifecycle: install a directory into the discovery dir
argo vertical install path/to/source --target ./verticals
argo vertical install path/to/source --force            # replace
argo vertical uninstall my-vertical --yes               # remove
```

Sessions bind via the existing `argo new` surface:

```bash
argo new --provider anthropic --model claude-sonnet-4-5 \
  --vertical my-vertical --title "first run"

# Filter session list by vertical
argo session list --vertical my-vertical --json
```

The gateway exposes a parallel surface (`GET /verticals`,
`POST /verticals/<id>/sessions`) so non-CLI clients can do the
same things without spawning a process per call.

## What you must NOT do

These are invariants. The reviewer will block PRs that break
them.

1. **Don't add `if vertical_id == ...` branches in core crates.**
   Vertical-specific behaviour belongs in the manifest /
   prompts / skills, never in the runtime. If you find yourself
   wanting to special-case a vertical, that's a sign the
   manifest schema needs a new field.

2. **Don't bypass the deny list.** Even with explicit operator
   approval, a tool in `[tools].denied` does not dispatch. If you
   need approval-gated dispatch, use `[escalation]`, not deny
   removal.

3. **Don't ship secrets in the manifest.** The whole tree is
   readable by the agent runner and the gateway. API keys go in
   `~/.config/argo/secrets.toml`, not `verticals/*/vertical.toml`.

4. **Don't store mutable state in the vertical dir.** Verticals
   are read-only at runtime — sessions, memory rows, audit
   records all live in the SQLite store. The vertical dir is the
   *template*; the running agent is the *instance*.

5. **Stay under 500 LOC per file** if you contribute to the
   reference verticals. The CI gate enforces it; split early
   rather than late.

## Recommended workflow when starting a new vertical

1. **Pick the closer of the two reference verticals.** PR-Checker
   for technical / structured / read-only agents; Receptionist
   for conversational / escalation-friendly agents.

2. **Copy + rename.** `cp -r verticals/receptionist verticals/my-vertical`
   then update `[vertical].id` and the dir name to match.

3. **Edit the manifest first.** Make it parse + validate before
   touching prompts. `argo vertical validate my-vertical` is your
   immediate feedback loop.

4. **Edit the eval suite second.** A small, real fixture set
   (5–10 rows) makes everything else easier — you stop guessing
   what the agent should do.

5. **Iterate on the system prompt + skills against the eval
   suite.** When the structural validator passes, the next layer
   is "does the agent actually behave per the prompt" — which is
   what the eval suite measures (live scoring lands in a follow-
   up; today the suite locks the input shape).

6. **Lock the security boundary.** Audit `[tools].denied` against
   the in-core tool set; add anything state-mutating you don't
   want. Add anything risky to `[escalation].human_approval_required`
   as defence-in-depth.

7. **Author tests.** The asset-test pattern from
   `crates/argo-cli/tests/vertical_pr_checker_assets.rs` is the
   canonical lock — it validates the manifest + the fixture
   shape against `argo_vertical::load_dir` so a future edit can't
   silently break it.

## Where to ask

- **Questions about platform invariants:** `CLAUDE.md` (root) +
  `docs/plan/argo-build-plan.md` §1.2.
- **Schema questions:** `crates/argo-vertical/src/manifest.rs`
  is the source of truth for `vertical.toml`;
  `crates/argo-vertical/src/workflow.rs` for `*.workflow.toml`.
- **Audit questions:** `crates/argo-audit/src/event.rs` lists
  every `kind` label the chain emits.

## The "Day-1" walkthrough

If you have 30 minutes and want to see the whole loop end-to-end,
do this:

```bash
# 1. Make a fresh tempdir
mkdir -p /tmp/argo-day1 && cd /tmp/argo-day1

# 2. Scaffold a minimal vertical
mkdir -p hello-vertical/{prompts,workflows,eval}
cat > hello-vertical/vertical.toml <<'EOF'
schema_version = "1"

[vertical]
id      = "hello"
name    = "Hello, world vertical"
version = "0.1.0"

[role]
system_prompt = "prompts/system.md"

[tools]
allowed = ["file_read"]
denied  = ["file_write", "file_patch", "terminal", "delegate_task"]

[workflows]
files = ["workflows/say.workflow.toml"]

[evaluation]
suites = ["eval/main.jsonl"]
EOF
echo "You are a friendly assistant. Reply in one sentence." \
  > hello-vertical/prompts/system.md

cat > hello-vertical/workflows/say.workflow.toml <<'EOF'
schema_version = "1"
[workflow]
name = "say"
[[step]]
name = "read_self"
kind = "tool_call"
tool = "file_read"
args = { path = "hello-vertical/vertical.toml" }
EOF

echo '{"id":"a","input":"hello","expected":{"intent":"greeting"}}' \
  > hello-vertical/eval/main.jsonl

# 3. Run the lifecycle commands
ARGO_VERTICALS_DIR=. argo vertical list
ARGO_VERTICALS_DIR=. argo vertical validate hello
ARGO_VERTICALS_DIR=. argo vertical eval hello --json
ARGO_VERTICALS_DIR=. argo vertical workflow hello say --json

# 4. Install + uninstall
argo vertical install hello-vertical --target ./installed --json
argo vertical uninstall hello --target ./installed --yes
```

Every command should succeed; the workflow output's first step
should be `status="ok"` with a preview of `vertical.toml`. If
not, `argo vertical validate` will tell you why.
