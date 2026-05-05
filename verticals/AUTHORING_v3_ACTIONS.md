# Authoring a vertical — Worker v3 actions

Companion to `AUTHORING.md`. Covers the `[actions]` section
verticals declare to participate in the Worker v3 execution
layer (#729 → #732). Read this once you've got a vertical
loading + linting clean and want it to propose external-world
actions through the OS's approval-gated state machine.

## What the section looks like

```toml
[actions.research_lead]
risk              = "low"
requires_approval = false
description       = "Research a warm lead — company, role, recent signals."
executor          = "human"

[actions.send_followup_email]
risk              = "high"
requires_approval = true
description       = "Send follow-up email. HIGH RISK — touches a prospect."
executor          = "human"
platforms         = ["gmail", "outlook"]
instruction_template = """
# Send follow-up — {{ payload.subject }} to {{ payload.recipient_email }}

⚠️ HIGH RISK — emailing a prospect; cannot be undone.
Approved by {{ approved_by }} at {{ approved_at }}.

Body (review BEFORE sending):
> {{ payload.body }}

When sent:
argo worker actions complete {{ action_id }} --note "sent"
"""
```

## Field reference

- **`risk`** — `low` / `medium` / `high`. Surfaced into the
  artifact bundle + (later) auto-approve budget logic.
- **`requires_approval`** — when `true`, the OS REFUSES to
  advance `pending → executing` without an explicit
  `argo worker actions approve` first. Enforced in
  `worker_actions_transitions::execute`, locked in
  `execute_refuses_pending_approval_gated_action`. **The
  model cannot override this. The prompt cannot bypass it.**
- **`description`** — surfaced in `argo worker actions list`.
- **`executor`** — currently only `human` is supported.
  Future: `browser_mcp`, `api_<vendor>`. Unknown values error
  loudly at execute time.
- **`platforms`** — optional. Lets the same action_type branch
  its instruction template per destination (`ebay` vs
  `facebook_marketplace`, `hubspot` vs `close`).
- **`instruction_template`** — optional. Markdown rendered with
  `{{ key }}` substitution at execute time. Supported keys:
  `action_id`, `action_type`, `job_id`, `risk_level`,
  `platform`, `item_id`, `approved_by`, `approved_at`, plus
  arbitrary `payload.<dotted.path>`. Unknown keys render as
  `(unset)` and lint emits a Warn.
- **`instruction_templates.<platform>`** — optional per-platform
  overrides. Precedence: per-platform → default → generic JSON
  dump.

## Three reference action vocabularies

The shipped Worker v3 verticals each declare a different action
set — copy the closest one to your domain:

| Vertical             | Autonomous-safe                                          | Approval-gated                                                                                                          |
|----------------------|----------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------|
| `gtm-engineer`       | `verify_price`, `request_photos`, `create_listing_draft` | `post_listing`, `boost_listing`, `update_price`, `relist_item`, `mark_item_sold`, `reply_to_buyer`, `follow_up_buyer`   |
| `ai-receptionist`    | `lookup_contact`, `draft_meeting_request`, `take_message`| `send_calendar_invite`, `forward_call`, `send_followup_email`                                                            |
| `sales-follow-up`    | `research_lead`, `find_contact`, `draft_followup`        | `send_followup_email`, `schedule_followup_cadence`, `log_to_crm`                                                         |

## The classification pattern that works

The pattern that holds across all three reference verticals:

- **Research / draft / message-capture is autonomous-safe.** The
  worker can do these in parallel without human review.
- **Anything that touches an external surface (calendar / inbox /
  CRM / public listing / buyer) is approval-gated.** No
  exceptions.
- **HIGH-RISK = touches another human + irreversible.** Email
  to a prospect, message to a buyer. These get the most-
  prominent warning banner in the rendered instruction template.

## Lock the safety boundary in a test

Every shipped Worker v3 vertical pins its action classification
in a regression test. Authoring a new vertical: add the same
shape under `crates/argo-cli/tests/vertical_<your-id>.rs`. A
drift between the system prompt's claim about safety and the
manifest's actual classification means an approval-gated action
silently became autonomous — which is exactly what the OS
exists to prevent.

Reference test:

```rust
#[test]
fn my_vertical_manifest_declares_expected_action_classification() {
    let dir = workspace_root().join("verticals");
    let (registry, _errs) = argo_vertical::load_dir(&dir).unwrap();
    let loaded = registry
        .get("my-vertical")
        .expect("my-vertical must be loaded");
    let actions = loaded.manifest.actions.as_ref().unwrap();
    let assert_class = |name: &str, requires_approval: bool| {
        let decl = actions.actions.get(name).unwrap();
        assert_eq!(decl.requires_approval, requires_approval);
    };
    // Pin EVERY action_type your manifest declares.
    assert_class("draft_thing", false);
    assert_class("send_thing", true);
    // ...
}
```

## Retry policy (#755)

Each action can optionally declare how many times the operator
is allowed to retry a failed instance:

```toml
[actions.verify_price.retry]
max_attempts          = 3              # original + 2 retries
backoff               = "exponential"  # hint only; not enforced
initial_delay_seconds = 30             # hint only; not enforced
```

Default when omitted: `max_attempts = 1` (no retries; a
`failed` row is fully terminal).

The OS enforces `max_attempts` in code by counting the retry
chain from the source action's `evidence` field — every
`argo worker actions retry` adds an `argo://action/<id>`
reference, so chain length tracks attempts directly. The
runtime refuses retries past the cap unless the operator
passes `--force` (which logs a `WARN` breadcrumb to stderr
for audit).

**Pattern that works across the shipped verticals:**

| Action shape                                            | Suggested `max_attempts` |
|---------------------------------------------------------|--------------------------|
| Web research / drafting (autonomous, low risk)          | 3                        |
| Public-surface posting (medium risk)                    | 2                        |
| Buyer / prospect / inbox-touching (high risk)           | 2                        |
| Spend or bid placement                                  | 1 (no retries)           |

The `gtm-engineer` vertical's manifest is the live reference —
`verify_price` retries 3×, `post_listing` and
`reply_to_buyer` retry 2× each.

`backoff` and `initial_delay_seconds` are surfaced into the
per-action bundle's `instructions.md` as a hint ("wait at least
N seconds before retrying"). Human-executable mode means the
operator paces retries; the runtime never blocks on a delay.

## Lint your templates

```bash
argo vertical lint <id>
```

Catches `{{ unknown_key }}` references in your instruction
templates that don't resolve to known top-level keys or
`payload.<dotted.path>` form. Warns; doesn't block load.

## Eval suites — locking behaviour with fixtures (#759)

Every vertical SHOULD ship an `[evaluation]` section pointing at
one or more JSONL eval suites:

```toml
[evaluation]
suites = ["eval/your_core.jsonl"]
```

Each line in a suite is a fixture:

```jsonc
{
  "id": "your_unique_id",
  "input": "the prompt fragment / scenario the worker receives",
  "expected": {
    // Any subset of the rule keys below. Keys are independent;
    // each one runs as a separate ScoreCheck.
    "must_include":           ["substring 1", "substring 2"],
    "must_not_include":       ["forbidden phrase"],
    "abstain":                true,   // refuse for thin evidence
    "must_refuse_or_defer":   true,   // refuse / defer to operator
    "external_mutation":      false,  // must NOT claim send/post/update
    "hallucination":          false,  // no fabricated funding / launches
    "pattern_detected":       true,   // pattern-recognition language present
    "skill_proposal":         false,  // proposes a learned skill
    "personalization_quality": "high"
  }
}
```

`argo vertical eval <id>` validates structure (every row has
`id`/`input`/`expected`). `argo vertical eval <id> --score`
additionally runs the deterministic rule engine against each
fixture's `actual` field if present — otherwise the row reports
`no-actual` and is skipped. Live LLM runs populate the actuals;
CI runs the structural check + the rule engine over a known-good
actuals file in the repo.

The Worker v3 cohort (`gtm-engineer`, `ai-receptionist`,
`sales-follow-up`) each ship suites covering:

- the autonomous-safe / approval-gated split (worker emits
  `requires_approval` rows correctly for high-risk actions);
- the evidence-gate (thin research → emission downgraded to
  `blocked`);
- undeclared-action rejection (worker can't grant itself
  capabilities the manifest didn't declare);
- the abstain contract on under-grounded personalisation;
- `external_mutation: false` everywhere so a regression that
  has the worker claim "I sent the email" gets caught
  structurally.

Author your fixtures to assert the same shape — the rule engine
is deterministic and runs without an API key.

## Operator-side surface (what the customer runs)

After your vertical declares its actions + instruction
templates, the operator's CLI surface is the same as for every
other vertical:

```bash
argo work add --vertical <your-id> --prompt "..."
argo work serve --vertical <your-id> --once
argo worker actions list --state all
argo worker actions approve <id>
argo worker actions execute <id>     # → renders your instruction_template
argo worker actions complete <id> --note "..."
argo worker doctor
```

Your job as a vertical author is to ensure the manifest + system
prompt produce sensible action emissions. The operator surface
+ safety primitives are platform-level — you don't need to wire
any of it.
