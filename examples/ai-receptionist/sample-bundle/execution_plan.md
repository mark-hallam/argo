# Execution plan

Emitted 3 action(s) — 1 pending approval, 2 autonomous-safe, 0 blocked.

## Pending approval

These actions require an explicit `argo worker actions approve <id>` before they advance.

- `019df917-4321-7777-aaaa-2222220c0003` — `send_calendar_invite` (risk: medium) · platform `google_calendar` · item `mark-chen-acme`

## Pending (autonomous-safe)

Autonomous-safe actions — the worker may advance these without operator approval per the bound vertical's manifest.

- `019df917-4321-7777-aaaa-2222220c0001` — `lookup_contact` (risk: low) · item `mark-chen-acme`
- `019df917-4321-7777-aaaa-2222220c0002` — `draft_meeting_request` (risk: low) · item `mark-chen-acme`
