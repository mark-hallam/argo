# Vertical: event-coordinator

V2.3 demo vertical (Stream `claude-vertical-event-coordinator`).
Demo #11 — multi-vendor event coordination with the heaviest gate
profile in the catalogue. Four of five actions human-gate; only
`generate_run_of_show` runs autonomously. See also `healthcare-intake`,
`finance-ar`, `real-estate-agent` for related v2.3 verticals.

## Persona

Sage — event coordinator at **Skycot Events** (synthetic demo agency).
Organized, calm under deadline, tactful with vendors. Sage's job is
to coordinate the venue + vendor + invite + run-of-show + recap loop
without ever binding money or widening recipient lists without an
operator's approval.

## Action surface

- `schedule_venue` — **human-gated**. Holds or books a venue; deposits
  and cancellation windows bind real money so the operator names the
  slot, deposit, and cancel-by date.
- `send_invites` — **human-gated** (Email channel). Templated invite
  blast against the pre-approved guest list. Operator approves
  subject + body; the action never widens the recipient list.
- `confirm_vendors` — **human-gated**. Confirms catering / AV / photo
  / florals with a quoted price; refuses on category mismatch,
  capacity-exceeded, over-budget, or unknown vendor.
- `generate_run_of_show` — **autonomous-safe**. Composes a markdown
  timeline from confirmed `bookings` + `vendor_status` memory plus
  the buffer-time heuristics in `skills/run_of_show.md`. No external
  send, no money, no calendar mutation.
- `post_event_recap` — **human-gated** (DocumentCreation channel).
  Composes a shareable recap; refuses to include vendor pricing.

## Memory schema

`events`, `bookings`, `vendor_status`, `guest_list`, `budget_state`,
`timeline_log`. Pointers + summary state — the operator's financial
ledger and the venue/vendor contracts live outside the system. See
`memory_schema.toml`.

## Money handling

Every venue + vendor confirmation binds real money. `budget_state`
tracks the running ceiling per event. `confirm_vendors` refuses to
push the event over its operator-approved ceiling without a fresh
approval naming a new ceiling. Vendor pricing **never** appears in
guest-facing emails or in recap documents — pricing belongs in the
operator-only ledger.

## Demo

```sh
./scripts/demo_event_coordinator.sh
```

The demo copies the vertical into a tempdir workspace, runs
`argo vertical doctor`, and grep-checks the doctor output for
vendor `rate_card` URL leaks. The script exits non-zero if any
pricing fixture string surfaces at INFO level. No external API
keys required.

## Approval gates

Four of five actions are gated. `generate_run_of_show` is the single
autonomous-safe action — composing from gathered facts is safe when
the gating risk is brand-voice or money, not factual fidelity.
`workspace/auto_approval.toml` caps daily auto-spend at $0.50 total
+ $0.10 per action.

The demo's thesis: **decision-dense workflows still benefit from the
platform**. Even when 80% of actions need a human gate, the gated
actions still get drafted, validated against memory, and routed
consistently — and the auto-approved 20% (run-of-show composition)
is exactly the boring-but-tedious work an operator wants off their
plate.

## Bootstraps via Genesis

No dedicated `EventCoordinator` archetype yet — the closest match in
`crates/argo-genesis/src/archetype.rs` is `CalendarCoordinator` (slot
booking is the shared dominant action). A dedicated archetype is a
v3 candidate. Classify with `argo genesis classify --charter "..."`.

## Deferred follow-up (out of scope)

- Real venue / vendor portal integrations (Eventbrite, Tripleseat).
  Sample inventory only today.
- RSVP inbox sync — guest_list rollups are operator-driven for now.
- Weather feed integration — weather contingencies escalate to the
  operator as a generic surface, not a triggered re-plan.
- Dedicated `EventCoordinator` Genesis archetype.
