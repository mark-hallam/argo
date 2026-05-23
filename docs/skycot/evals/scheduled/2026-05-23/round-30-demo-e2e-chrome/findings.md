# Round 30 — v1.0 demo e2e via Chrome browser

**Date**: 2026-05-23
**Tool**: Chrome MCP (`claude-in-chrome`) at `http://localhost:3000` (Next.js dev) + `http://127.0.0.1:7400` (gateway)
**Script driven**: [`docs/skycot/demo_script.md`](../../../demo_script.md) — the 5-7 minute v1.0 mission demo, 6 scenes.
**Operator directive 2026-05-23 21:30Z**: "can you run the demo e2e in the browser chrome that is"

## TL;DR — verdict

🟡 **Demo runs cleanly through Scene 2 step 2 (workspace name), then hits the auth gate.** The system correctly refuses to call `/api/v1/onboarding/provision` without a verified `X-Argo-Principal: human:<email>` header — i.e., **the auth gating is working as designed**. To complete Scenes 3/4/5 the demo needs a real magic-link signup, which requires either (a) a working SMTP / mail server hooked up, or (b) a flag to mint a verified principal locally for demo runs.

## What ran cleanly (Scenes 0 + 1 + Scene 2 step 1+2)

| Scene | Step | Verdict | Notes |
|---|---|---|---|
| 0 | Cold-open / marketing landing | 🟢 | Audiowide hero "Don't buy another tool. Hire a teammate." + "V1.0 SHIPPED · HIRING OUR FIRST 100 TEAMS" badge. CTA "Hire your first teammate" routes correctly. |
| 1 | `/auth/signup` page | 🟢 | F1 editorial-center layout. SSO buttons (Google / GitHub / Microsoft) + magic-link email field with "Send magic link" CTA. Demo email `demo@throwaway.cc` accepts input. |
| 2.1 | `/onboarding/welcome` | 🟢 | "STEP 01 OF 05" indicator + Audiowide "Build your AI workforce." + AUDIT-FIRST / SINGLE-BINARY / ENTERPRISE-GRADE trust strip. "Get started" advances. |
| 2.2 | `/onboarding/workspace` | 🟢 (UI) | "STEP 02 OF 05" + workspace + company name fields. Input round-trips. Continue button submits to gateway. **Auth gate blocks here** — see below. |

## Where the e2e demo blocks

### 🚨 Finding 1 (P1 for demo): Auth gate on `/api/v1/onboarding/provision`

**Symptom**: clicking "Continue" on Scene 2 step 2 surfaces "X-Argo-Principal header must carry a verified `human:<email>` value" inline below the form.

**Network trace**:
```
POST http://127.0.0.1:7400/api/v1/onboarding/provision → 401
```

**Root cause**: gateway demands the SPA's Next.js BFF stamp `X-Argo-Principal: human:<email>` on every onboarding call. The BFF only has that value after a successful magic-link verify, which requires a real email-delivery loop.

**Why this is correct security behavior**: an unauthenticated request that could provision a tenant would be a tenant-creation DoS vector. The 401 IS the safety net working.

**Why this is a demo barrier**: the demo script's Scene 1 ("Cut to a real inbox. The magic-link email arrives. Click the link.") assumes a real SMTP/mail server is wired up. Without it, the demo can't complete e2e in a single take.

**Recommended brief for conductor**: `demo-ops-add-magic-link-bypass-for-recording` (P2, 50-100 LOC) — add an env-var-gated dev-mode signup path (NOT a production code path) that takes a `--demo-principal demo@throwaway.cc` flag and mints a verified principal session WITHOUT real email round-trip. Gated by `ARGO_DEMO_MODE=1` + non-zero `RUST_LOG=warn` so it never lands in a production gateway. Pairs with the existing `scripts/demo-dry-run.sh` validator.

### 🚨 Finding 2 (P0 for fresh-machine demo): Gateway startup needs explicit DB key

**Symptom**: `argo gateway start` fails on fresh macOS with:
```
argo gateway: store open failed (provider error: gateway store: derive SQLCipher key for workspace-default tenant: provider error: keyring read tenant-00000000-0000-0000-0000-000000000000: Platform secure storage failure: User canceled the operation.. Set SKYCOT_DB_KEY_BASE64 to a base64-encoded 32-byte secret, or run with an OS keyring available.)
```

**Root cause**: gateway uses macOS Keychain to derive the SQLCipher key at startup. On a fresh machine — or when the operator dismisses the keychain prompt — the gateway refuses to bind. The error message correctly points at the fix.

**Why this matters for demo recording**: the demo runbook ([`docs/skycot/runbook/v1-demo-recording.md`](../../../runbook/v1-demo-recording.md)) doesn't mention `SKYCOT_DB_KEY_BASE64` in its env-var setup section. On a fresh recording machine, this would block the operator at "start the gateway."

**Workaround that lands the gateway**:
```bash
SKYCOT_DB_KEY_BASE64="$(openssl rand -base64 32)" \
  ANTHROPIC_API_KEY=sk-ant-placeholder \
  ARGO_GATEWAY_AUTH_BYPASS=1 \
  argo gateway start --bind 127.0.0.1 --port 7400
```

**Recommended brief**: `docs-demo-runbook-add-skycot-db-key-section` (P2, 20 LOC) — add a §5.1 to the runbook explaining the SQLCipher key derivation path with both keyring + `SKYCOT_DB_KEY_BASE64` options.

### 🟢 Finding 3 (resolved by 2nd attempt): Gateway provider-key startup check is strict

The gateway refuses to start with "no provider API key set (need ANTHROPIC_API_KEY or OPENAI_API_KEY)" — even with `ARGO_GATEWAY_AUTH_BYPASS=1`. The fix is to set `ANTHROPIC_API_KEY=sk-ant-placeholder` (the check only verifies presence, not validity). This is correctly documented in §5 of the demo runbook.

Not a bug, but worth a one-line callout in the demo dry-run validator script.

## Scenes 3 + 4 + 5 (gated behind auth)

These scenes need a real signed-in session. Documented here so the conductor can see what the demo COULD show end-to-end once the auth-bypass demo flag exists:

- **Scene 3** (brief + auto-approval ASK) — would land on `/workers/maya` with chat composer, type a launch-blog brief, watch Maya turn through research → outline → draft → propose `publish_post`, see the activity feed show ApprovalAsked.
- **Scene 4** (Slack approval round-trip) — cut to Slack, click ✅, cut back to SPA, see `ApprovalReplied → Executing → Completed`.
- **Scene 5** (audit chain + close) — `/audit` page filtered to Maya's worker id, hover row group to show hash-chain tooltip.

All three scenes have green regression tests on `main` covering their substrate (rounds 22+23+24+27 for Slack approval round-trip + audit-row shape; round 21 for audit-chain integrity; round 28 for trust grade). The browser demo would showcase what the regression tests pin programmatically.

## Cross-reference with mission claim coverage

This round didn't author new regression tests — it ran the existing demo script against the live SPA + gateway. The findings reinforce coverage gaps that can be closed without writing new e2e tests:

- The auth-bypass demo flag (finding 1) is purely a developer-experience improvement.
- The DB key documentation (finding 2) is a one-page docs fix.
- The auth gating itself (the 401) is working — that's the desired behavior, no fix needed.

## Recommendations to operator

The demo IS recordable today, but the operator needs the following pre-flight steps that aren't in the current runbook:

1. **Set `SKYCOT_DB_KEY_BASE64`** before starting the gateway (or accept the macOS keychain prompt — but that's a recording interruption).
2. **Real SMTP-backed email** for the magic-link cut (or wait for `ARGO_DEMO_MODE=1` flag).
3. **Throwaway Anthropic key + Slack workspace** as documented.

For the **next demo-recording attempt**, give the operator either:
- A shell script that bootstraps gateway + workspace + injects a verified principal session (probably 80 LOC, easy to ship as a worker brief).
- OR finish the magic-link email loop with a real SMTP relay (more work, but matches the script verbatim).

The 8 mission-claim regression tests landed this week (rounds 17 + 21 + 22 + 23 + 24 + 26 + 27 + 28 + 29 = 10 total) prove the BACKEND is ready for the demo. The remaining gap is the **operator-recording ergonomics** — bringing the system from cold to recordable in <5 minutes without macOS keychain dance or SMTP setup.

## Screenshots captured

- 5 screenshots in `screenshots/` showing scenes 0 → 2.2 (cold-open, signup form, signup with email, onboarding welcome, workspace step).
- Each captured at 1440×723 viewport (matching the demo script's "stay above 1280px wide" guidance).
