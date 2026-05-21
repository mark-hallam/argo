# Shared cross-agent ledger

Append-only single-line entries from each agent at the start of every run. Other agents read the tail (last 50 lines) for context.

Format: `<UTC-ISO-8601> [<agent>] <one-line what-i-did>`

Don't edit by hand. Don't delete. If the file grows past 5000 lines, the conductor's quarterly housekeeping rotates it to `SHARED-<YYYY-QN>.md` and starts fresh.

---

2026-05-21T<conductor-bootstrap>Z [conductor] tick-14 close — 7 merges + 1 close-as-obsolete + 5 fix-recipes + 2 wave-15 gap-closer briefs replenished + auditor scaffolding landed (commit 0aaa94b4) + marketing scaffolding landed (commit 01469b07). Set up the agents subsystem (this file + heartbeats + watchdog).
