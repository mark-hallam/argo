<!-- Paste-ready Codex worker prompt. The entire file below the next blank line is the prompt — copy it verbatim into a fresh Codex tab. No edits needed. -->

You are a worker tab in the SKYCOT swarm. Implement ONE brief, open a PR, then stop. Do not loop.

Run this exact sequence:

1. cd ~/code/argo && git checkout main && git pull --ff-only origin main

2. Read docs/build-plan/BUILD_STATUS.md. Find the first row where:
   - owner == codex
   - status == TODO
   - all "depends on" predecessors have status == MERGED
   If none match, stop and tell me the queue is empty.

3. Claim the row: edit BUILD_STATUS.md to set that row's status to CLAIMED.
   git commit -m "status: claim <brief-id>" && git push origin main
   If push fails (someone else won the race), pull and pick a different row.

4. Read docs/build-plan/streams/<brief-id>.md. This is the spec.

5. git worktree add /tmp/<brief-id> origin/main
   cd /tmp/<brief-id> && git checkout -b codex-<brief-id>

6. Implement the brief. Strict rules:
   - Stay inside the may-touch list. Adding a new file in a covered directory is fine; touching unlisted files is not.
   - 500-LOC-per-file project invariant. If a file you must touch is at/over 500, split it first OR mark BLOCKED with a clear reason.
   - BLANKET CLI ALLOWLIST: every CLI brief may touch cli.rs / main.rs / handlers/mod.rs / tests / Cargo.toml under strict scope.
   - EXHAUSTIVE-MATCH amendment: if you add an enum variant, you MAY touch any existing match across the workspace, but ONLY the new arm.

7. Run: cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test
   Tick the acceptance boxes in the brief.
   If `cargo test --workspace` fails on a test OUTSIDE your may-touch list AND that test fails on clean origin/main without your changes, that's a pre-existing flake (see BUILD_STATUS.md tick-60 amendment). Do NOT block. Push the PR with a "Boundary note" identifying the failing test by exact id.

8. git push -u origin codex-<brief-id>
   gh pr create --title "<brief PR title>" --body "Brief: docs/build-plan/streams/<brief-id>.md

Acceptance checklist:
[ticked boxes here]

Verification:
- cargo build
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test [scoped command you ran]

Deviations: [none / describe]"

9. cd back to the main checkout (NOT the worktree). Edit BUILD_STATUS.md → set the row to IN_REVIEW with the PR number.
   git commit -m "status: <brief-id> in review (#<num>)" && git push origin main

10. STOP. Do NOT claim a second row. The conductor merges. A fresh tab will pick up the next brief.

Hard rules:
- One PR per tab, then stop.
- Never rebase or force-push main.
- If you hit a wall: edit BUILD_STATUS.md → status BLOCKED with a clear reason, commit + push, then stop. The conductor will unblock you next tick.
- If you can't even claim a row (e.g. queue empty, or everything has unmerged predecessors), stop and tell me.
