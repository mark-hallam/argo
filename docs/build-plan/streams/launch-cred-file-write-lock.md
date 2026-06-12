# launch-cred-file-write-lock

**Owner**: claude or codex — reliability on a security-critical file.
**Depends on**: none (the tg2 #2846 review follow-up, announced but never rowed).
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh launch-cred-file-write-lock

## Problem (tg2 adversarial review 2026-06-12)

`cred/io.rs` `set_tenant` does an unlocked load-modify-write
(load_document → mutate → truncate-write) of `credentials.toml` — two
concurrent connector-authorize calls can lost-update (last writer wins,
another tenant's just-saved credential silently dropped). Not a leak
(each write only adds its own section) but a data-integrity hole on the
file holding every tenant's connector credentials, now reachable
concurrently via the tg2 gateway route.

## Fix

Serialize writers: file-lock (fs2/flock on a sidecar .lock) or a process
mutex around the read-modify-write in `cred/io.rs` (single-process gateway
→ a process-wide tokio/std Mutex is sufficient and simpler; document why).
Atomic replace (write temp + rename) instead of truncate-in-place so a
crash mid-write never leaves a torn file. Keep 0600 perms.

## Acceptance
1. Concurrency test: N parallel set_tenant writers, all sections present
   afterwards (would FAIL/flake today). 2. Torn-write test: temp+rename
   path leaves either old or new, never partial. 3. clippy clean.

~40-90 LOC. **🟠 P2 — credential durability under the new HTTP surface.**
