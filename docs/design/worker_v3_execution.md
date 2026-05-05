# Worker v3 — Generic Execution Layer

**Status:** design (#728 ledger landed; #729-#732 in progress).
**Audience:** anyone touching `argo-cli/src/handlers/worker_actions.rs`,
`argo-cli/src/handlers/work_steps.rs`, or a vertical's manifest /
prompt.

## Why this doc

#728 shipped an action ledger whose `ActionType` enum hard-coded
ten marketplace-specific variants (`post_listing`, `boost_listing`,
…). That was useful as a seed but **wrong as a design**. Worker v3
is the OS execution layer. It must support every vertical Argo
ships now (gtm-engineer, pr-checker, receptionist) and every
vertical it ships later, without recompiling the OS to add
action types.

This doc refactors the v3 execution model so:
- the **OS layer** owns: states, transitions, the ledger,
  approval gating, the artifact bundle shape, the CLI surface
- the **vertical layer** owns: which action types it produces,
  what payload each carries, which need approval, what risk
  level each is, how each renders into a human-executable
  artifact

The marketplace-seller flow becomes one vertical mapping among
many.

## Goal

Turn Argo from *"AI that produces plans"* into *"AI workers that
execute real-world tasks under controlled approval"* —
generically, across every vertical, with the same OS substrate.

---

## 1. Core Worker v3 Model (OS Layer)

### Execution states

Generic for every job and every action. The job moves between
overall states; each action moves between its own action-level
states (see §1.3).

| State | Meaning |
|---|---|
| `draft_plan` | Worker is producing the plan + action proposals (steps 1..N of the multi-step loop). |
| `approval_required` | Plan has at least one pending action that requires human approval. |
| `approved_for_execution` | At least one action has been approved; worker can execute the safe + approved set. |
| `executing` | Worker is currently running approved actions (one at a time, in declared order). |
| `blocked` | Pre-flight failed (e.g. research insufficient — #727) OR an action's prerequisite blocked. Terminal for THIS run; operator retries by adding a fresh job. |
| `completed` | All declared actions either `completed` or explicitly `rejected`. |
| `failed` | Execution attempted but errored unrecoverably (transport failure, platform refused, etc.). Operator retries by creating a new job. |

### Job-level transitions

```
                ┌─────────────┐
                │  draft_plan │
                └──────┬──────┘
                       │ worker emits action proposals
                       ▼
              ┌────────────────────┐
              │ approval_required  │◄─────┐
              └─────────┬──────────┘      │ new approval-gated
                        │                 │ action emitted
                        │ operator        │ later
                        │ approves at     │
                        │ least one       │
                        ▼                 │
              ┌────────────────────┐      │
              │approved_for_       │──────┘
              │execution           │
              └─────────┬──────────┘
                        │ argo worker actions execute
                        ▼
              ┌────────────────────┐
              │     executing      │
              └─────────┬──────────┘
                        │
       ┌────────────────┼────────────────┐
       ▼                ▼                ▼
 ┌──────────┐    ┌────────────┐    ┌─────────┐
 │completed │    │   failed   │    │ blocked │
 └──────────┘    └────────────┘    └─────────┘
```

### Action lifecycle (per-action)

This is the per-action sub-machine inside the job, already
shipped in #728 (`ActionState::can_transition_to`):

```
 pending ──→ approved ──→ executing ──→ completed
   │           │              │           ▲
   │           └──────────────┴───────────┘  (synchronous shortcut)
   │                                ↓
   │                              failed
   ↓
 blocked  (terminal)
 rejected (terminal — operator declined)
```

Rules (locked by `ActionState::can_transition_to` + tests):

- **`pending` is the only initial state.** All actions enter
  here, regardless of whether they require approval.
- **`approved → executing` is the ONLY path to execution.**
  Cannot skip approval if `requires_approval = true`.
- **Cannot execute twice.** `completed` and `failed` are
  terminal for the action; the writer rejects re-transitions.
- **`blocked` is terminal.** A blocked action never executes.
  Downstream actions whose prerequisites include a blocked
  action also block.
- **`rejected` (#730 will add this transition)** lets an
  operator explicitly decline an action. Different from
  `blocked` — operator made a positive choice.

### Action model — the universal record

Locked in #728 (`crates/argo-cli/src/handlers/worker_actions.rs`):

```rust
pub(crate) struct ActionRecord {
    pub schema_key: String,        // "worker_action"
    pub action_id: String,         // UUIDv7, stable across transitions
    pub job_id: String,            // links to pending_jobs.id
    pub item_id: Option<String>,   // optional — for items the action
                                   // operates on (a listing, a thread, …)
    pub action_type: String,       // FREE-FORM string — vertical-defined
                                   // (see §5)
    pub state: String,             // ActionState as_str
    pub payload: Value,            // proposed action body — vertical-shaped
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
    pub executed_at: Option<String>,
    pub platform: Option<String>,  // optional destination
    pub result: Option<Value>,     // execution outcome
    pub evidence: Vec<String>,     // source URLs / artifact paths
    pub blocked_reason: Option<String>,
    pub created_at: String,        // when THIS row was written
}
```

#### Two fields the v3 refactor adds (delivered in #729)

`ActionRecord` already has the structural fields. #729 adds:

- **`risk_level: RiskLevel`** — `low` / `medium` / `high`. The
  vertical declares this in its action manifest (§5). The OS
  uses it for sorting / filtering and for surface UI hints in
  `approval_queue.md`.
- **`requires_approval: bool`** — already implicit via the
  ActionType enum in #728; #729 makes it explicit per-action
  in the persisted record AND derived from the vertical's
  action manifest, not from a hardcoded OS-side enum.

The `ActionType` enum from #728 is **deprecated in #729** — it
moves from the OS to the gtm-engineer vertical's manifest as a
seed example. The OS just sees `action_type: String`.

---

## 2. Approval Boundary System

Three categories. Enforced by the OS reading the bound vertical's
manifest, not by prompts.

### A. Safe autonomous actions

Worker may transition `pending → approved → executing →
completed` autonomously. `requires_approval = false` in the
vertical's manifest.

Examples (across verticals):
- gtm-engineer: `define_icp`, `research_company`,
  `analyze_campaign` (read-only / draft-only)
- marketplace seller: `create_listing_draft`, `verify_price`,
  `request_photos`
- receptionist: `triage_inbound_message`, `summarise_thread`
- pr-checker: `review_diff`, `flag_concern`

### B. Approval-gated actions

Worker may transition `pending` only. Any further transition
requires an explicit operator action via
`argo worker actions approve`. Cannot fire without that approval.

Examples:
- marketplace seller: `post_listing`, `boost_listing`,
  `reply_to_buyer`, `mark_item_sold`, `update_price`,
  `relist_item`
- gtm-engineer: `send_outreach`, `propose_gtm_skill_promotion`
- receptionist: `send_reply`, `escalate_to_human`
- pr-checker: `auto-merge_pr`, `request_changes`

### C. Forbidden actions (no capability grant)

Action types that exist in the vertical's vocabulary but are
NOT executable by the OS at all in the current build. They land
as `blocked` immediately on emission. Operator must explicitly
grant a capability (a future feature, not in the v3 stack) for
the OS to honour them.

Examples:
- any action that would spend money beyond a configured budget
- any action against a platform we haven't wired (initially:
  every platform — see §7)
- any action whose `risk_level = high` and the operator hasn't
  explicitly opted into letting the worker propose them

### Enforcement (NOT prompt-only)

The OS enforces this at write time. The action emitter (the
worker, in #729) calls into a checker:

```rust
pub fn classify_for_emission(
    vertical: &Manifest,
    action_type: &str,
) -> Result<ApprovalCategory, EmissionError>
```

- `ApprovalCategory::SafeAutonomous` → action lands `pending`
  with `requires_approval = false`. Worker's outer loop may
  transition it through `approved → executing → completed` in
  the same run.
- `ApprovalCategory::Approval` → action lands `pending` with
  `requires_approval = true`. Worker MUST stop at
  `approval_required` for this action.
- `ApprovalCategory::Forbidden` → action is rejected at
  emission time. Persisted as `blocked` with
  `blocked_reason = "no capability grant for action_type 'X'"`.

The worker's prompt restates these rules for the model's
benefit, but the model can't bypass them — the writer rejects
emissions that violate the manifest.

---

## 3. CLI Design (generic)

#### `argo worker actions list` — already shipped (#728)

Filters by state / job / vertical / action_type / platform /
limit. Default state is `all`.

#### `argo worker actions show <action_id>` — already shipped (#728)

Latest snapshot for one action.

#### `argo worker actions approve <action_id> [--note "..."]` — #730

Append a row with `state = "approved"`, `approved_by = $USER`,
`approved_at = now()`. Validates `pending → approved` is legal
+ that action's `requires_approval` matches expectation.
Refuses if action is already terminal.

```sh
$ argo worker actions approve 019df-a-001
approved 019df-a-001 (post_listing → facebook_marketplace)
```

#### `argo worker actions approve-batch <job_id> [--state-filter <state>]` — #730

Batch-approve every pending action for a job. Default filter:
`pending`. Prints each action approved + a summary count.

```sh
$ argo worker actions approve-batch 019df-job-1
approved 4 of 5 pending actions for job 019df-job-1
  019df-a-001 post_listing facebook_marketplace
  019df-a-002 post_listing ebay
  019df-a-003 post_listing gumtree
  019df-a-004 reply_to_buyer
skipped:
  019df-a-005 update_price (blocked: pricing not verified)
```

#### `argo worker actions execute <action_id>` — #730 / #731

Validates `approved → executing` is legal. Calls the vertical's
executor for that action_type (which produces the human-
executable artifact in v1, browser/API call in v2 — see §7).
On success: appends `state = "completed"` row. On error:
appends `state = "failed"` with the error in `result`.

Refuses if action is `pending` (must approve first), already
`completed/failed/blocked`, or executed before.

```sh
$ argo worker actions execute 019df-a-001
executing 019df-a-001 (post_listing on facebook_marketplace)
→ wrote .skycot/outputs/<job_id>/executions/019df-a-001/
  - listing_copy.md
  - platform_url.txt
  - photos_required.txt
  - instructions.md
status: completed (human-executable mode)
```

#### `argo worker actions complete <action_id> --result <json>` — #731

Operator-side completion record. After they paste the listing
copy and click Post on FB Marketplace, they tell the ledger
what happened:

```sh
$ argo worker actions complete 019df-a-001 --result '{"posted_at":"2026-05-05T16:00Z","listing_url":"https://fb..."}'
recorded outcome for 019df-a-001
```

This is the "human-executable mode" complement to `execute`. In
v2 (browser-automation), `execute` records the result directly;
`complete` becomes optional.

#### `argo worker status <job_id>` — #730

One-screen status for a job:

```sh
$ argo worker status 019df-job-1
job 019df-job-1
  vertical:        gtm-engineer
  job state:       approval_required
  steps completed: 2/8
  stop_reason:     approval_required
  evidence:        sufficient (3 hits, 1842 chars extracted)
  not_ready_to_post: false

actions:
  total:     7
  completed: 2  (verify_price, request_photos)
  pending:   4  (3 post_listing, 1 reply_to_buyer)
  blocked:   1  (post_listing on tiktok — no capability grant)

artifacts:
  research.md            ✓ written (1.4 KB)
  pricing.md             ✓ written (820 B)
  listings.md            ✓ written (3.1 KB)
  approval_checklist.md  ✓ written
  execution_plan.md      ✓ written
  approval_queue.md      ✓ written
  completed_actions.md   ✓ written (2 entries)
  blocked_actions.md     ✓ written (1 entry)

next:
  argo worker actions approve-batch 019df-job-1
```

---

## 4. Artifact System (generic)

The v3 artifact bundle, in addition to the v2 set
(`transcript.md`, `assistant_output.md`, `research.md`,
`pricing.md`, `listings.md`, `approval_checklist.md`, `job.json`,
`result.json`), adds:

### `actions.json`

The full ledger snapshot for this job. Same shape as
`argo worker actions list --job-id <X> --json`, just bundled
with the run's artifacts so an operator can grep without
needing the CLI:

```json
[
  {"action_id":"019df-a-001","action_type":"post_listing","state":"pending","requires_approval":true, ...},
  {"action_id":"019df-a-002","action_type":"verify_price","state":"completed", ...}
]
```

Written at the end of every run + after every
approve / execute / complete CLI call.

### `execution_plan.md`

Operator-readable plan: the ordered list of actions the worker
proposed, grouped by step, with risk level + approval status
visible at a glance. Generated when the worker emits actions
(#729).

```markdown
# Execution plan — job 019df-job-1

## Step 1 — research + draft (autonomous)
- ✓ verify_price (low risk, completed)
- ✓ request_photos (low risk, completed)

## Step 2 — listing drafts (autonomous → approval-gated)
- ⏸ post_listing on facebook_marketplace (medium, awaits approval)
- ⏸ post_listing on ebay (medium, awaits approval)
- ⏸ post_listing on gumtree (medium, awaits approval)

## Step 3 — buyer comms (approval-gated)
- ⏸ reply_to_buyer (high, awaits approval)
```

### `approval_queue.md`

Just the actions awaiting approval, with the exact
human-executable payload preview. The doc the operator opens
when they're ready to click "approve".

```markdown
# Approval queue — job 019df-job-1

## 019df-a-001 — post_listing → facebook_marketplace
**Risk:** medium
**Title:** Pro Skates 110mm — lightly used
**Body preview:** "Selling my Pro Skates 110mm, AU. Lightly used …"
**Photos required:** 3
**Suggested price:** $250
**Sources for price:**
  - https://example.com/comp1
  - https://example.com/comp2

To approve: `argo worker actions approve 019df-a-001`
```

### `completed_actions.md`

Outcomes log. Each completed action gets a section with what
happened and any operator-supplied result data.

### `blocked_actions.md`

Why blocked actions blocked. The OS-side `blocked_reason` and
any vertical-supplied detail.

All artifact files generated by deterministic renderers from
the action ledger — the operator can re-render after every
state transition without re-running the worker.

---

## 5. Vertical Binding Layer

A vertical declares its action vocabulary in a new section of
its `vertical.toml` manifest (#729 adds the schema):

```toml
[actions]

[actions.create_listing_draft]
risk = "low"
requires_approval = false
description = "Draft a marketplace listing (title + body + tags) without posting."
payload_schema = "schemas/create_listing_draft.json"
executor = "human"  # in v1; "browser_mcp" / "api" later

[actions.post_listing]
risk = "medium"
requires_approval = true
description = "Post a previously-drafted listing to a marketplace platform."
payload_schema = "schemas/post_listing.json"
executor = "human"
platforms = ["facebook_marketplace", "ebay", "gumtree"]

[actions.reply_to_buyer]
risk = "high"
requires_approval = true
description = "Send a reply to a buyer message."
payload_schema = "schemas/reply_to_buyer.json"
executor = "human"
```

The OS at startup (or per-job) loads the vertical's manifest
and uses it to:
1. Validate every action emission against `[actions.<name>]`
2. Stamp `risk_level` + `requires_approval` on the persisted
   action record (no inference, no override)
3. Reject emissions for action_types not declared
4. Render the human-executable artifact via the per-action
   schema + the vertical's templates (§7)

Verticals without an `[actions]` section behave exactly like
v2 — they produce the four sectioned deliverables but no
ledger entries. v3 is opt-in per-vertical.

### Risk-level semantics

- **low** — read-only / draft-only / file-side-effect-only.
  Safe to autonomously execute. Default category for
  research / planning / drafting actions.
- **medium** — touches a public surface but reversible
  (post a listing → can delete it; update a price → can revert).
- **high** — touches another human (a buyer reply, an
  outbound email) OR is irreversible (mark sold, transfer
  funds). Requires approval AND a human-executable preview
  showing the exact payload.

The OS uses risk for surface UI ordering and (later) for
budget gating ("auto-approve everything ≤ medium up to
$50/day").

---

## 6. Marketplace Seller Mapping

The skate-resale flow maps cleanly into the generic frame.
Marketplace-seller is one vertical (the same one as
gtm-engineer in this build, since gtm-engineer's scope already
includes marketplace work; future verticals may split it out).

### Action manifest entries

| action_type | risk | requires_approval | platform-specific | executor (v1) |
|---|---|---|---|---|
| `create_listing_draft` | low | no | yes (FB/eBay/Gumtree shape) | human |
| `verify_price` | low | no | no | human |
| `request_photos` | low | no | no | human |
| `post_listing` | medium | yes | yes | human |
| `boost_listing` | medium | yes | yes | human |
| `reply_to_buyer` | high | yes | yes (per-platform DM shape) | human |
| `follow_up_buyer` | high | yes | yes | human |
| `mark_item_sold` | medium | yes | yes | human |
| `update_price` | medium | yes | yes | human |
| `relist_item` | medium | yes | yes | human |

### Payload examples

```json
// post_listing payload
{
  "platform": "facebook_marketplace",
  "title": "Pro Skates 110mm",
  "body": "Lightly used pair of Pro Skates...",
  "category": "Sporting Goods",
  "price_aud": 250,
  "photos": ["./photos/skates-front.jpg", "./photos/skates-side.jpg"],
  "location": "Sydney NSW",
  "tags": ["inline skates", "rollerblades", "sports"]
}

// reply_to_buyer payload
{
  "platform": "facebook_marketplace",
  "thread_id": "fb-thread-abc",
  "buyer_handle": "John D.",
  "message": "Hi John — yes still available! Happy to do $230 if you can pick up this weekend.",
  "context": "Buyer asked if still available + counter-offered $200"
}
```

### Sample run for "sell these skates"

1. `argo work add --vertical gtm-engineer --prompt "Sell these
   skates"`
2. Worker enters `draft_plan`, runs research (web_search ×N,
   web_extract ×N), evidence gate passes (#727)
3. Worker emits actions in step 1: `verify_price` (low),
   `create_listing_draft` (low) × 3 platforms
4. Worker auto-executes `verify_price` →`completed`
5. Worker auto-executes the three `create_listing_draft` →
   `completed`; per-platform draft text persisted
6. Worker emits actions in step 2: `post_listing` × 3 platforms
   (medium, requires_approval=true). Job state →
   `approval_required`
7. Operator runs `argo worker status` → sees the queue
8. Operator runs `argo worker actions approve-batch <job_id>` →
   the three `post_listing` actions move to `approved`
9. Operator runs
   `argo worker actions execute <action_id>` per action →
   for each, the OS renders the per-platform listing copy +
   exact platform URL into the artifact bundle, transitions
   the action to `executing`
10. Operator pastes the copy into FB Marketplace (or whichever
    platform), clicks Post
11. Operator runs `argo worker actions complete <action_id>
    --result '{"posted_at":"...","listing_url":"..."}'` → the
    action transitions to `completed`
12. `actions.json` + `completed_actions.md` updated

In v2 (browser-automation), steps 9-11 collapse into a single
`execute` call — same ledger, same artifact bundle, just the
executor changes.

---

## 7. Execution Mode v1 — Human-Executable

Because no browser MCP / marketplace API is wired yet, the v1
executor for every action_type is **human**. The worker's job
is to produce a complete, ready-to-paste artifact that the
operator runs manually.

For each action, the executor must produce (under
`.skycot/outputs/<job_id>/executions/<action_id>/`):

- **`listing_copy.md` (or equivalent)** — exact text the
  operator pastes
- **`platform_url.txt`** — the URL where the operator goes to
  paste it
- **`required_inputs.md`** — what the operator must supply
  manually (photos, location, contact info)
- **`instructions.md`** — step-by-step "do this, then that"
- **`evidence.md`** — source URLs + reasoning for any factual
  claims in the listing

The vertical's manifest declares the templates that render
each action_type's payload into these files. Templates live
under the vertical's source tree; the OS just invokes them.

When the operator records `complete`, the result is captured
verbatim in the action record's `result` field for later
analysis (which platforms converted, which copy worked, etc.).

---

## 8. Future Execution Mode — Connectors

The same action ledger drives:

- **Browser automation (next).** A new executor, `browser_mcp`,
  invokes the existing `mcp-servers/browser/` server to drive
  the operator's logged-in browser. The action's payload becomes
  the MCP call args; the outcome (post ID, URL, error) lands in
  `result` directly. **No ledger schema change.**
- **Marketplace APIs (later).** For platforms with proper APIs
  (eBay sandbox, Etsy, Shopify), executors talk to the API
  directly. Same ledger; the executor's name is just `api` or
  `<platform>_api`.
- **Multi-step automation.** A single action can have a longer
  executor that chains internal sub-steps (open browser → log
  in → navigate → fill → submit). The ledger sees one action
  with its outcome; the executor's internal trace lives in
  `result` for replay.

The contract that doesn't change:
- The OS owns the state machine + the approval gate
- The vertical owns the action vocabulary + templates
- Executors are pluggable behind a Rust trait — `human`,
  `browser_mcp`, `api` are all impls

This is what "lift human-executable to automated" means
without changing the worker's logic.

---

## 9. Test Plan

Tests live alongside the code that implements each piece. Naming
shows what the test locks.

### OS-layer (in argo-cli)

- `action_state_round_trips_through_string` — wire format
  stable [#728 ✓]
- `action_state_terminal_set_is_completed_failed_blocked` —
  terminal predicate [#728 ✓]
- `action_state_transition_matrix_matches_design` — locks
  every legal + illegal transition [#728 ✓]
- `action_record_round_trips_through_json_with_defaults` —
  serde shape matches on-disk [#728 ✓]
- `list_returns_latest_snapshot_when_action_has_multiple_state_rows`
  — append-only contract [#728 ✓]

### Approval boundary (#729)

- `vertical_action_manifest_round_trips_through_toml`
- `emission_against_undeclared_action_type_is_rejected`
- `emission_against_safe_autonomous_action_lands_pending_no_approval`
- `emission_against_approval_gated_action_lands_pending_with_approval_required`
- `emission_against_forbidden_action_lands_blocked`

### Approval CLI (#730)

- `approve_legal_pending_action_writes_approved_row`
- `approve_already_terminal_action_is_rejected`
- `approve_batch_only_picks_pending_actions_for_job`
- `execute_unapproved_approval_gated_action_is_rejected_with_clear_reason`
- `execute_already_executed_action_is_rejected`
- `execute_blocked_action_is_rejected`
- `complete_writes_result_into_latest_action_row`

### Execution mode v1 (#731)

- `human_executor_writes_listing_copy_platform_url_required_inputs_instructions_evidence`
- `executor_template_render_uses_payload_schema`
- `multi_action_run_writes_actions_json_plus_section_artifacts`

### End-to-end (drives `argo worker` CLI)

- `marketplace_seller_full_flow_skate_resale_synthetic` —
  drives `work add` → `serve --once` (mock) → `worker actions
  list` → `approve` → `execute` → `complete`; asserts the
  ledger has the right shape at each point and the artifact
  bundle matches the design above
- `blocked_research_prevents_post_listing_emission` — when
  evidence is insufficient (#727), `post_listing` actions
  emit as `blocked`, not `pending`
- `multi_step_persistence` — if the worker's process crashes
  mid-run, restart picks up at the right action

---

## Critical constraints

These are immutable through the v3 stack:

- **Generic OS, vertical-specific action types.** The OS never
  hardcodes an action_type name.
- **No UI.** Every operation is CLI. UI is a future product
  surface (skycot-web).
- **Ledger is canonical.** Every artifact is regenerable from
  the action ledger.
- **500-LOC budget per file.** Test-size growth lives in
  sibling test files, not in the modules being tested.
- **Backwards-compatible with Worker v2 artifacts.** v2
  bundles (`research.md`, `pricing.md`, etc.) keep working;
  v3 adds new files alongside them.
- **No worker authority broadens.** v3 still cannot post,
  message, spend, or commit funds without the operator
  explicitly approving each action.

---

## Implementation roadmap

| PR | Scope | Status |
|----|-------|--------|
| #728 | Action ledger + state machine + read-side CLI | ✅ merged |
| **#729** | **Vertical action manifest schema; worker emits action proposals; `execution_plan.md` artifact; deprecate the OS-side `ActionType` enum.** | **next** |
| #730 | Approval CLI (`approve` / `approve-batch` / `execute` / `complete` / `status`); `approval_queue.md` + `completed_actions.md` + `blocked_actions.md` artifacts | after #729 |
| #731 | Human-executable executor v1; per-action artifact bundle; gtm-engineer's marketplace mapping | after #730 |
| #732 | Marketplace-specific renderers (FB / eBay / Gumtree templates); per-platform payload schemas | after #731 |
| #733+ | Browser-automation executor (uses the existing browser MCP server); same ledger | future |

This roadmap turns Argo from "AI that produces plans" into "AI
workers that execute real-world tasks under controlled
approval", one PR at a time, without ever broadening the
worker's authority.
