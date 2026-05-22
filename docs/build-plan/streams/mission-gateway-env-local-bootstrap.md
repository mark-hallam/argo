# mission-gateway-env-local-bootstrap

**Owner**: claude.
**Depends on**: `mission-gateway-restart-executable-runbook` MERGED (✅ via #2043).
**Authored against**: origin/main `ff0e664e6` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-env-local-bootstrap`.

🟡 **P2 OPERATOR-ONBOARDING FOOTGUN.** Source: real operator incident 2026-05-23 (multi-hour debug session). First-time gateway setup after a fresh DB hit a chain of footguns:

1. **macOS Keychain prompt** for `skycot.db.encryption` blocked launch — operator didn't know the login-keychain password (Mac password / Keychain password drift after Apple-ID reset).
2. **No documented bypass** — conductor had to grep the codebase to find `SKYCOT_DB_KEY_BASE64` env var.
3. **No persistence** — operator generated a key inline (`$(openssl rand -base64 32)`), gateway launched fine, but the random key was lost the moment the shell exited. Next restart → DB unreadable.
4. **Manual fix-string had wrong length** — a conductor-suggested literal "32-byte" string was actually 42 bytes, rejected by gateway startup check.
5. **`make restart-live-gateway` no-ops on schema-current** — even after env.local was created, the no-op optimization left the OLD random-key gateway running, with env.local key not yet in effect.

Total operator time lost: ~2 hours. **Every other operator setting this up cold will hit the same chain.**

This brief delivers a first-run bootstrap path in `make restart-live-gateway` (or a sibling `make first-run-setup` target) that handles all 5 footguns automatically.

## Scope

Add a first-run bootstrap to the gateway launch path. Two acceptable implementations:

**Option A — auto-bootstrap inside `make restart-live-gateway`** (preferred): the script detects that `~/.argo/env.local` doesn't exist + Keychain access would prompt, and generates a fresh env.local automatically before launching.

**Option B — sibling `make first-run-setup` target**: explicit one-time bootstrap the operator runs once. Then `make restart-live-gateway` assumes env.local exists and sources it.

Either approach, the bootstrap must:

1. **Detect first-run state**: `~/.argo/env.local` doesn't exist OR doesn't contain `SKYCOT_DB_KEY_BASE64`.
2. **Generate a 32-byte DB key**: `openssl rand -base64 32` (verify length decodes to exactly 32 bytes before writing — fail loud otherwise).
3. **Prompt the operator for an LLM API key**: read `ANTHROPIC_API_KEY` from stdin or detect that one of `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` is already exported in the parent shell.
4. **Write `~/.argo/env.local` with `chmod 600`**:
   ```
   export SKYCOT_DB_KEY_BASE64='<generated-key>'
   export ANTHROPIC_API_KEY='<operator-provided-key>'
   ```
5. **Print a clear next-step hint**: `Source it with: source ~/.argo/env.local && make restart-live-gateway`.

Also update the `scripts/restart-live-gateway.sh` script (or `Makefile` target) so that:

6. **`make restart-live-gateway` sources `~/.argo/env.local` automatically** if it exists. Removes the need for operators to remember `source ~/.argo/env.local &&` prefix.

7. **The no-op-on-schema-current optimization (current step 2) gets a `--force` flag** to allow operators to override and force a restart even when schema is current. Use case: switching from one-time inline key to persistent env.local key.

8. **Update `docs/skycot/operator/gateway-restart-policy.md`** with the new bootstrap section + first-run quickstart.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `scripts/restart-live-gateway.sh` — extend with first-run detection + auto-source of env.local + `--force` flag.
- `Makefile` — narrowly scoped: ONLY if adding a sibling `first-run-setup` target.
- `NEW: scripts/restart-live-gateway-first-run-tests.sh` (or inline test block) — at minimum: smoke test that running the bootstrap on a clean system creates env.local with a valid 32-byte key.
- `docs/skycot/operator/gateway-restart-policy.md` — narrowly scoped: add "First-run setup" section + update the "Quick reference" example.
- `~/.argo/env.local.example` (NEW, committed to repo) — example file showing the expected env.local shape with placeholder values. Operators copy this to `~/.argo/env.local` if they prefer manual setup.

## Files you must NOT touch

- The gateway binary's source — this brief is operator-tooling only, not gateway-internal.
- `crates/argo-config/src/cred/db_key.rs` — the key-resolution code is correct (env var → Keychain → generate). The footgun is operator UX, not config code.
- The auto-generate-and-persist-to-Keychain path in `db_key.rs` — leave it; it's the right default when Keychain works. The bootstrap brief is the escape hatch for when Keychain is unusable.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] Running `make restart-live-gateway` on a system where `~/.argo/env.local` doesn't exist AND `SKYCOT_DB_KEY_BASE64` env var is unset triggers the bootstrap: generates a key, prompts for the API key, writes env.local with 0600 mode.
- [ ] Running `make restart-live-gateway` when `~/.argo/env.local` EXISTS sources it automatically before launching (operator no longer needs the `source ... &&` prefix).
- [ ] Adding `--force` flag forces a restart even when schema is current. Verify with `make restart-live-gateway --force` while a gateway is running.
- [ ] The generated SKYCOT_DB_KEY_BASE64 always decodes to exactly 32 bytes — verify via test that decodes the file's value and asserts length.
- [ ] If the operator provides an API key < 20 chars or that doesn't start with `sk-ant-` or `sk-`, print a warning but accept it anyway (it might be a custom proxy or future provider).
- [ ] `~/.argo/env.local.example` exists in the repo with placeholder values + comments explaining each var.
- [ ] `docs/skycot/operator/gateway-restart-policy.md` has a "First-run setup" section at the top covering: (a) the auto-bootstrap path, (b) how to recover from a lost DB key (move sessions.db aside + restart), (c) how to rotate the API key safely (edit env.local, no restart needed since gateway has it in memory).
- [ ] PR body includes a "What changed for first-time setup" section with the before/after operator journey (link to this brief's "Source" section).
- [ ] Smoke test passes against a temp directory simulating first-run (no `~/.argo/env.local`, no Keychain).
- [ ] No regression: an operator with an EXISTING valid `~/.argo/env.local` sees the same behavior as today + the auto-source convenience.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-gateway-env-local-bootstrap`
- PR title: `ops(gateway): first-run env.local bootstrap + auto-source + --force (mission-gateway-env-local-bootstrap)`

## Notes for the implementer

- ~150-250 LOC expected. Bash script extension is ~100, doc update ~50, tests ~30, example file ~10.
- **Detection heuristic for "would-prompt-Keychain"**: not strictly necessary to implement. The simpler rule is "if `~/.argo/env.local` doesn't exist AND `SKYCOT_DB_KEY_BASE64` env var not in environment, run bootstrap". The Keychain prompt would happen during gateway boot if neither source is present.
- **Validating the 32-byte key**: `echo "$KEY" | base64 -d | wc -c` should equal 32. If not, regenerate. Don't ship a 42-byte fix-string as the default; my mistake in this incident.
- **Don't auto-prompt for the API key on every `make restart-live-gateway`** — only on first run when env.local is missing. After that, the file should have what's needed.
- The `--force` flag is small but high-leverage: when an operator wants to switch from an inline one-time key to a persistent env.local key, they need a way to force the restart. The no-op optimization currently blocks that.
- **Test against a temp $HOME**: run the script with `HOME=/tmp/argo-bootstrap-test bash scripts/restart-live-gateway.sh` to simulate a fresh user. This avoids polluting the real `~/.argo/`.
- The operator's terminal-paste-comment-lines pattern (`# Comment\ncommand`) keeps tripping up bash. The script's stdout messages should avoid emitting `#`-prefixed lines that could confuse copy-paste.
- **Document the recovery flow**: "If you ever need to wipe and re-bootstrap: `rm ~/.argo/env.local ~/.argo/sessions.db && make restart-live-gateway`". Make sure this loses no important data the operator might want (just the runtime DB).
- This brief came out of a real ~2-hour operator incident. The PR body should include a "Forensic timeline" appendix briefly listing the 5 footguns the operator hit, so reviewers understand the motivation and future operators reading the PR understand what got fixed.
