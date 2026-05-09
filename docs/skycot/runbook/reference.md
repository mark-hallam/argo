# SKYCOT Operator Runbook — Reference

Where to read next, the CLI command index, and the customer-demo
file checklist. For the full table of contents, see
[RUNBOOK.md](../RUNBOOK.md).

---

## 10. Where to next

- **The mission, in full**: [MISSION.md](../MISSION.md). Read this
  to understand the six load-bearing claims SKYCOT is built
  against.
- **The 14-day swarm protocol**:
  [../../build-plan/PROTOCOL.md](../../build-plan/PROTOCOL.md). How
  multiple agent families (Claude Code, Codex) built this
  repo concurrently in 14 days.
- **Active follow-ups**: see
  [BUILD_STATUS.md](../../build-plan/BUILD_STATUS.md) for in-flight
  briefs. The v1.x roadmap items that didn't make the swarm:
  - Federated OCI marketplace for the skill library tier
  - `argo session start/list/show` cli (in flight as
    `H-orchestrator-cli`)
  - WebSocket audit stream (`claude-realtime-audit-stream`)
  - Per-vertical proposal thresholds (manifest field)
- **Contributing**: PRs welcome. The brief format + protocol
  are documented in `docs/build-plan/PROTOCOL_BRIEF_TEMPLATE.md`
  + `PROTOCOL.md`. Start with a small, self-contained slice;
  the swarm has shown that 100–400 LOC PRs are the right size.
- **Demo recording**: TODO — once the cli for
  `argo session start` lands, the v1 demo gate (the
  marketing-intern → sales-sdr → audit walkthrough) becomes
  scriptable and we'll attach the recording here.

---

## Appendix A — Cli command index

Every command shown above is in v1 main. Run `argo --help` for
the full tree, or `argo <subcommand> --help` for a specific
group. The 25 top-level commands are:

<!-- quickref:commands:start -->
```
argo new           argo resume        argo chat          argo steer
argo agents        argo gateway       argo doctor        argo health
argo db            argo workspace     argo audit         argo setup
argo vertical      argo genesis       argo messages      argo cron
argo work          argo worker        argo costs         argo eval
argo forge         argo judge         argo mcp           argo memory
argo model         argo plugin        argo provider      argo route
argo session       argo skill         argo trace         argo tui
argo tools
```
<!-- quickref:commands:end -->

`argo worker actions <list|show|approve|execute|complete|fail|block|retry>`
is the hot-path family for driving the action ledger. Genesis
+ doctor + worker actions + audit are the four families an
operator touches daily.

## Appendix B — File checklist for a customer demo

Before showing this to a customer, verify:

- [ ] `argo --version` returns a v1.x string
- [ ] `argo doctor` reports 🟢 ready
- [ ] `argo genesis create --charter "..."` writes a complete
      tree
- [ ] `argo vertical lint <id>` passes
- [ ] An emitted action goes pending → approved → executing →
      completed cleanly
- [ ] `argo audit query --since-minutes 5` shows the trail
- [ ] `argo doctor --json` includes a non-zero
      `auto_approval_today` if you've configured a policy
- [ ] (Optional) `cargo test -p argo-cli --test e2e_v1_demo`
      passes — this is the e2e gate that exercises the same
      paths from above against an in-memory store

If all eight tick, the platform really does what MISSION.md
promises.
