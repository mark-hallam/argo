# ws6-fix-stale-waitlist-frontend-backend-pending-comments

**Owner**: claude.
**Authored against**: origin/main `eddcab780` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ws6-fix-stale-waitlist-frontend-backend-pending-comments`.

## Why (NO-THEATER inversion on the PUBLIC funnel, VERIFIED 2026-06-09)
The waitlist BACKEND shipped (#2675) but the FRONT-END still claims it does not exist — so a real signup succeeds while the UI can show a false "not saved" panic. This is a NO-THEATER (DEC-007) inversion: instead of faking success, the surface now fakes FAILURE on a real conversion. It is the public funnel's first conversion, so the lie is maximally costly.

VERIFIED live (read 2026-06-09):
- **Backend EXISTS + works.** `crates/argo-cli/src/handlers/gateway/routes_waitlist.rs:48` (`is_waitlist_request`) + `:55` (`waitlist_response`): `POST /api/v1/waitlist` validates the email, runs the shared per-IP signup limiter + honeypot, inserts into `waitlist_signups` (additive migration v88) with `ON CONFLICT(email) DO NOTHING`, and returns an idempotent `202 {"queued":true}` (`:94`, `:124`). Mounted un-gated (public/anonymous-safe) ahead of the bearer-auth gate, same lane as magic-link signup.
- **Front-end says it does NOT exist.** `web/lib/api/waitlist.ts:4-12` header: "the gateway route this posts to, `POST /api/v1/waitlist`, DOES NOT EXIST YET … every call here will resolve to a 404". `:46-53` `joinWaitlist` doc repeats "the 404 that the not-yet-built backend currently returns".
- **Front-end shows a false "not saved" warning.** `web/components/marketing/WaitlistForm.tsx:11-18` header ("⚠️ DRAFT — BACKEND PENDING … will hit a 404 until the backend lands"); `:64-72` treats a 404 OR any `>= 500`/network error as "backendPending" and renders `:71` "Waitlist capture isn't connected yet — this is a draft surface. **Your email was NOT saved.**". A real `202` already routes to the correct `queued` success state (`:85-97`), so the only fix needed is removing the stale "doesn't exist / not saved" framing — the happy path is already correct.

## Grep-before-write
- `rg -n 'DOES NOT EXIST|NOT saved|not-yet-built|BACKEND PENDING|backendPending|isn.t connected' web/lib/api/waitlist.ts web/components/marketing/WaitlistForm.tsx` — every stale string to strip.
- `rg -n 'joinWaitlist|WaitlistForm|isValidEmail' web/` — confirm the only consumers (the landing-page hero + final-CTA captures); no other call sites to update.
- `rg 'pub (struct|enum|fn) waitlist_response' crates/` and `rg 'pub (struct|enum|fn) is_waitlist_request' crates/` — confirm the canonical backend handlers already exist (do NOT add a parallel route; this brief is FRONT-END copy + a vitest only).
- `rg -n "POST /api/v1/waitlist" crates/argo-cli/src/handlers/gateway/routes.rs` — confirm the public mount so the route is reachable un-authenticated.

## Fix
1. `web/lib/api/waitlist.ts`: rewrite the file header (`:1-24`) and the `joinWaitlist` doc (`:46-53`) to state the route is LIVE (`POST /api/v1/waitlist` → idempotent `202 {"queued":true}`, public/anonymous, deduped on email). Remove every "DOES NOT EXIST YET / not-yet-built / the 404 that the backend currently returns / backend brief to author" line. The runtime logic is already correct (it posts the trimmed `email` + `source` and treats a `202` as `queued`) — do NOT change the wire shape or behaviour. Keep the existing `isValidEmail` pre-check.
2. `web/components/marketing/WaitlistForm.tsx`: rewrite the component header (`:11-22`) to drop "DRAFT — BACKEND PENDING / will hit a 404". Remove the `backendPending` state + its branch (`:40`, `:55`, `:64-72`, and the `wl-msg-pending` class at `:141`): a real 404 from the live route now means a genuine routing/deploy fault, not an expected draft state, so it should fall through to the generic honest error path (`:74-78`) — never the "Your email was NOT saved" copy. Leave the `queued` success card (`:85-97`) and the email-validation error path unchanged. Consumers: the landing-page hero + final-CTA `WaitlistForm` instances (the `source` prop) — no other component changes.
3. Additive only: no new pub types, no new route, no migration. This is stale-comment + dead-branch cleanup on two front-end files plus the test below (subtract before adding).

## Acceptance (truth-bounded)
1. Extend `web/lib/api/waitlist.test.ts` (vitest, already mocks `./client`): add a case asserting `joinWaitlist({email,source})` posts to `/api/v1/waitlist` and resolves `{queued:true}` on a mocked `202` (empty-body and `{queued:true}` shapes) — the live success path, not a thrown error.
2. Add/extend a `WaitlistForm` render test (vitest + `@testing-library/react`, matching the existing web test setup) asserting: on a resolved `joinWaitlist`, the form renders the success card (`data-testid="waitlist-success"` / "You're on the list") and NEVER renders the "Your email was NOT saved" / "isn't connected yet" copy.
3. Checkable string verifier — both commands return ZERO matches after the fix:
   - `rg -n 'DOES NOT EXIST|NOT saved|not-yet-built|BACKEND PENDING' web/lib/api/waitlist.ts web/components/marketing/WaitlistForm.tsx`
   - `rg -n 'backendPending|wl-msg-pending' web/components/marketing/WaitlistForm.tsx`
4. `cd web && npx vitest run lib/api/waitlist.test.ts components/marketing` green; `npm run lint` clean on the two touched files. Total diff well under 500 LOC (two-file copy edit + one test).

## On ship (same commit)
- Flip the WS6 waitlist row in `docs/build-plan/FEATURE_MATRIX.md` to reflect the front-end now matching the live `POST /api/v1/waitlist` backend (status badge + this commit SHA + the two web files + the route). Do NOT regenerate `docs/skycot/feature-matrix.html` (GITIGNORED per DEC-043 — `.md` row only).
- Note in the commit body that this closes the front-end half of #2675 (backend) — the public funnel's first conversion no longer reports a false "not saved".

## Effort
S (two front-end files: header/comment rewrite + remove one dead state branch, plus one vitest extension and one form render test; no backend, no migration).
