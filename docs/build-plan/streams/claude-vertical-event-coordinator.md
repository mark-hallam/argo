# V2.3 — Event Coordinator demo vertical

**Owner**: claude. **Depends on**: G-genesis (#790),
claude-vertical-finance-ar (#937),
C-gcal + C-gmail merged. Vertical demo #11.

## Scope

Event Coordinator demo vertical — proves the platform
handles multi-vendor coordination flows where most actions
have material spend or guest-list implications. Persona
"Sage" handles event ops:
- Schedules venue bookings (always human-gated — venues
  involve deposits)
- Sends invites and tracks RSVPs
- Confirms vendors (catering, AV, photo)
- Generates run-of-show timeline docs
- Posts event recaps post-event

High-value demo because (a) events are a clean "spans many
vendors" coordination story, (b) the heavy human-gate
profile shows the platform isn't just for low-stakes
automation, (c) generate_run_of_show is a great example of
auto-approved "compose from gathered facts" workflow.

## What lands

11 NEW files (no NEW crate — re-use existing executors):

- `verticals/event-coordinator/vertical.toml` — manifest
  (persona=Sage, allowed_executors=[gcal, email,
  document_creation, human_approval])
- `verticals/event-coordinator/persona.toml` — voice
  (organized, calm under deadline, tactful)
- `verticals/event-coordinator/system_prompt.md` — duties,
  budget guardrails, vendor handling rules
- `verticals/event-coordinator/memory_schema.toml` —
  event_id, vendor_status, guest_list, budget_state,
  timeline_log
- `verticals/event-coordinator/eval/seeds.jsonl` — 10
  scenarios (small dinner / corporate offsite / wedding /
  vendor cancellation / weather contingency)
- `verticals/event-coordinator/inventory/sample_vendors.json`
  — 8 fake vendors across catering/AV/photo/florals
- `verticals/event-coordinator/skills/run_of_show.md` —
  template + buffer-time heuristics
- `verticals/event-coordinator/workspace/auto_approval.toml`
  — generate_run_of_show only; rest gated
- `verticals/event-coordinator/actions/` — 5 action defs:
  - `schedule_venue.toml` (NotifyChannel.Human)
  - `send_invites.toml` (NotifyChannel.Email)
  - `confirm_vendors.toml` (NotifyChannel.Human)
  - `generate_run_of_show.toml` (auto)
  - `post_event_recap.toml` (NotifyChannel.DocumentCreation)
- `bin/demo_event_coordinator.sh` — end-to-end demo run
- `docs/verticals/event-coordinator.md` — operator doc

## Acceptance criteria
- [ ] Vertical bootstraps via Genesis from charter
- [ ] Demo script runs end-to-end (mocked vendor APIs OK)
- [ ] All 5 actions exercise
- [ ] schedule_venue + confirm_vendors gate to human (real
      money implication)
- [ ] generate_run_of_show produces a markdown timeline
- [ ] post_event_recap produces a markdown recap doc
- [ ] E2E smoke passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Total LOC ≤ 700 across the 11 files

## Branch + PR
- Branch: `claude-vertical-event-coordinator`
- PR title: `verticals: Event coordinator demo vertical (claude-vertical-event-coordinator)`

## Notes
- This vertical has the HEAVIEST human-gate profile of the
  10 demos — only 1 of 5 actions auto-approves. Intentional:
  events are decision-dense and operators want sign-off.
- send_invites uses Email channel (not Human) because
  invites are templated and the guest list is pre-approved
  during event setup
- 8 sample vendors mix categories so demo can show
  cross-category coordination
- post_event_recap is a great showcase for the
  document_creation executor — it composes a narrative
  from memory state without external calls
