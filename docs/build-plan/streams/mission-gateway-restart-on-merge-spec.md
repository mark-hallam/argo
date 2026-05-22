# mission-gateway-restart-on-merge-spec

**Owner**: claude.
**Depends on**: none (this is a SPEC brief — the implementation lands in a follow-up brief).
**Authored against**: `origin/main` `d2be56888` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-restart-on-merge-spec`.

🔴 **P0 SHIPPING-VS-DEPLOYED GAP.** Source: evaluator bootstrap (`docs/skycot/evaluator/scenarios/`) + auditor T03-39 §3.3 / §4 P0 #4. Live gateway PID 28198 reports `schema_version: 70`; code on `origin/main` is at v76. ~30 PRs of new functionality (workflow rollback, sticky-notes, sub-workflows, http+code nodes, hire-banner) are not in the running binary. The demo MP4 path walks through new workflow routes that 404 against this stale gateway. Even after the cookies-in-client-bundle and `;`-in-SQL-comment P0s land, the demo still 404s without a restart policy.

## Scope

This is a **SPEC brief**, not an implementation brief. Output is a markdown decision document the operator approves; the actual restart-policy implementation lands in a follow-up brief named in this doc.

The worker investigates and documents three options, picks the recommended one, and produces a one-page decision doc at `docs/skycot/operator/gateway-restart-policy.md`:

**Option A — auto-restart on merge.** The merge step (currently `gh pr merge --squash --delete-branch --admin`) is followed by a deploy hook that restarts the gateway. Pros: zero operator action; demo path always current. Cons: more failure modes (restart races merge commits; bad merges immediately impact live service).

**Option B — surface drift in conductor tick prompt.** The conductor's tick prompt reads the live gateway's `/healthz?include_schema_version=1` (or equivalent) and compares it to `git rev-parse origin/main` schema version. If drift > N migrations, the tick prompt flags it loudly in the §9 summary and the conductor asks the operator to restart. Pros: keeps merge fast; operator stays in control. Cons: drift can accumulate silently between conductor ticks.

**Option C — scheduled bounce.** A cron (every hour? every 6h?) restarts the gateway unconditionally. Pros: simple, predictable. Cons: restart-during-demo risk; doesn't react to merge bursts.

The decision doc captures: chosen option, rationale, the one-line implementation tag (e.g. "follow-up brief: `mission-gateway-auto-restart-hook`" or "follow-up brief: `mission-conductor-gateway-schema-drift-surface`"), and rollback (how the operator turns it off if it causes more harm than good).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: docs/skycot/operator/gateway-restart-policy.md` — the one-page decision doc.
- `docs/skycot/operator/README.md` — narrowly scoped: ONE link entry pointing to the new doc.
- `docs/build-plan/MISSION_BACKLOG.md` — narrowly scoped: flip the `[NEW]` tag on the source backlog item (if one exists) to `[BRIEF: mission-gateway-restart-on-merge-spec]`. If no such item, skip this file.

## Files you must NOT touch

- Any source code (Rust, TypeScript, scripts) — this is a spec-only brief.
- The conductor tick prompt — the recommendation may include a tick-prompt edit, but that's the IMPLEMENTATION's job, in the follow-up brief.
- Any new GitHub Actions workflow file — same reasoning.

## Acceptance criteria

- [ ] `docs/skycot/operator/gateway-restart-policy.md` exists.
- [ ] Doc covers all three options (A / B / C) with pros + cons in concrete terms (no vague "more reliable", be specific about failure modes).
- [ ] Doc names ONE recommended option with a one-sentence justification.
- [ ] Doc names the follow-up implementation brief's expected ID and ~LOC range.
- [ ] Doc includes a "rollback" section: how the operator disables the policy if it goes wrong.
- [ ] Doc has the freshness header (`Authored against` line).
- [ ] PR description names the operator decision point (the rec is a proposal; operator may pick a different option, which produces a different follow-up brief).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-gateway-restart-on-merge-spec`
- PR title: `docs(operator): gateway-restart-on-merge policy spec (mission-gateway-restart-on-merge-spec)`

## Notes for the implementer

- ~100-150 LOC (one markdown file).
- The CURRENT gateway-stale state on the operator's machine is "PID 28198 at schema_version 70 vs code v76". Use that as the concrete example throughout the doc — it grounds the abstract problem in a real number.
- Don't try to pick a winner the operator wouldn't pick. If you're unsure between Options A and B, name both as "viable" and let the operator break the tie. The point of a spec brief is to surface the trade-offs, not pre-commit the operator.
- Don't propose **Option D** (custom approach) — three options is the right number for a one-page doc.
- The follow-up implementation brief will be authored by the conductor in the next replenishment pass, **after** the operator picks an option.
