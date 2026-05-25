# fix-onboarding-provider-key-500-friendly-error

**Owner**: codex.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-provider-key-500-friendly-error`.
**Source**: e2e-ux-audit (onboarding-hire-an-ai-teammate) finding F5 — `docs/skycot/ux-audits/2026-05-26-0820/onboarding-hire-an-ai-teammate.md`.

🔴 **P0 — Provider-key validation returns HTTP 500 with a raw infrastructure error in the form when the workspace root key env-var is unset. Investor demo would record "wrap key: schema validation failed: no workspace root key: set SKYCOT_WORKSPACE_ROOT_KEY_BASE64..." on screen.**

## Root cause

`POST /api/v1/onboarding/provider-key` with any plausibly-shaped key (e.g. `sk-ant-blah-1234567890123`) returns:

```
HTTP 500
{"error":{"code":"internal_error","message":"wrap key: schema validation failed: no workspace root key: set SKYCOT_WORKSPACE_ROOT_KEY_BASE64 (base64 32 bytes) or configure ARGO_CREDENTIALS_MASTER_KEY for back-compat derivation"}}
```

The shape pre-check (≥16 chars) returns 400 cleanly — only the post-shape encryption path 500s when `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` is unset and the wrap step fails.

The wizard at `web/app/(setup)/onboarding/connect-provider/page.tsx:41` catches the error and surfaces `e.message` as-is — so the raw infrastructure message renders in the form. On recording day, a screenshot of this would be devastating: it leaks the env-var name, implies the workspace is misconfigured, and looks like the product is broken.

Two failure modes for the same defect:
- (a) Some workspaces ship without the root key configured (dev installs, demo workspaces, self-host evaluators trying the product without reading the docs)
- (b) Any future regression that makes the wrap step fail will surface the same way

Fix is two-layer: server returns a structured envelope; client renders a friendly message regardless of what the server says.

## Curl evidence (from the audit)

```
curl -X POST http://127.0.0.1:7400/api/v1/onboarding/provider-key \
  -H 'X-Argo-Principal: human:smoke@skycot.com' \
  -d '{"api_key":"sk-ant-totally-fake-key-xxxxxx"}'
→ 500 with the raw "wrap key: schema validation failed..." string
```

## Findings this brief closes

- onboarding-hire-an-ai-teammate F5 [P0] — Provider-key validation 500s with a raw internal error when workspace root key is unset

## Fix recipe

**Server-side** (`crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` — the `provider-key` handler):

1. Detect the specific `wrap key: no workspace root key` failure path. Catch the error variant (or downcast / match on substring) BEFORE returning the generic 500.
2. Return HTTP **503 Service Unavailable** with body:
   ```json
   {
     "error": {
       "code": "provider_validation_unconfigured",
       "message": "Provider validation isn't configured in this workspace. You can skip this step — your workers will run with the built-in mock provider until you add a real key later in Settings."
     }
   }
   ```
3. Keep the existing 400 for shape failures unchanged.
4. Generic 500 stays for genuinely unexpected errors (network failure to Anthropic API verification, etc.) — but never leak env-var names in the message body.

**Client-side** (`web/app/(setup)/onboarding/connect-provider/page.tsx`):

1. The `catch` block at line 41 must NOT surface `e.message` raw. Instead, switch on `e.code`:
   - `provider_validation_unconfigured` → render a soft notice + auto-enable the "Skip for now" path: "We couldn't validate your key right now. You can continue — your workers will use the mock provider until validation is available."
   - `invalid_key_shape` → "Your key doesn't look right. Anthropic keys start with `sk-ant-`."
   - `anthropic_rejected_key` → "Anthropic rejected that key. Double-check you copied the whole value."
   - default → "Something went wrong. You can skip this step and add a key later." (NEVER show the raw error message)

2. Log the raw error to console (gated on `NODE_ENV !== "production"`) so developers can still debug.

## Acceptance criteria

- [ ] `curl -X POST /api/v1/onboarding/provider-key -d '{"api_key":"sk-ant-fakekey-xxxxxxxxxxxxxx"}'` against a workspace with `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` unset returns HTTP 503 (not 500) with code `provider_validation_unconfigured` and a friendly message.
- [ ] The error response body NEVER contains the string "SKYCOT_WORKSPACE_ROOT_KEY_BASE64" or "ARGO_CREDENTIALS_MASTER_KEY" or "wrap key: schema validation".
- [ ] Wizard `/onboarding/connect-provider` page renders a friendly notice in the failure path (no raw error string).
- [ ] Wizard auto-enables the "Skip for now" affordance when the server returns `provider_validation_unconfigured`.
- [ ] Demo-day curl smoke: visit `/onboarding/connect-provider` in a workspace without the root key, type a fake key, submit — see friendly message + skip option, no infrastructure leak.
- [ ] Integration test added at `crates/argo-cli/tests/onboarding_provider_key_no_root_key.rs`: with root-key env var unset, POST a valid-shape key → assert 503 + the friendly code.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` (provider-key handler)
- `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` (error type mapping if needed)
- `web/app/(setup)/onboarding/connect-provider/page.tsx` (catch + UI)
- `web/lib/api/onboarding.ts` (error code constants if not present)
- `crates/argo-cli/tests/onboarding_provider_key_no_root_key.rs` (NEW)

## Out of scope

- Setting the workspace root key in dev env (operator/runbook concern, not code)
- Multi-provider support (separate brief: `fix-onboarding-connect-provider-multi-provider-copy-or-options` already exists)
- Mock-provider persistent banner (separate brief: `fix-mock-provider-persistent-banner-in-app-shell` already exists)

## Effort

~50-100 LOC across handler + UI + test. ~1-2 hours.
