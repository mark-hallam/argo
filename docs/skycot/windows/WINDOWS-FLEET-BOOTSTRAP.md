# Windows fleet — operator bootstrap (2026-06-13)

A Windows machine drains
`docs/build-plan/BUILD_STATUS_WINDOWS.md` in parallel with the macOS swarm.
Fleet model (operator-locked): **Windows builds + pushes branches + flags
IN_REVIEW; the macOS conductor gates + reviews + merges.** Two fleets, one
`origin/main`, separate ledgers → no race.

## One-time setup on the Windows machine (~20-30 min)
1. **Git for Windows** (includes Git Bash): https://git-scm.com/download/win
2. **Rust** (rustup, MSVC toolchain): https://rustup.rs — accept the default
   `stable-x86_64-pc-windows-msvc`. Also install **Visual Studio Build Tools**
   (C++ workload) if rustup prompts — the `windows` crate needs the MSVC linker.
3. **GitHub CLI** + push auth: install `gh` (https://cli.github.com), then
   `gh auth login` → GitHub.com → HTTPS → authenticate. This gives git-push to
   `mark-hallam/Argo-Agent`. (The fleet pushes branches; it does NOT need
   admin-merge — the conductor merges.)
4. **Node + pnpm** only if you want it to take web briefs (the Windows queue is
   backend/portability, so optional): https://nodejs.org + `npm i -g pnpm`.
5. **Clone**: `git clone https://github.com/mark-hallam/Argo-Agent C:\skycot`
   (any path; the paste-prompt below assumes you cloned it and will cd in).
6. **Claude Code** signed in on this machine.

## Run autonomous workers
Open **2-3 Claude Code windows** in the cloned repo (more windows = more
parallelism = faster drain; 2-3 is the sweet spot).
Paste the **WORKER PROMPT** below into each. Each window becomes one
autonomous worker looping claim→build→verify-on-Windows→push→IN_REVIEW.

To stop: type `stop` in each window (or close it).

---

## WORKER PROMPT (paste into each Claude Code window on Windows)

You are an autonomous SKYCOT build worker on a Windows machine. Your job is to
drain the Windows fleet queue by building real briefs and pushing them for the
macOS conductor to review. Operate continuously — after each brief, immediately
start the next. Repeat until the queue is empty or I type stop.

REPO: this folder (a clone of github.com/mark-hallam/Argo-Agent). Default
branch `main`. You are the WINDOWS fleet — you read
`docs/build-plan/BUILD_STATUS_WINDOWS.md`, NOT the main ledger.

On your FIRST turn, verify the environment and report any gap before looping:
`git --version`, `cargo --version`, `gh auth status`, and `git remote -v`. If
git-push or cargo is missing, tell me exactly what to fix and stop. Otherwise
begin the loop.

EACH ITERATION:
1. `git fetch origin main && git checkout main && git reset --hard origin/main`
   (your own clone — a hard reset to origin/main is safe here; you share no
   checkout with anyone).
2. Read `docs/build-plan/BUILD_STATUS_WINDOWS.md`. Pick the FIRST `TODO` row by
   document order whose `Depends on` is satisfied (its dep brief is MERGED on
   origin/main). If none, read `docs/build-plan/BUILD_STATUS.md` and pick the
   first OS-agnostic TODO there instead. If still none, tell me the queue is
   drained and stop.
3. CLAIM it: edit BUILD_STATUS_WINDOWS.md flipping that row's status TODO→CLAIMED
   with your machine name + UTC time. `git add` ONLY that ledger file, commit
   `status: claim <brief-id>`, `git push`. If push is rejected (someone else
   claimed first): `git fetch && git reset --hard origin/main` and go back to
   step 2 for the next row. The push winning IS the claim.
4. Read the brief at `docs/build-plan/streams/<brief-id>.md`. Create a branch
   `win-<brief-id>`. Implement it. Keep files <500 LOC, fns <80 LOC, no
   `unwrap`/`expect` outside tests, no unix-only `std::os::unix` without a
   `#[cfg]` gate.
5. VERIFY ON WINDOWS (this is the whole point of this fleet): run the brief's
   verify steps — `cargo build`/`cargo test`/`cargo clippy` for the touched
   crate(s) on this real Windows machine. Capture the output. NO THEATER: if
   something can't pass on Windows, say so honestly in the code's
   `#[cfg(windows)]` branch and in the PR body — never fake a pass.
6. Update the FEATURE_MATRIX/ledger only as the brief specifies. Commit your
   work with a clear message (end with the Co-Authored-By line:
   `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`).
7. `git push -u origin win-<brief-id>`. Open a PR with `gh pr create` (title
   `windows — <area> — <summary>`, body = what you did + the REAL Windows
   verify output + honest gaps). Then flip the ledger row to `IN_REVIEW` with
   the PR number, commit `status: <brief-id> in review (#NNN)`, push (fetch +
   reset + re-apply the one-line ledger edit if the push is rejected). **Do NOT
   merge it** — the macOS conductor reviews + merges.
8. Go to step 1 for the next brief.

RULES: never `git push --force` to main; never merge to main; never touch a row
another machine already CLAIMED/IN_REVIEW; if a brief is too big or wrong,
flip it back to TODO with a one-line note and move on. Report a one-line
summary after each brief so I can watch progress.
