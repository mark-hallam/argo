# ops-host-setup-runbook-path-canary

**Authored against:** `origin/main` e92343c85 on 2026-05-23.
**Validate before claim:** `bash scripts/check-brief-freshness.sh ops-host-setup-runbook-path-canary`.

## Why this brief exists

Surfaced as P2 finding #5 from the 2026-05-23T09:40Z scheduled audit. The mac-host doom-loop incident (tick-96 → tick-99b, ~40 minutes, ~91 commits in 20 min) was root-caused to a host PATH defect: the supervisor was launched from a shell whose PATH lacked `~/.local/bin`, so every `claude --dangerously-skip-permissions ...` invocation died with `command not found`, releasing each claim. Fix landed in commit `7e9b1426a` (worker-loop + supervisor PATH guards) + `~/.zshenv` export.

The commit body has the diagnostic; the audit calls for promoting it to a discoverable runbook so the next host migration is 30 seconds, not 40 minutes.

## Acceptance criteria

1. Create `docs/build-plan/HOST_SETUP.md` (~30-50 lines), OR extend `docs/build-plan/CONTRIBUTING_AGENT.md` with a new "Host setup canary" section.
2. Document explicitly:
   - The PATH-canary line at `scripts/claude-worker-loop.sh:274` (the guard added in `7e9b1426a`).
   - The `~/.zshenv` export pattern (or equivalent for bash users) — why interactive `~/.zshrc` is insufficient for supervisor subshells.
   - How to confirm the canary is active: `which claude` from a non-interactive subshell should return a path, not error.
   - How to verify in 30 seconds before launching the supervisor: `bash -c "which claude" && bash -c "echo $PATH"` from a fresh terminal.
3. Cross-link from `docs/build-plan/RESUME.md` so fresh agents see it during onboarding.
4. Cross-link from `scripts/claude-worker-loop.sh` (a comment at line ~274 pointing to the runbook).

## Parallel-surface declarations

- No new code, no new pub surface. Docs-only.
- Existing related docs: `docs/build-plan/RESUME.md`, `docs/build-plan/CONTRIBUTING_AGENT.md`, `docs/build-plan/SWARM_PROTOCOL.md`, `docs/build-plan/KNOWN_FLAKES.md`. This brief extends the onboarding chain.

## May-touch

- `docs/build-plan/HOST_SETUP.md` (NEW) — or extend `CONTRIBUTING_AGENT.md`
- `docs/build-plan/RESUME.md` (one cross-link line)
- `scripts/claude-worker-loop.sh` (one comment line at the canary, no code change)

## Boundary

- Do NOT change `claude-worker-loop.sh` behaviour — the fix already shipped. This is documentation only.
- Do NOT introduce a "host doctor" script — that's a separate brief if scoped. This brief is the runbook companion, not the automation.
- Do NOT touch the supervisor binary's PATH handling — `7e9b1426a` already covers it.

## Sizing

~30-50 LOC (markdown). Fast-mode candidate.

**🟡 P2 (operational resilience)** — cheap insurance for next host migration. The same 40-min outage on a production demo day would be unacceptable.
