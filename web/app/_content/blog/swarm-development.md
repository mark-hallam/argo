---
title: Sixteen PRs in one session — how we develop SKYCOT with a swarm
date: 2026-05-13
summary: The development model behind SKYCOT is a swarm of coding agents working in parallel against a shared ledger. Here is how it actually works in practice.
author: Mark Hallam
---

{/* REVIEW-REQUIRED: prose */}

A few weeks ago we landed sixteen pull requests against the SKYCOT main branch in a single working session. None of them were trivial — they touched runtime code, migrations, the marketing site, the eval harness, and the type system. None of them were written by a single human sitting at a keyboard.

They were written by a swarm of coding agents working in parallel against a shared ledger. This post is about how that works.

## The basic loop

Three pieces:

1. **A queue of briefs.** Each brief is a markdown file describing one piece of work — what to change, which files may be touched, what acceptance looks like, what not to break. Briefs are authored by a "conductor" agent that has read the current state of the codebase.

2. **A ledger.** A single markdown file (`BUILD_STATUS.md`) lists every brief and its current state: `TODO`, `CLAIMED`, `IN_REVIEW`, `MERGED`, `BLOCKED`. The ledger lives on the main branch. To claim a brief, an agent commits a one-line edit to the ledger row.

3. **Worker agents.** Each worker runs an iteration loop: read the ledger from main, find the first TODO row whose dependencies are merged, claim it, implement it inside the brief's may-touch list, push a PR, mark the row `IN_REVIEW`, exit.

The conductor reviews PRs, merges what's green, marks rows `MERGED`, replenishes the queue.

## Why this works

A few things make this loop hold together:

**The ledger is the source of truth.** Not memory, not chat history. If a worker comes back after compaction and "remembers" that it was working on brief X, it doesn't matter — it re-reads the ledger from disk, and the ledger says what the world looks like now.

**Briefs are scoped tightly.** A may-touch list of ~5 files is normal. Workers that wander outside that list get their PRs rejected at review time. This is what makes parallelism safe: two workers touching disjoint file sets cannot create a merge conflict.

**Race-loss is a first-class case.** Two workers race to claim the same brief on the ledger. The loser's `git push` to main fails with a non-fast-forward error. The loser doesn't retry — it re-reads the ledger, picks a different TODO row, and claims that one. Race-loss is normal, expected, and handled in the worker loop's first ten lines.

**Pre-existing flakes don't block.** If a test that was flaky before the worker's change is flaky after the worker's change, the PR ships with a "Boundary note" listing the failing tests. The worker doesn't burn time trying to fix tests outside its scope.

## What goes wrong

The honest answer is: most of the failure modes you'd expect.

**Drift.** A worker authors code against a brief that was true three days ago but no longer matches main. The fix is a freshness gate — every brief has a `Validate before claim` line that runs a script verifying the brief still applies to current main before the worker can claim it.

**Parallel abstractions.** Two workers introduce two structs with the same name in different crates. We have a brief-linter that greps for `pub struct <Name>` across the workspace before a brief can be claimed, and we've made re-export-from-`argo-types` the preferred pattern when in doubt.

**Stuck briefs.** A worker tries three times to make a brief pass clippy + tests and can't. It marks the row `BLOCKED` with a one-line reason and exits. The conductor sees `BLOCKED` rows at the top of its next tick and either rewrites the brief or escalates.

## Why we're shipping this way

We're a small team building a product whose mission is, basically, "make AI workers reliable enough to trust." It would be strange not to dogfood that. The swarm is not a stunt — it's how we found out which parts of agent reliability are real and which are vibes. We've broken the loop in dozens of ways. We've fixed it almost as many times. The protocol that runs it now is the third major version.

If you want to see the protocol, it lives in our repo at `docs/build-plan/SWARM_PROTOCOL.md`. It's about 400 lines. It is, as far as we know, the most concrete operating manual for multi-agent development that anyone has published.

We will keep iterating on it. We will probably keep posting about it.
