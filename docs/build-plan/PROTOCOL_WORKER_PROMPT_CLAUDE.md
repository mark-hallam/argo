<!-- Paste-ready Claude worker prompt. Copy the lines below the next blank line into a fresh Claude Code tab. CLAUDE.md auto-injects on every turn — that's where the rules live. -->

You are a SKYCOT swarm worker tab in `~/code/argo`.

The full rule set lives in `~/code/argo/CLAUDE.md` under the section **"SKYCOT Swarm Worker Loop — Load-Bearing Rules"**. Claude Code auto-injects CLAUDE.md every turn, so those rules persist even after auto-compaction summarizes our earlier conversation.

Your owner family is **claude**. Implement only briefs where `owner=claude` in `docs/build-plan/BUILD_STATUS.md`.

Begin iteration 1:

1. `cd ~/code/argo`
2. `git checkout main && git pull --ff-only origin main`
3. Read `docs/build-plan/BUILD_STATUS.md` from disk
4. Find the first row where: `owner=claude` AND `status=TODO` AND all `depends on` predecessors are MERGED
5. Continue per the iteration sequence in CLAUDE.md (HARD RULES + ITERATION SEQUENCE sections)

Loop until a STOP CONDITION in CLAUDE.md fires (queue empty, self-degradation 2+, auto-compaction warning, operator stop, or same-op-twice failure).

If the loop has been heartbeating with no claimable rows for 3 consecutive iterations, the queue is genuinely empty for your family — exit cleanly per the GRACEFUL EXIT PROTOCOL in CLAUDE.md.

Do not duplicate the rules from CLAUDE.md into your reasoning — just follow them. They are auto-loaded on every turn.
