# docs-demo-runbook-add-skycot-db-key-section

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `5a626ef98` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh docs-demo-runbook-add-skycot-db-key-section`.

🔴 **P0 — fresh-machine demo recording blocks at gateway startup; runbook does not mention the fix.**

## Why this is P0

Round 30 of the evaluator tried to bring up `argo gateway start` on the demo path. The gateway refused to bind with:

```
argo gateway: store open failed (provider error: gateway store: derive SQLCipher key
for workspace-default tenant: provider error: keyring read
tenant-00000000-0000-0000-0000-000000000000: Platform secure storage failure:
User canceled the operation.. Set SKYCOT_DB_KEY_BASE64 to a base64-encoded 32-byte
secret, or run with an OS keyring available.); /sessions etc. will return 503
```

The error message correctly points at the fix: **`SKYCOT_DB_KEY_BASE64` env var**.

But `docs/skycot/runbook/v1-demo-recording.md` §5 ("Environment variables") does not mention `SKYCOT_DB_KEY_BASE64`. On a fresh recording machine the operator will hit this error and not know what to do, because:

1. The runbook's env-var block lists 6 expected vars (`ANTHROPIC_API_KEY`, `SLACK_*`, `ARGO_BROWSER_MCP_BIN`, `ARGO_GATEWAY_AUTH_BYPASS`) — `SKYCOT_DB_KEY_BASE64` is the 7th and it's missing.
2. The macOS Keychain prompt can be hidden behind the operator's recording-screen overlay or accidentally dismissed during pre-flight.
3. Operator memory says "fresh recording machine" — meaning the keyring won't have a cached entry yet, so the prompt fires the first time.

Result: a 90-second fix becomes a 30-minute scramble on recording day.

## Scope

Add a §5.1 to `docs/skycot/runbook/v1-demo-recording.md` between the current §5 (Environment variables) and §6 (Recording session). Title: "**5.1. SQLCipher key (one extra env var)**". Plain language; copy-pasteable.

## Exact change

`docs/skycot/runbook/v1-demo-recording.md` — between line 173 (end of §5 sanity-check) and line 175 (start of §6):

```markdown
### 5.1 SQLCipher key (one extra env var)

The gateway encrypts its local SQLite database with SQLCipher
(Structured Query Language Cipher — an open-source extension to
SQLite that adds 256-bit AES encryption to every database file).
The encryption key is derived from one of two sources, in this
order of preference:

1. **`SKYCOT_DB_KEY_BASE64`** — a 32-byte secret you provide,
   encoded as base64 (a way of representing binary bytes as
   ASCII text so they can fit in an env var).
2. **macOS Keychain** (or Linux `keyring`, Windows
   `Credential Manager`) — the gateway asks the operating
   system's password vault on startup. The OS may pop a prompt
   asking "Allow access to keyring item?" — if you click No (or
   the prompt fires behind your recording overlay and you don't
   see it), the gateway refuses to start with a 503 on every
   request.

**For recording: always set `SKYCOT_DB_KEY_BASE64` explicitly.** The
keyring path is fine for daily development but the prompt is
fragile to dismiss accidentally during a screen recording. Use:

```bash
# Generate a fresh 32-byte secret + base64-encode it. Stable
# across this recording session only — drop it after recording.
export SKYCOT_DB_KEY_BASE64="$(openssl rand -base64 32)"
```

Add this to the env-var block in §5. Your sanity check from §5
becomes:

```bash
env | grep -E '^(ANTHROPIC_API_KEY|SLACK_|ARGO_BROWSER_MCP_BIN|ARGO_GATEWAY_AUTH_BYPASS|SKYCOT_DB_KEY_BASE64)=' | wc -l
# expect: 7
```

If the gateway logs `"store open failed ... derive SQLCipher key
for workspace-default tenant: ... User canceled the operation."`,
you forgot this var. Re-export it and restart `argo gateway start`.
```

Update §5 (line 171) sanity-check from `# expect: 6` to `# expect: 7` and add `|SKYCOT_DB_KEY_BASE64` to the grep alternation.

## Files you may touch

- `docs/skycot/runbook/v1-demo-recording.md` — the one required edit.

## Files you must NOT touch

- Any production code. This is a docs-only change.

## Done means

- The new §5.1 lands between §5 and §6.
- The §5 sanity-check `# expect:` count is bumped to 7 and the grep alternation matches.
- A fresh operator following the runbook end-to-end on a clean machine successfully starts the gateway without dismissing or missing a keyring prompt.

## Why this matters NOW

The v1.0 demo recording is gated on this docs fix + the companion `demo-ops-add-magic-link-bypass-for-recording` brief. Without both, the operator either fights with environment ergonomics on recording day or postpones the demo. P0.

## Authoring trail

Companion to `demo-ops-add-magic-link-bypass-for-recording` (the magic-link auth bypass brief). Both were authored by the evaluator after round 30's live Chrome MCP walkthrough surfaced both barriers. Findings + screenshots: `docs/skycot/evals/scheduled/2026-05-23/round-30-demo-e2e-chrome/findings.md`.
