# ops-gateway-restart-after-2026-05-25-relight

**Owner**: claude or codex.
**Authored against**: origin/main `bec35dd45` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-gateway-restart-after-2026-05-25-relight`.

🟡 **P1 — v1.0 launch remaining item (b) from CLAUDE.md "Remaining for v1.0 launch" list**

CLAUDE.md item (b): "**agent**: restart gateway (`launchctl list com.skycot.gateway` shows last exit 0; no process running; nothing listening on port 8088)". Confirmed at T07-40 conductor tick: `lsof -nP -iTCP:8088 -sTCP:LISTEN` returns no listener; `launchctl list | grep skycot.gateway` shows `-` PID + last exit 0.

The gateway being down blocks: web SPA dev-mode against the real backend, any cold-rerun regression tests that hit the gateway HTTP surface, claim-1/4/6 LIVE-test reruns.

## Scope

1. **Inspect current state** — capture before-state:
   - `launchctl list | grep skycot.gateway`
   - `lsof -nP -iTCP:8088 -sTCP:LISTEN` (or whatever port is in the plist)
   - Tail the log file — read the plist (`~/Library/LaunchAgents/com.skycot.gateway.plist`, or `/Library/...` depending on tier) for `StandardOutPath` + `StandardErrorPath` to find the actual log location.

2. **Identify shutdown reason** — read the last ~100 lines of the gateway log. Is it: clean shutdown? migration failure? port-bind error? SQLite lock? Note the answer in the PR body.

3. **Restart** —
   - Primary: `launchctl kickstart -k <domain>/com.skycot.gateway` (domain is `system` or `user/<uid>` depending on where it's loaded; check with `launchctl print-disabled <domain>` or just `launchctl list`).
   - Fallback: `launchctl bootout <domain>/com.skycot.gateway && launchctl bootstrap <domain> <plist-path>`.
   - If the binary path in the plist points at a stale build — rebuild via `cargo build --release --bin argo` and re-bootstrap.

4. **Verify** — wait up to 60s post-restart, then:
   - `lsof -nP -iTCP:8088 -sTCP:LISTEN` should show the gateway PID.
   - `curl -fsS http://localhost:8088/health 2>&1 | head -3` should return 200 with a JSON body.
   - `bash scripts/check-gateway-freshness.sh` should report `FRESH` with `gateway_schema=v81 codebase_latest=v81 drift=0` (or v82+ if the schema bumped since this brief was authored).

5. **Report** — PR body must include:
   - Before-state: launchctl output + lsof output + log tail (~20 lines).
   - Recipe used to restart (kickstart or bootout/bootstrap or rebuild+bootstrap).
   - After-state: launchctl output + lsof output + curl /health output + check-gateway-freshness.sh output.
   - Any plist edits required (rare — only if binary path or port was wrong).

## Acceptance criteria

- `lsof -nP -iTCP:8088 -sTCP:LISTEN` returns at least one row (the gateway).
- `curl -fsS http://localhost:8088/health` returns HTTP 200 OK.
- `bash scripts/check-gateway-freshness.sh` exits 0 with `FRESH` status.
- PR body has full before/after state per the Report step above.

## Worker result 2026-05-25

- Interpreted the listener check on the repo-canonical gateway port `7400`:
  `Makefile`, `scripts/restart-live-gateway.sh`, and
  `scripts/check-gateway-freshness.sh` all default to
  `http://127.0.0.1:7400`; the `8088` references in this brief are stale.
- Ran `launchctl kickstart -k gui/$(id -u)/com.skycot.gateway`; launchd
  executed `make restart-live-gateway` from `~/code/argo`.
- The restart helper detected a current gateway at schema v81 and exited
  cleanly as a no-op. Post-checks show PID 37695 listening on
  `127.0.0.1:7400`, `/health` returning schema v81, and
  `scripts/check-gateway-freshness.sh` reporting `FRESH`.
- No plist changes were required.

## May-touch

- `Library/LaunchAgents/com.skycot.gateway.plist` (ONLY if the plist itself is stale; if so, also copy the change into `scripts/launchd/com.skycot.gateway.plist` if a canonical copy exists in the repo, and explain why in PR body)
- This brief file (status updates)

## Out of scope

- Modifying any gateway Rust code (`crates/argo-cli/src/handlers/gateway/`)
- Modifying any migration
- Changing the port from 8088 (file a separate brief if that's needed)

## Effort

~20-40 minutes (10 min to read state + 10-30 min depending on diagnosis difficulty).
