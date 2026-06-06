# Mission e2e — "can the platform do interview → plan → mint → useful work?"

**GO / NO-GO for the operator's "true test", live-verified 2026-06-06 (overnight autonomous run).**

Every claim below was checked **this session in the live browser + against the live gateway** (schema 90, build_commit `7fbac75` = current `main`). No claim is relayed from the ledger without a fresh check. This follows DEC-029 (truth-bounded status reporting) and the NO-THEATER rule.

---

## ✅ UPDATE 2026-06-06 (later same session) — ALL THREE BLOCKERS CLEARED + verified live

The three blockers below were fixed, merged (operator-approved, DEC-040), the gateway was rebuilt to **schema 91** (`build_commit c84ec73`), the demo tenant re-seeded clean (8 workers), and each blocker **re-verified live against the running gateway this session**:
- **Blocker 1 (tier cap → #2731 `13655b6b2`):** demo workspace tier is now **`enterprise`** (was `free`) — confirmed in the store after re-seed; Genesis hire no longer hits `tier_worker_cap`.
- **Blocker 2 (tenant isolation → #2734 `aa2b14b1c`):** two demo-mint principals resolve to **different** tenants (`019e9b1b-e241…` vs `019e9b1b-e2cc…`), and re-minting the same email returns the **same** tenant (idempotent). The mint JWT now carries a per-principal `tnt` claim.
- **Blocker 3 (worker management → #2732 `05c71376c`):** the worker **detail** route now returns `can_edit_duties:true` + full owner perms for a demo principal (the override the overview route already had) — "Add schedule" + management are live.

**Remaining for the FULL autonomous loop:** the teammate-execution path (B1+B2+B3 of UF Phase B) is now **all merged but default-OFF/dormant** — a hired teammate still drafts only. Turning on real autonomous execution is the operator-gated live flip, pending the conductor-driven adversarial blast-radius review (operator watches) + an explicit go. So: **hire + manage + isolate now work end-to-end; autonomous execution is built, dormant, and awaiting the review + your go-live.**

---

## Verdict (one line) — original finding, preserved for the record (now superseded by the UPDATE above)

**NO-GO** for "mint a teammate that does *autonomous useful work*" in the current live build — **but** the front half of the flow (generate a real, well-specified teammate from a plain-English role) genuinely works and is impressive. The blockers are three concrete, fixable things, not a fundamental dead end.

Plain English: today the platform is a **superb teammate-*designer*** (it reads your role description and writes a real, safety-bounded job spec), but in the live demo you currently **cannot finish hiring**, **cannot manage** the teammate, and the teammate **cannot run on its own** — so the loop doesn't close end-to-end yet. *(↑ the hire/manage/isolate half is now FIXED per the UPDATE banner; only the autonomous-execution half remains, by design, behind the operator-gated flip.)*

---

## What I tested

Drove `/genesis` ("Onboard a new teammate") live, twice, then probed `/team`, a worker's profile + Schedules tab, the hire endpoint, the demo-mint sign-in path, and the gateway permission routes directly with curl.

---

## ✅ What genuinely WORKS (and is good)

1. **Genesis generation is real, not canned.** I described *"A customer support specialist who answers billing questions and escalates refunds over $100 to a human."* In ~45s (a real LLM call, `POST /api/v1/genesis`) it produced a tailored spec:
   - Decomposed the role into **5 concrete actions** each tagged with a **risk level** and **approval requirement**: `close_ticket`, `escalate_to_manager`, `explain_billing`, `log_interaction` (low risk / autonomous) and `process_refund` (**HIGH risk / NEEDS APPROVAL**).
   - It **correctly extracted the $100-refund rule** as an approval-gated action — genuine comprehension of my wording.
   - Strong **truth-boundary** language: *"draft-only at birth… can only draft and propose… physically cannot transmit through any live integration until you grant one."* This is the differentiator, working.
2. **Name parsing works.** Second role *"A research assistant named Remy…"* → the teammate was correctly named **Remy** (the `named X` parser, PR #2681, is live).
3. **The safety gate is real.** The first (refund-handling) role was **blocked by the safety gate** before hire — *"can't be hired… regenerate with a narrower scope."* Safety-first is a genuine, working selling point.
4. **The view/browse surfaces are solid.** Pages load; **dark mode is clean** (0 large white blocks scanned on settings, landing, app shell — this session's dark-mode fixes held); the roster, worker profiles, and tabs all render.

---

## ❌ What BLOCKS the full loop (3 concrete issues)

### Blocker 1 — You can't finish hiring in the live demo (tier cap)
The hire (`POST /api/v1/workers`) returns **403 `tier_worker_cap`**: the demo/showcase tenant is on the **"free" tier (max 1 worker)** but is **seeded with 9 workers**, so every new hire is refused with an upsell.
- This is the sprint's tier-gate enforcement (#2698) working *as designed* — the bug is that the **showcase tenant is on the free tier** instead of **enterprise** (`max_workers: 0` = unlimited).
- **Root cause:** the demo seed (`crates/argo-demo-seed`) never sets `workspace.tier`, so it defaults to `"free"`. There is **no CLI/route to set tier** today — it's only ever set via raw SQL.
- **Important nuance:** a *brand-new* tenant also starts free, but with **0 workers**, so a real new signup *can* hire their first teammate (0 < 1). The cap only bites the *pre-seeded* demo tenant.

### Blocker 2 — Demo principals all share ONE tenant (no isolation)
A freshly **demo-minted** principal (`human:demo+…@skycot.test`, role `owner`) still sees the **same 9 workers** as the anonymous browse. So "sign in to talk to your own workspace" does **not** create a fresh workspace — every demo principal resolves to the **same shared seeded tenant**.
- **Likely root cause:** the demo sign-in JWT carries **no `tenant_id`**, so the WS1 tenant resolver (`JWT → header → cwd`) falls through to the **cwd default tenant** (the shared seeded one).
- Consequence: even the "your own workspace" path lands in the capped shared tenant, so it *also* can't hire (9 > 1, Blocker 1).

### Blocker 3 — Worker management is disabled (permission-override route gap)
On a worker's **Schedules tab**, "Add schedule" is **disabled** and every management permission reads **false** (`can_edit_duties`, `can_edit_charter`, `can_pause_resume`, `can_view_audit`, …).
- **Root cause (precisely isolated):** the demo permission-override (`compute_permissions_demo`, which grants an owner all-true in demo mode) is applied on the **overview** route but **NOT** the **detail** route:
  - `GET /api/v1/workers/{id}/overview` + demo bearer → **all-true** ✅
  - `GET /api/v1/workers/{id}`          + demo bearer → **all-false** ❌
  - The SPA's `useWorker` hook reads perms from the **detail** route, so the Schedules tab (and anything else using `useWorker`) is stuck read-only.
- **Fix:** apply the same `compute_permissions_demo` override in `routes_worker_detail.rs` that `routes_worker_overview.rs` already has. ~1 function, mirrors existing code, gateway-side, needs a cargo test.
- **Second layer:** in plain auth-bypass browse the SPA may not even send the demo bearer; that needs confirming separately.

### (Architectural) — No autonomous-execution path yet
Even past the three access blockers, the teammate cannot *run on its own*: there is **no "Run now"** anywhere, worker schedules are empty, and the scheduler is gated off (`ClosedGate` denies every fire; hire writes the `duty` table while the scheduler reads `role_duty` with no bridge; generated actions are `executor="human"` = draft-only). This is **exactly the operator-driven Phase B** (DEC-036 scheduler / DEC-037 graduated execution / DEC-038 per-teammate cost) that is **deliberately on HOLD** pending operator go-live + adversarial review. Confirmed against the 2026-06-05 usefulness audit; nothing here contradicts that audit.

---

## 🟡 Quality notes (not blockers)
- **Risk mislabel:** Remy's `extract_takeaways` was tagged **HIGH RISK / NEEDS APPROVAL** — extracting takeaways from a saved article is not high-risk. The risk classifier is noisy.
- **Templated action descriptions:** e.g. *"Extract takeaways as the assistant named Remy reads"* — mechanical string concatenation, not natural language. Action *names* are good; *descriptions* read awkwardly.
- **Safety-gate UX dead-end:** a reasonable role with one sensitive action is blocked with only "regenerate narrower" as the exit, even though the approval gate already mitigates the risk. Arguably over-conservative; a product-judgment call for the operator.

---

## What the operator's secret test will hit
- If it's **"sign up fresh and hire your first teammate"**: the *hire-cap* won't block a genuinely-fresh tenant (0 workers) — **but** demo-mint currently shares the capped seeded tenant (Blocker 2), so via the demo path it **will** 403. A *real* email signup that auto-provisions its own tenant is the path most likely to succeed at the first hire.
- If it's **"make the teammate actually do the work"**: it will hit the **no-autonomous-execution** wall (Phase B, on HOLD).

## Smallest set of changes to turn this GO
1. Demo seed sets the showcase tenant to **enterprise** tier (unlimited workers). *(demo-seed + reseed)*
2. Give demo-mint principals an **isolated tenant** (put `tenant_id` in the demo JWT, or provision-on-mint). *(gateway)*
3. Apply the **demo permission-override on the worker detail route** to match overview. *(gateway, ~1 fn)*
4. *(Operator-gated)* turn on Phase B autonomous execution per DEC-036/037 with adversarial review.

Items 1–3 are bug-fixes I can brief for the swarm; item 4 is the operator's interactive call. **Because 1–3 touch tenancy/RBAC/pricing (locked areas), they're surfaced here for the operator to direct rather than auto-applied.**
