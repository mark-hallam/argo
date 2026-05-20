# Wave-12 Audit #5 — Worker Name Collision + Duplicate-Prevention

**Date:** 2026-05-21
**Auditor:** Wave-12 audit agent #5 (claude opus 4.7, 1M)
**Scope:** What happens when the user hires two workers named "Mariah"? Two
identical descriptions? Worker ID guarantees? In-flow visible naming guidance?
**Branch / SHA:** `origin/main` at audit time
**Verdict:** **Duplicate-prevention is effectively absent for worker personas.**
The system uniquely keys workers by UUIDv7 (`WorkerId`), so two workers named
"Mariah" coexist as two distinct rows with two distinct UUIDs — no warning, no
disambiguation, no rename suggestion. The only protection against a
double-click double-hire is a process-local `Idempotency-Key` cache that
**the web frontend never populates**, so it has zero effect in practice.

---

## Scope of investigation

Files / paths examined:

- Frontend hire wizard:
  - `web/app/(hire)/hire/page.tsx` (entry / Step 1)
  - `web/app/(hire)/hire/step/[step]/page.tsx` (Steps 2–5 page-level controller)
  - `web/components/skycot/hire/Step1Land.tsx`
  - `web/components/skycot/hire/Step3Drafting.tsx`
  - `web/components/skycot/hire/Step4Introduce.tsx`
  - `web/components/skycot/hire/WorkerDraftCard.tsx`
  - `web/lib/hire/use-hire-draft.ts`
  - `web/lib/hire/use-genesis-worker-draft.ts`
  - `web/lib/hire/draft-store.ts`
  - `web/lib/hire/draft-types.ts`
  - `web/lib/hire/genesis-stub.ts`
  - `web/lib/hire/celebration-storage.ts`
  - `web/lib/hooks/use-hire-worker.ts`
- Frontend API client:
  - `web/lib/api/workers.ts` (hireWorker, fetchWorker, freeze/unfreeze)
  - `web/lib/api/genesis.ts` (classifyRole, draftWorker)
  - `web/lib/api/client.ts` (typed fetch wrapper)
- Backend hire / Genesis routes:
  - `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` (J5)
  - `crates/argo-cli/src/handlers/gateway/routes_genesis.rs` (J4)
- Backend persistence:
  - `crates/argo-identity/src/store.rs` (`create_worker`, `create_worker_from_draft`)
  - `crates/argo-identity/src/store_hire.rs` (`HireDraft`, `validate_hire_draft`, `assemble_worker_from_draft`)
  - `crates/argo-store/src/models.rs` (`WorkerId::new` → UUIDv7)
  - `crates/argo-store/src/migrations/v12.rs` (persona / worker / duty schema)
  - `crates/argo-genesis/src/lib.rs` (`name_to_persona_slug`)

---

## Scenario 1 — Hire two workers named "Mariah" with different descriptions

**Outcome:** Both hires succeed. The tenant ends up with two `worker` rows whose
`persona.name = "Mariah"`. They differ by `WorkerId` (UUIDv7) and by `persona.id`
(also UUIDv7) and by `persona_id` foreign key on the `worker` row.

**Where it could have been caught — but isn't:**

1. **Genesis `POST /api/v1/genesis/draft-worker`**
   (`routes_genesis.rs:124-177`) is stateless — it reads the `answers.name`
   value and echoes it back as `suggested_name` after running it through
   `name_to_persona_slug` (a deterministic kebab-caser, no DB lookup). Two
   identical inputs produce identical output. No call to `IdentityStore`.

2. **Step 4 frontend introduce step**
   (`Step4Introduce.tsx`, `WorkerDraftCard.tsx`) lets the user inline-edit
   the name (`InlineEditable` with `minLength={1}`). No `existing-workers`
   query is fired; no warning is rendered if the suggested name matches an
   existing worker in the tenant.

3. **Hire-commit handler `POST /api/v1/workers`**
   (`routes_workers_hire.rs:51-139`) decodes the body, parses `HireDraft`,
   calls `validate_hire_draft`, then calls
   `IdentityStore::create_worker_from_draft`. None of these touch the
   `persona` table to check for an existing row.

4. **`validate_hire_draft` (`store_hire.rs:116-149`)** only checks:
   - `suggested_name` non-empty (trim)
   - `charter.len() ≥ 20`
   - `suggested_vertical` non-empty
   - `duties` non-empty AND at least one `defaultEnabled = true`
   - `approval_policy.level ∈ {conservative, balanced, aggressive}`

   No name-collision rule.

5. **Schema layer (`v12.rs:67-89`)** —
   `persona.worker_id TEXT NOT NULL UNIQUE` enforces 1-persona-per-worker
   (a 1:1 relation). `persona.name TEXT NOT NULL` has no constraint at all.
   `worker.id TEXT PRIMARY KEY` is the only worker-side uniqueness, and it's
   on `WorkerId`, which is fresh on every call. **No `UNIQUE(tenant_id, persona.name)` anywhere.**

6. **Idempotency cache (`routes_workers_hire.rs:212-247`)** keys on
   `(tenant_id, Idempotency-Key)` and only fires if the header is present.
   Web frontend never sets the header (see "Scenario 2" below).

**Net result:** Two "Mariah"s sit in the cohort list. Their initials are
identical. The team-page UI keys on `WorkerId`, so each Mariah is a distinct
row. Audit logs distinguish them by `worker_id`, but UI surfaces that only
show `persona.name` (e.g. inbound message routing, scheduled-duty cron output)
are ambiguous.

---

## Scenario 2 — Hire two workers with the SAME description (double-click / double-submit)

**Outcome:** Two distinct workers are created. The system **does not dedupe by
input content**. There are three layers of partial protection — each leaks:

### Layer A — Button-disabled while pending (`Step4Introduce.tsx:118`)

```tsx
disabled={adjustOpen || redraft.isPending || hire.isPending}
```

This stops a literal double-click within the same React mount. It does NOT
protect against:

- User clicks Hire, the request takes 8 s, user gets impatient, refreshes
  the page, returns to Step 4, clicks Hire again → two workers.
- User clicks Hire on tab A, gets bored, opens tab B (same draft from
  localStorage), clicks Hire there too → two workers (two POSTs).
- Network blip: `hire.commit` resolves but the `200 OK` body is dropped;
  React Query's `isError=true` fires, the user sees the toast,
  clicks "Retry" → two workers.

### Layer B — Hire mutation has `retry: false` (`use-hire-worker.ts:48`)

This is correct (it prevents React Query from auto-retrying the POST). But
the manual retry button on the error toast (`Step4Introduce.tsx:113-117`)
calls `hire.commit` again unconditionally. If the first commit actually
succeeded server-side but returned a transient 5xx (e.g. write-ahead-log
fsync race), the manual retry creates a duplicate.

### Layer C — Server-side `Idempotency-Key` cache (`routes_workers_hire.rs:42-247`)

The gateway DOES support `Idempotency-Key`: keyed on
`(tenant_id, key) → (body_digest, worker_id, 24h-TTL)`. If the same key
arrives twice with the same body, returns the original `worker_id` and
`{success: true, idempotent: true}`. Mismatched body → 409 Conflict.

**However:** `web/lib/api/client.ts:62-78` (the typed fetch wrapper) does
NOT set `Idempotency-Key`. `web/lib/api/workers.ts:hireWorker` does not
pass one through. Grep confirms zero occurrences of `Idempotency-Key`
anywhere under `web/`. Every hire commit travels without the header, so
the gateway cache never matches.

Additional weakness: the cache is **process-local**
(`OnceLock<Mutex<HashMap<…>>>`, lines 212-215). Multi-instance gateway
failover (Track-H) would create dupes even WITH the header. The brief calls
this out as `mission-enterprise-hire-idempotency-redis` for a future Redis
backing.

### Bonus — celebration storage DOES dedupe (correctly)

`web/lib/hire/celebration-storage.ts:88-102` upserts on `workerId`, so the
confetti page never queues two banners for the same hire. This is the only
dedup logic in the entire hire path, and it lives on the wrong layer
(post-hire UI surfacing, not pre-hire commit).

---

## Scenario 3 — Worker ID uniqueness

**Verdict: GUARANTEED.** `WorkerId(uuid::Uuid)` in `argo-store/src/models.rs:19-29`
is generated via `Uuid::now_v7()` on every `WorkerId::new()` call. UUIDv7 is
time-ordered + 62 bits of randomness, so collision probability is effectively
zero. The schema has `worker.id TEXT PRIMARY KEY`, so an actual collision
would surface as an `ArgoError::Provider("insert worker: …")` and short-circuit
the transaction — no silent overwrite.

This is the one layer of the architecture that holds. The problem is that
worker-id-uniqueness is not the same as worker-name-uniqueness, and the user
experiences names, not UUIDs.

---

## Scenario 4 — Visible naming guidance

**Verdict: NONE.** No part of the hire wizard:

- Pre-checks the name against existing workers in the tenant.
- Suggests a different name when the proposed one is taken.
- Surfaces a "Mariah already exists in your cohort" warning.
- Offers a numeric suffix (Mariah, Mariah 2, Mariah-2).
- Shows the current cohort of workers as a sidebar for context.

The `Step1Land.tsx` placeholder rotates among `PLACEHOLDER_PROMPTS`
(role-shaped examples) and the `ExamplesPopover` offers pre-canned example
roles (`web/lib/hire/examples.ts`), but these are pre-Genesis input prompts,
not name-collision hints. Step 4's `InlineEditable` for the name has no
async validation hook.

The `routes_genesis.rs` `draft_worker_response` echoes back whatever
`answers.name` the user typed; the deterministic-stub
`stubWorkerDraft` (`web/lib/hire/genesis-stub.ts:53`) hardcodes the name
"Mariah" regardless of input, which is **the single largest UX risk**: any
user who completes Step 2 while the gateway is unreachable will see "Mariah"
suggested. If they accept the default each time, the tenant accumulates
identical "Mariah"s every time Genesis is offline.

---

## Top 3 risks

### Risk 1 — Silent dupes on retry (HIGH severity)

The combination of (a) no frontend `Idempotency-Key`, (b) no server-side
name-or-content dedup, and (c) retry-button in the error toast means a
single user can easily produce 2-3 identical workers by clicking through a
transient gateway flake. There is no visible "you already have a Mariah —
did you mean to edit her instead?" confirmation step. This is a
**data-integrity bug** that goes live on first commercial use.

**Fix shape:** Two layers.
- **Frontend (cheap):** generate an `Idempotency-Key` UUID in
  `useHireWorker` on mount, persist in the draft store, send on every
  retry. ~20 LOC.
- **Backend (correct):** add `validate_hire_draft` check that queries
  `persona` for `(tenant_id, name)` and either rejects with a 409 + a
  `{ suggested_alternatives: ["Mariah 2", "Mariah Jr"] }` payload, or
  returns the existing `worker_id` with an `idempotent_by_name: true`
  flag. Operator decision needed: hard-reject vs. auto-suffix vs.
  warn-and-allow.

### Risk 2 — Genesis-stub "Mariah" floods tenants (MEDIUM-HIGH severity)

`web/lib/hire/genesis-stub.ts:53` returns
`{ suggestedName: "Mariah", … }` for every problem statement when the
gateway is unreachable (404 / malformed response). The stub also exists
on the live deployment paths because the brief explicitly preserves the
fallback for resilience. If Genesis goes down for an hour and ten users
each hire through the stub flow, the tenant has ten Mariahs (or one
Mariah per tenant ten times).

**Fix shape:**
- Genesis-stub gets a small rotation pool, e.g.
  `["Mariah", "Sam", "Riley", "Jordan", "Alex"]`, seeded by
  `Math.floor(Math.random() * pool.length)` so consecutive stub hires
  diverge. ~5 LOC.
- OR remove the suggested-name field entirely from the stub and force
  Step 4's `InlineEditable` to require a user-typed name before the
  Hire button enables. Higher friction but eliminates the dupe-magnet
  default.

### Risk 3 — Cohort UI ambiguity from duplicate persona names (MEDIUM severity)

Even setting aside accidental dupes, an operator might LEGITIMATELY want
two workers named "Mariah" (e.g. "Mariah on East coast", "Mariah on West
coast", "Mariah for VIP customers"). Every UI surface that shows
`persona.name` without `WorkerId` disambiguation becomes ambiguous:

- Cohort page (locked design `D3 magazine tiles`) — the lead tile + stacked
  tiles render `persona.name`; two Mariahs are visually identical.
- Audit log (locked design `B3-enterprise Etherscan-style`) — rows show
  `principal: persona.name`, so "Mariah approved $400 PO" is ambiguous if
  two Mariahs exist.
- Approvals inbox (locked design `E-ops-dashboard`) — "Mariah requested
  approval" needs the worker-id badge.
- Inline-edited example replies (`Step4Introduce.tsx:167`) read
  `workerName={workerDraft.suggestedName}` — at runtime, two Mariahs send
  messages signed "Mariah" indistinguishably from the recipient's end.

**Fix shape:** When two-or-more workers share a `persona.name` within a
tenant, every UI surface that renders the name adds a disambiguation
suffix (`Mariah · West coast` from `persona.bio`, or
`Mariah #2` from creation order, or a short worker-id fragment
`Mariah · 7e9f`). The cohort UI also needs an "merge into existing worker?"
flow for accidental duplicates — operator-only, capability-gated.

---

## Additional findings

### Finding A — `worker_actions` rate-limit / quota does NOT cover hire calls

`routes_workers_hire.rs` does NOT invoke `super::rate_limit` — only
`routes_genesis.rs:8` imports it. A scripted attack could POST 1000 hires
in 60s and the gateway accepts every one (assuming bearer-auth succeeds,
the `enforce_role` allows it, and the vertical id resolves).

**Recommendation:** add a per-tenant rate-limiter on `WorkerHire` action
in the dispatch layer. The brief
`mission-enterprise-tenant-worker-quota` (referenced in the file header
at line 25) is the proper home; until it lands, a 60-second sliding window
of `max 10 hires per tenant` would block the obvious DoS pattern.

### Finding B — `Idempotency-Key` cache lacks observability

`routes_workers_hire.rs:96-105` silently swallows cache hits / misses /
conflicts. No `tracing::info!` on a Match, no metric counter on a Conflict.
Once we wire the frontend to send keys, we'll have zero observability into
whether the cache is doing anything. Recommend adding two
`tracing::info!` + two `metrics::counter!` instrumentation points.

### Finding C — `persona.name` length is unbounded

The schema (`v12.rs:72`) has `name TEXT NOT NULL` — no
`CHECK(length(name) <= N)`. `validate_hire_draft` only checks non-empty.
A worker named with 10 KB of text would parse, persist, and break every
UI surface that renders it. **Recommend** `CHECK(length(name) BETWEEN 1 AND 64)`
in a forward-only migration.

### Finding D — `name_to_persona_slug` is collision-prone

`name_to_persona_slug("Maria") == name_to_persona_slug("Maria!") == "maria"`.
The slug is used by Genesis-side persona resolution in the planning pipeline.
Confirm there's no downstream `(tenant_id, persona_slug)` lookup that would
silently merge two distinct workers when their names happen to slugify the
same way.

---

## Verdict (200 words)

**Collision behavior:** the system creates a fresh `WorkerId` (UUIDv7) per
hire, so two workers named "Mariah" coexist as two distinct rows under two
distinct UUIDs. The user sees no warning, gets no disambiguation prompt,
and ends up with two indistinguishable Mariahs in their cohort list.
Worker-id uniqueness is the only guarantee that holds — name uniqueness is
not enforced at the schema, validation, or UI layer. Genesis is stateless
and never consults existing-workers.

**Duplicate-prevention verdict: effectively absent for the user-visible
plane.** The server has an `Idempotency-Key` cache, but the web frontend
never sends the header, so it has zero effect. Manual retry on the
error-toast can produce duplicate workers when the first commit succeeded
but returned a transient 5xx. The genesis-stub fallback hardcodes
"Mariah" — every gateway-down hire produces another Mariah.

**Top 3 risks:** (1) silent dupes on retry — frontend Idempotency-Key
missing, no name-or-content dedup; (2) genesis-stub "Mariah"
proliferation when gateway is unreachable; (3) cohort / audit / approvals
UI rendering `persona.name` without worker-id disambiguation makes legitimate
same-name workers indistinguishable to operators. Fixes are 20-50 LOC each
in the frontend + a small schema-level `UNIQUE` decision (auto-suffix vs.
hard-reject).
