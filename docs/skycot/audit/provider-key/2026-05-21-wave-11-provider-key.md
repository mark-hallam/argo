# Wave-11 audit #3 — provider-key entry flow

**Date:** 2026-05-21
**Auditor:** Wave-11 agent #3 (read-only static analysis)
**Scope:** `/onboarding/connect-provider` step + the downstream gateway endpoint it depends on
**Inputs:**
- Page: `web/app/(setup)/onboarding/connect-provider/page.tsx`
- Mutation client: `web/lib/onboarding/use-provider-key.ts`
- Draft store: `web/lib/onboarding/use-onboarding-draft.ts`
- Wave-3 audit: `docs/skycot/audit/onboarding/2026-05-20-wave-3-onboarding-e2e.md`
- Wave-3 brief: `docs/build-plan/streams/web-gateway-endpoints-onboarding-batch.md` (authored 2026-05-21, **not shipped**)

---

## TL;DR — security verdict, UX score, ship-blocker

- **Security verdict:** FAILS — the key is sent over HTTP to a `POST /api/v1/onboarding/provider-key` endpoint that **does not exist on the gateway**. Today the page silently looks broken: `apiPost` throws an `ApiError(404)` → error pill renders "404 Not Found" → no provider key is ever stored. There is no encrypted-at-rest path because there is no backend path. The user's only working option is "Skip for now" → mock provider, which means **the demo path never exercises real provider-key storage at all**.
- **UX score:** 4/10. The single-provider hardcode, no masked echo, no real test-call confirmation, and bare-text validation ("sk-ant-") mean the form is shaped like a TODO. Skip is the actual happy path.
- **Gateway endpoint exists?** No. Only `POST /api/v1/onboarding/provision` is implemented in `crates/argo-cli/src/handlers/gateway/onboarding_provision.rs`. The wave-3 brief `web-gateway-endpoints-onboarding-batch` covers it but is unshipped as of `origin/main 50e65e9f` (2026-05-21).

---

## Dimension-by-dimension findings

### 1. Provider list

**Result:** FAIL — hardcoded to Anthropic only.

- `page.tsx:11` declares `ANTHROPIC_CONSOLE_URL` as the only console link.
- `page.tsx:31` enforces `key.startsWith("sk-ant-")` — no branch for `sk-` (OpenAI), `AIza` (Google), `sk-proj-` (OpenAI project), or any custom-endpoint provider.
- `page.tsx:60-62` hero copy reads "Connect **Anthropic.**" — no provider picker, no logo grid, no toggle.
- **No svgl.app logos** anywhere. The page renders zero provider iconography — just the lucide `ExternalLink` icon on the "Get a key from the Anthropic console" link.
- Provider routing in core supports manifests for multiple providers (see `argo-provider-manifest`), but the onboarding wizard exposes none of that.

**Operator directive violation:** the brief instructs providers be sourced from svgl.app. The page has no logo loader, no provider list, no `<select>`.

### 2. Key validation

**Result:** PARTIAL — prefix check only; no test-call sanity even in the optimistic path.

- `page.tsx:31-34` is the entire client-side validation: `if (!key.startsWith("sk-ant-")) → "Anthropic API keys start with sk-ant-."`. No length check (Anthropic keys are ~108 chars), no character-set check, no whitespace stripping beyond `trim()`, no console-copy-paste hardening (newlines from copy can carry through `password` inputs).
- The "test-call" is server-side: `use-provider-key.ts:35-41` calls `apiPost("/api/v1/onboarding/provider-key", ...)` and unpacks `{ validated, test_cost, test_tokens }`. The brief at `web-gateway-endpoints-onboarding-batch.md:24` states this endpoint must "store encrypted key … return `{ stored: true, masked_key }`" — note the brief's response shape is `stored: bool` + `masked_key`, but the client expects `validated: bool` + `test_cost/test_tokens`. **The contract is already drifted between client and brief.** If a worker ships the brief literally, the client will throw `ApiError(502, "provider-key endpoint returned an unexpected shape")` because the response will not satisfy `isValidateResponse`.
- The hero subcopy at `page.tsx:65` promises "A **$0.001 test call** confirms the key works" — the brief makes no mention of an actual upstream Anthropic API call. The brief's storage table is `workspace_provider_keys`; there is no mention of an Anthropic `POST /v1/messages` round-trip. **The "$0.001 test call" copy is currently aspirational, not contractual.**

### 3. Storage — does the gateway endpoint ship encrypted storage?

**Result:** FAIL — endpoint unshipped. Brief exists but the Rust handler is missing.

- `rg -rn "onboarding/provider-key|/provider-key" crates/` returns zero hits in handler code.
- Confirmation: `grep -n "onboarding" routes_dispatch_post.rs` shows only `/api/v1/onboarding/provision` is dispatched (lines 206-209). The `/provider-key`, `/install-vertical`, `/first-hire` paths fall through to the default 404.
- The wave-3 brief `web-gateway-endpoints-onboarding-batch.md` (authored 2026-05-21) covers it: ~300-500 LOC, encrypted-at-rest via SQLCipher-if-available-else-AES-GCM, `workspace_provider_keys` table. The brief is in `streams/` but no PR has landed.
- **Implication for the demo path:** running `npm run dev` + a live gateway today and submitting a real `sk-ant-...` key produces a 404 error to the user, surfacing the wrong message: "404 Not Found". The optimistic path doesn't work. Only Skip works.

### 4. Masked echo (`sk-...abc1` after entry)

**Result:** FAIL — no masked echo, no key fingerprint, no confirmation surface anywhere.

- `page.tsx:39` clears the input on success (`setApiKey("")`) and immediately routes away.
- The `useOnboardingDraft` store stores only `providerKeyConfigured: boolean` and `providerSkipped: boolean` (`use-onboarding-draft.ts:12-13, 40-41`). The key fingerprint / last-4 / `masked_key` field is never stashed and never displayed.
- `ProviderKeyResult` (`use-provider-key.ts:10-14`) exposes `{ validated, testCost, testTokens }` — no `maskedKey` field. The brief at `web-gateway-endpoints-onboarding-batch.md:24` says the endpoint returns `{ stored: true, masked_key }`, but the client throws away the second field even if the server provides it.
- A returning user on `/settings/providers` (if such a page existed yet — wave-11 scope doesn't confirm) would have no way to confirm "yes, that key is the one I added on Tuesday" because the last-4 / fingerprint is never plumbed.

### 5. Test connection button — fires a real test call?

**Result:** FAIL — there is no Test Connection button. The single "Validate & continue" button conflates test-call and forward navigation.

- `page.tsx:36-43` is one atomic flow: `validate.mutateAsync` → `setProvider(true, false)` → `router.push(...)`. There is no separate "Test" affordance that the user can click before committing.
- No retry button on failure either — the user has to manually re-type or paste again. The error pill at `page.tsx:100-108` only shows the failure; the form remains submittable but the input is the same string.
- Because the gateway endpoint is unshipped (dimension 3), today the "Validate & continue" button does not in fact fire any test call — it 404s. So the user has zero feedback that the system "works" until Skip is used and they reach the next step on the mock provider.

### 6. Skip option

**Result:** PASS (works as designed) — but the absence of a real path makes Skip the only happy path.

- `page.tsx:46-49`: clicking Skip calls `setProvider(false, true)` (configured=false, skipped=true) and routes to `/onboarding/pick-vertical`.
- `page.tsx:147-153` renders an amber callout: "Skipping uses a mock provider. Workers will run, but responses are simulated until you add a real key in Settings." — clear, well-copyedited.
- Returning to add a key later: the brief promises `/settings/providers`, but no audit on wave-11 has confirmed that surface exists. Wave-3 already noted onboarding has no "I'll add this later" deep-link from this very page (`audit/onboarding/2026-05-20-wave-3-onboarding-e2e.md:114`).

---

## Top 3 issues (prioritised by impact)

1. **The gateway endpoint does not exist.** Real `sk-ant-...` keys 404 today. The "happy path" of the wizard is broken; Skip is the only forward motion. Until the wave-3 brief `web-gateway-endpoints-onboarding-batch` ships, the page is a fake form. Worse, the **brief's response shape (`{ stored, masked_key }`) does not match the client's expected shape (`{ validated, test_cost, test_tokens }`)** — the brief needs to be re-aligned with the client contract before a worker claims it, or the client needs adjusting in the same PR.

2. **Hardcoded to Anthropic; no multi-provider surface.** No svgl.app logos (operator directive violated), no OpenAI / Google / Mistral / Azure option, no custom-endpoint affordance. The hero literally reads "Connect Anthropic." — meaning a customer who wants OpenAI cannot complete onboarding non-mock today. This blocks enterprise customers with provider-mandated vendor lists.

3. **No masked echo, no Test Connection button, no key fingerprint stored.** The wizard validates and immediately forgets. A user has zero confirmation surface — no `sk-...abc1` chip in the draft, no "Test again" button, no way to see in Settings later "you added a key ending …abc1 on 2026-05-21". This is both a UX gap and a security gap: ops-on-call has nothing to grep audit logs for. The brief mandates `masked_key` in the response; the client must surface it.

### Minor / honourable mentions

- `page.tsx:79` input uses `type="password"` — good (browser won't autofill, won't render keys in screenshots). But no `<button>` to toggle visibility, so a user who paste-fumbles can't easily verify what they pasted.
- `apiPost` adds `Authorization: Bearer <token>` (`client.ts:71`). The brief assumes the same. Without backend, can't confirm cross-tenant guard is enforced — but the brief's acceptance criteria explicitly requires 403 on cross-tenant, so any implementation must wire that.
- No rate-limit on submit. If a user pastes a malformed key 10 times in a row, the client doesn't debounce. Brief should specify rate-limit middleware (the gateway already has `rate_limit_tenant`).
- No CSP / TLS verification surface in the page copy — operators presented with a "give us your $0.05/req API key" prompt see no trust signal (no SOC 2 / GDPR / "encrypted at rest" badge). Compare to the `/hire` shell which surfaces SOC 2 + GDPR.
- Skip routes to `/onboarding/pick-vertical` without recording any audit row. The brief doesn't mention auditing the skip decision, but it should — "operator chose mock provider" is a load-bearing capability signal.

---

## Recommended brief follow-ups

1. **Re-align `web-gateway-endpoints-onboarding-batch`** so the `/provider-key` response shape is `{ validated, test_cost, test_tokens, masked_key }` (union of brief intent + client contract). Document the real Anthropic test-call ($0.001 = small `POST /v1/messages` with `max_tokens: 1`).
2. **Author a follow-up brief** `web-onboarding-multi-provider-picker` to widen from Anthropic-only to a provider grid (svgl.app logos, prefix-aware validation per provider, default-routing per tenant policy).
3. **Author a follow-up brief** `web-settings-provider-keys-page` so the masked echo + rotate/revoke surface exists. The Skip flow needs somewhere to land.
4. **Add a Test Connection affordance** separate from "Validate & continue" so the user can verify before commit. This is a P2 polish item, parented under the same brief as #1.

---

## One-line recommendation

The page is a wireframe today. The gateway endpoint is missing, the contract is drifted between client and brief, only Anthropic is supported, no masked echo, no test button, no logos — fix dimensions 1, 3, 4, 5 in a single brief before this surface can ship to a real customer.
