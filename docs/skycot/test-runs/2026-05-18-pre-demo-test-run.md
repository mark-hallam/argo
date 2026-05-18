# 2026-05-18 pre-demo test run — results

**HEAD**: `a491acb1` (test-run started from)
**Operator**: mark-hallam
**Run by**: claude conductor
**Date**: 2026-05-18
**Build**: release binary at `/tmp/argo-shared-target/release/argo` (`bundled-sqlcipher` feature on)
**SQLCipher version**: 4.5.7 community

---

## Headline

🟢 **The platform is in much better shape than the operator's earlier dry-run suggested.** The dry-run failed 10/12 scenes because of dev-environment config gaps (services not running, env vars unset) — not because the platform code is broken.

🚨 **But the dry-run uncovered 2 real product bugs that are demo-recording blockers**, both now fixed in working tree (not committed):

1. **3 verticals failed to load** due to stale `[[scope]]` schema (parser expects `[[entry]]`). The 3 broken verticals: `marketing-intern`, `customer-support`, `sales-sdr`. **marketing-intern is the vertical the demo's first-hire payload uses.**
2. **Demo first-hire fixture used wrong schema** — `workerDraft` (camelCase) instead of `worker_draft` (snake_case) and missing required fields (`suggestedName`, `suggestedVertical`, `approvalPolicy`, `duties`, `tagline`).

After both fixes, **worker creation via the demo path returns HTTP 201 cleanly**.

---

## Phase-by-phase results

### Phase 1: Build & binary health — ✅ PASS
- `cargo build --release -p argo-cli --features argo-cli/bundled-sqlcipher` exit 0 in 10m 39s
- 53 MB binary
- SQLCipher 4.5.7 community linked (verified via `scripts/verify-sqlcipher-link.sh`)

### Phase 2: Gateway boot from fresh encrypted DB — ✅ PASS
- Auto-created `~/.argo/sessions.db` (encrypted)
- schema_version=70 (latest migration)
- 4+ subsystem schedulers boot cleanly:
  - approval reminder (interval=300s, window=30m)
  - duty scheduler (interval=60s)
  - cu_screenshot sweep (interval=3600s)
  - audit chain watchdog
  - skill learning cron

### Phase 3: Health endpoint matrix — ✅ PASS
- `GET /health` → 200 `{"status":"ok","schema_version":70,"uptime_seconds":4}`
- `GET /health/live` → 200
- `GET /health/ready` → 200
- `GET /version` → 200 `{"version":"0.0.1","profile":"release"}`

### Phase 4: Product API surface — ⚠️ MOSTLY PASS (1 demo blocker fixed)
- ✅ `POST /api/v1/auth/signup` → 200 `{"status":"sent"}`
- ✅ Magic-link rate-limit: 5/hour/IP enforced (6th attempt → 429)
- ✅ `POST /api/v1/auth/verify` (invalid token) → 401 `{"error":"invalid"}`
- ✅ `GET /` (root landing) → 200 with full JSON API descriptor
- ✅ `GET /providers` → 200, all 9 provider manifests (anthropic, bedrock, cohere, groq, mistral, openai-compatible, openai-responses, perplexity, stub-echo)
- ❌→✅ `GET /verticals` → 200 with 29 loaded, 3 errored — **FIXED**: schema migrations in working tree → 32 loaded, 0 errored
- ❌→✅ `POST /api/v1/workers` (demo first-hire) → 400 "worker_draft: invalid shape" — **FIXED**: fixture schema migration → 201 `{"success":true,"worker_id":"..."}`
- ✅ `POST /api/v1/me/export` (no auth) → 401 "magic-link sign-in required" (correctly gated)
- ✅ `GET /marketplace` → 200 HTML

### Phase 5: Magic-link security properties — ✅ PASS
- ✅ Tokens are 32-byte base64url-encoded (e.g., `X5lRxOcSaXqIkn8qNoLsUJSHKL69bJFkZc3rFEvzL_I`)
- ✅ Token issued via `tracing::warn!` log (matches `noop` email provider per #1827)
- ✅ Invalid token → 401 `{"error":"invalid"}`
- ✅ Rate-limit confirmed: 5/hour/IP (Phase 4)

### Phase 6: Audit chain integrity (claim 5) — ✅ PASS (live HTTP level)
- ✅ Live session emit (`turn_start` + `halt`) creates 2 audit rows with linked hashes
- ✅ `GET /sessions/<id>/audit/verify` → 200 `{"breaks":0,"ok":true,"rows":2}`
- ✅ Every HTTP request also audited (`/audit/recent` returns http.request rows with parent_hash chains)
- ✅ tenant_id stamped on audit rows
- ⏳ `audit_chain_replay_smoke` cargo test still running

### Phase 7: Encryption-at-rest — ✅ PASS (the v1.0 SHIP-GATE)
- ✅ `sqlite3 ~/.argo/sessions.db "SELECT count(*) FROM sqlite_master;"` → error: "file is not a database (26)"
- ✅ First bytes of file: `0657 d35d f66a 7647` — random ciphertext (NOT plaintext SQLite magic `5351 4c69 7465`)
- ✅ Verified via `scripts/verify-sqlcipher-link.sh`: SQLCipher 4.5.7 community linked

### Phase 8: Multi-tenant isolation — ⏳ cargo test in progress
- Test: `crates/argo-cli/tests/cross_tenant_skill_discovery_negative.rs`

### Phase 9: Skill loop closure (claim 4) — ⏳ cargo test in progress
- Test: `crates/argo-cli/tests/skill_loop_closure_smoke.rs`

### Phase 10: Cargo test sweep — partially proven via Phases 6/8/9

### Phase 11: CLI subcommand surface — ✅ PASS
- 25+ top-level subcommands present
- `argo --version` shows release build with `bundled-sqlcipher,default` features
- `argo doctor` shows build info + workspace check (flagged `argo.toml MISSING` — acceptable in test env)
- All sampled subcommands (`migrate`, `skill`, `session`, `audit`) show clean help

### Phase 12: Migration encrypt CLI — ✅ PASS
- `argo migrate encrypt --help` shows usage
- Schema validation works (rejects missing `SKYCOT_DB_KEY_BASE64`)
- Required args: `--db <DB>`; optional `--key-source [env|keyring]`

### Phase 13: Performance bench — skipped (5-min budget out)

---

## Real bugs found

| # | Severity | Bug | Status | Fix |
|---|---|---|---|---|
| 1 | **P0 demo-blocker** | 3 verticals fail to load: stale `[[scope]]` schema instead of `[[entry]]` | Fixed in working tree | Migrated 3 files: `verticals/{marketing-intern,customer-support,sales-sdr}/memory/schema.toml` |
| 2 | **P0 demo-blocker** | `first-hire-payload.json` uses wrong schema (`workerDraft`/`name`/`vertical`) vs endpoint's expected `worker_draft`/`suggestedName`/`suggestedVertical` + missing required fields | Fixed in working tree | Rewrote `scripts/demo-dry-run-fixtures/first-hire-payload.json` |
| 3 | **P1** | Stub-echo provider rejects plain-string content: "parse content_json: invalid type: string, expected a sequence" | Open | Out of scope for test run; needs a brief or message-format compatibility fix |
| 4 | **P1** | `/api/v1/me` returns 404 (expected 401 since unauthenticated) | Open | Investigate; might be route missing or 404 is intentional |
| 5 | **P2** | `/audit/feed` returns 404 (expected — endpoint not in catalog; use `/audit/recent` instead) | n/a | Dry-run script referenced wrong path |
| 6 | **P2** | `argo doctor` flags `argo.toml MISSING` in fresh test env | n/a | Expected without workspace config; production deploys provide it |

---

## Live user-journey proof

End-to-end happy path exercised:

```
1. Boot gateway with fresh encrypted DB                 ✅ HTTP 200 /health
2. POST /api/v1/auth/signup                              ✅ HTTP 200 {"status":"sent"}
3. POST /api/v1/auth/signup × 5 more                     ✅ HTTP 429 on 6th (rate-limit)
4. POST /sessions (vertical=marketing-intern)            ✅ HTTP 201 {"vertical_id":"marketing-intern"}
5. POST /sessions/<id>/messages                          ✅ HTTP 201
6. POST /sessions/<id>/runs                              ⚠️ HTTP 500 stub-content shape
7. GET /sessions/<id>/audit (chain rows)                 ✅ HTTP 200 (2 rows, hashes linked)
8. GET /sessions/<id>/audit/verify (integrity)           ✅ HTTP 200 {"breaks":0,"ok":true}
9. POST /api/v1/workers (demo first-hire shape)          ✅ HTTP 201 {"worker_id":"019e3939-..."}
10. GET /audit/recent (workspace-wide chain)             ✅ HTTP 200 (all HTTP requests audited)
```

**9 of 10 live-flow steps PASS.** The 1 fail (step 6 stub-echo run-turn) is a stub-provider content-shape mismatch, not a v1.0 demo blocker (real demo uses Anthropic + computer-use, not stub-echo).

---

## What this proves about v1.0 readiness

- ✅ **Build works**: release binary compiles cleanly with `bundled-sqlcipher`
- ✅ **Encryption-at-rest works**: DB on disk is unreadable without key (sqlite3 confirms ciphertext)
- ✅ **Gateway is healthy**: schema v70 migrations work; all subsystems boot
- ✅ **Magic-link auth works**: signup + rate-limit + token format + 401-on-invalid
- ✅ **Audit chain works at HTTP level**: every request creates a chain row, verifies clean
- ✅ **All 32 verticals load** (after schema fixes)
- ✅ **Workers create successfully** (after fixture fix)
- ✅ **Multi-provider manifests load** (9 providers, tier metadata visible)
- ⏳ Cargo integration tests still validating in parallel

---

## Operator action items before recording

1. **Decide on the 2 in-working-tree fixes**: commit them OR author proper briefs to fix them
   - `verticals/{marketing-intern,customer-support,sales-sdr}/memory/schema.toml` — 3 files migrated `[[scope]]`→`[[entry]]`
   - `scripts/demo-dry-run-fixtures/first-hire-payload.json` — schema match endpoint
2. **Set `AUTH_SECRET` env var** before SPA dev start (or NextAuth.js 500s on every auth route)
3. **Resolve marketing dev Turbopack + MDX loader incompatibility** (try `pnpm dev --no-turbopack`)
4. **Re-run `bash scripts/demo-dry-run.sh` after the above** — should hit 12/12 pass

---

## Footprint of this test run

- ~30 min wall clock (mostly the release build + cargo test compile)
- Fresh encrypted DB created at `~/.argo/sessions.db`
- Gateway process running on :7400 (PID will need teardown)
- 2 schema migration files + 1 fixture file modified in working tree (uncommitted)
- Earlier gateway start backed up `~/.argo` to `~/.argo.before-sqlcipher-test-run-<timestamp>`
