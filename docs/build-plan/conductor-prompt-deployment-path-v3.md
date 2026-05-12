# Conductor prompt — DEPLOYMENT-PATH DIRECTIVE v3

Authored 2026-05-12. Supersedes v2 with the four review fixes applied:
§1 advisory dependency resolved (advisories exist; row-insertion is
mechanically specified); §2 tier enforcement uses BLOCKED status not
salvage-note convention; §3 K-brief paths verified by `rg` + K-tier
split into 5a (parallel-now) and 5b (after Tier 0+1); §5/§6/§7 row-
count math and throughput floor reconciled.

Paste the block below to the conductor verbatim.

---

```
DEPLOYMENT-PATH DIRECTIVE v3 (supersedes v2)

You are the conductor. Stop normal replenishment. Your sole
objective until §5 stop conditions are met is to drive the swarm
to "deployable end-to-end."

=== CURRENT STATE (2026-05-12) ===
- 446 claude-MERGED rows in BUILD_STATUS.md.
- 28 claude-TODO rows; 3 CLAIMED in flight; 1 IN_REVIEW.
- 4 advisories EXIST on main as `docs/build-plan/conductor-
  advisory-{E0-E1,E2-E5,F-reconciliation,G-J}.md` (verified
  2026-05-12; total 1,066 LOC).
- 38 E-track briefs + 18 J-track briefs + 3 G-track briefs ALREADY
  AUTHORED as files in `docs/build-plan/streams/` — but NONE of
  them have ledger rows yet (verified: 0 E*/J*/G* matches in
  BUILD_STATUS.md).
- 5 K-track deployment briefs authored by this directive (K1-K5).
- Grand total remaining to "deployable end-to-end":
    2 F-reconciliation flips
  + 7 E0-E1 row inserts
  + 31 E2-E5 row inserts
  + 21 G-J row inserts
  + 5 K-track row inserts (this directive)
  + outstanding existing TODO rows that don't block deployment
  = 66 ledger-edit operations producing 64 net new claimable rows
    (2 F-rows are pre-existing).

=== THROUGHPUT + ETA ===
Trailing-6h rate: ~5/hr (3 PRs merged in 90 min on 2026-05-12).
Targets:
  - Baseline (5/hr):  ~13 swarm-hours.
  - Stretch (10/hr):  ~7 swarm-hours.
Escalate (per §6) if trailing-6h rate < 4.0/hr for 2 consecutive
ticks. There is no separate "floor" — 4.0/hr IS the floor; v2's
3.6 figure was stale.

=== §1. INSERT QUEUED ADVISORIES ===

Apply in this order, ONE commit per advisory. The advisories are
mechanical paste — their fenced "Paste this to the conductor"
blocks list each row's note text + commit message verbatim.

  1. conductor-advisory-F-reconciliation.md (2 BLOCKED→TODO flips
     + 1 confirm)
     - Flip `mission-skill-propagation-loop` BLOCKED → TODO.
     - Flip `mission-skill-session-boot-wire` BLOCKED → TODO.
     - Use note text VERBATIM from the advisory.
     - Two separate commits per advisory's convention:
         `status: unblock mission-skill-propagation-loop TODO`
         `status: unblock mission-skill-session-boot-wire TODO`

  2. conductor-advisory-E0-E1.md (7 new TODO/BLOCKED rows)
     - Insert E0-frontend-foundation as TODO (CONTRACT — claim-
       able immediately).
     - Insert E1-a..f as BLOCKED — see §2 for enforcement.
     - ONE commit: `briefs: 7 E0-E1 frontend rows (advisory E0-E1)`

  3. conductor-advisory-E2-E5.md (31 new rows)
     - CONTRACT rows TODO: E4-a (worker-profile shell), E5-a
       (audit feed). Both claimable immediately.
     - All other E2-* / E3-* / E4-b..i / E5-b..h rows: BLOCKED on
       their CONTRACT (E4-a or E5-a or the relevant J* brief).
     - ONE commit: `briefs: 31 E2-E5 frontend rows (advisory E2-E5)`

  4. conductor-advisory-G-J.md (21 new rows)
     - CONTRACT rows TODO: J1 (approvals), J4 (genesis), J7
       (worker detail), J14 (audit feed). Claimable immediately.
     - J2 (PII redaction) TODO — blocks E1-e but is itself
       independent.
     - J3, J5, J6, J8-J13, J15-J18 BLOCKED on their CONTRACT J*
       brief per §2.
     - G1/G2/G3 TODO — independent of CONTRACT chain.
     - ONE commit: `briefs: 21 G+J rows (advisory G-J)`

After all 4 advisories committed: PRODUCT_ROADMAP.md Tracks E, F,
G, J reflect IN PROGRESS (Track F already does); add Track K row.

=== §2. TIER ENFORCEMENT VIA BLOCKED STATUS ===

v2 mistake: relied on salvage-note convention ("BLOCKED until X
MERGES") with TODO ledger status. Workers self-claim TODO rows
without reading the note. This let lower-tier rows race ahead.

v3 fix: USE THE BLOCKED LEDGER STATUS for downstream rows. The
existing pattern (already used on `mission-skill-propagation-
loop` etc.) — workers respect BLOCKED status; claim-with-retry
will not claim a BLOCKED row.

Conductor responsibilities:
  - Insert non-CONTRACT rows in BLOCKED state with note format:
      "**BLOCKED on <contract-brief-id> MERGE** — flip to TODO
       once the contract lands. Brief: streams/<file>.md.
       <existing brief summary>"
  - When a CONTRACT brief's PR merges, on the SAME tick that
    flips the contract row to MERGED, ALSO flip every dependent
    BLOCKED row to TODO. Use the dependency table below.
  - Commit message for cascade flip:
      "status: cascade unblock <N> rows on <contract-brief> MERGE"

Dependency table (CONTRACT → dependents):
  E0-frontend-foundation       → E1-a, E1-b, E1-c, E1-d, E1-e,
                                  E1-f, E2-a, E3-a, E4-a*, E5-a*
                                  (* = sub-CONTRACTs; flip them
                                  TODO not their downstreams yet)
  E4-a-worker-profile-shell    → E4-b..E4-i (8 rows)
  E5-a-audit-timeline-feed     → E5-b..E5-h (7 rows)
  J1-gateway-approvals-json    → E1-a (already gated on E0; this
                                  is the second gate — flip to
                                  TODO only when BOTH J1 + E0
                                  have MERGED)
  J4-gateway-genesis           → E2-b..i (8 rows; gated also on
                                  E0 + E2-a)
  J7-gateway-worker-detail     → E4-b..f (5 rows; gated also on
                                  E0 + E4-a)
  J14-gateway-audit-feed       → E5-b..h (7 rows; gated also on
                                  E0 + E5-a)

For double-gated rows (e.g. E1-a needs both E0 AND J1), the
BLOCKED note lists both contracts. The cascade flip only fires
when the LAST gate merges.

=== §3. AUTHOR TRACK K — DEPLOYMENT RECIPE BRIEFS ===

K-track is split into Tier 5a (parallel-now — doesn't depend on
Tier 0) and Tier 5b (after Tier 0+1 lands).

Tier 5a — package what builds today:
  K1 deploy-docker-compose-stack
  K2 deploy-k8s-manifests
  K3 deploy-single-vm-systemd

Tier 5b — needs typed surfaces:
  K4 deploy-first-run-setup-wizard (needs E0 + at least one J*
     endpoint to talk to)
  K5 deploy-smoke-test-e2e (exercises a real hire→approve flow
     through the deployed stack; needs J1 + J4 + J5 + E0 + E1-a +
     E2-a-b MERGED at minimum)

Per-brief detail:

Brief K1 — `K1-deploy-docker-compose-stack`
  Branch: claude-K1-deploy-docker-compose-stack
  PR:     deploy: docker-compose stack (argo + web + postgres) (K1)
  Files (NEW):
    - deploy/docker/Dockerfile.argo
    - deploy/docker/Dockerfile.web
    - deploy/docker/docker-compose.yml
    - deploy/docker/docker-compose.prod.yml
    - deploy/docker/.env.example
    - deploy/docker/README.md
  Status: TODO immediately (Tier 5a parallel-now).
  Acceptance:
    - Both images build clean on multi-arch (amd64 + arm64).
    - `docker-compose up` from clean clone, no manual steps.
    - Web SPA reachable on :3000; gateway on :8080.

Brief K2 — `K2-deploy-k8s-manifests`
  Branch: claude-K2-deploy-k8s-manifests
  Status: TODO immediately (Tier 5a parallel-now).
  Files (NEW):
    - deploy/k8s/base/{argo,web,postgres}-{deployment,service,
      configmap,secret.example,ingress,kustomization}.yaml
    - deploy/k8s/overlays/{dev,prod}/kustomization.yaml
    - deploy/k8s/README.md
  Reuses images from K1. Liveness/readiness probes. HPA stub.
  Kustomize overlays. Acceptance: `kubectl apply -k
  deploy/k8s/overlays/dev` succeeds against kind/minikube; pods
  Ready in < 60s.

Brief K3 — `K3-deploy-single-vm-systemd`
  Branch: claude-K3-deploy-single-vm-systemd
  Status: TODO immediately (Tier 5a parallel-now).
  Files (NEW):
    - deploy/vm/install.sh (idempotent bootstrap)
    - deploy/vm/argo.service (systemd)
    - deploy/vm/web.service (systemd)
    - deploy/vm/nginx-skycot.conf
    - deploy/vm/certbot-renew.cron
    - deploy/vm/README.md
  Target: Ubuntu 24.04 LTS VM. install.sh idempotent. SLA: "≤ 15
  min from fresh VM" (NOT the v2 "10 min" — that assumed pre-
  built release artifacts. v1 builds from source via cargo; web
  via npm).

Brief K4 — `K4-deploy-first-run-setup-wizard`
  Branch: claude-K4-deploy-first-run-setup-wizard
  Status: BLOCKED on E0-frontend-foundation MERGE.
  Files (touched — paths VERIFIED 2026-05-12):
    - crates/argo-cli/src/handlers/setup.rs (EXTEND — exists)
    - crates/argo-cli/src/handlers/setup_steps.rs (EXTEND)
    - crates/argo-gateway/src/router.rs (EXTEND — add /setup
      route; NOT crates/argo-gateway/src/handlers/setup.rs,
      that subdir doesn't exist)
    - crates/argo-gateway/src/adapters/setup.rs (NEW)
    - web/app/(setup)/setup/page.tsx (NEW — route group)
    - web/lib/api/setup.ts (NEW)
  Scope: `argo setup` CLI (already partial); add HTTP /setup
  endpoint; first browser visit pre-setup redirects to wizard;
  migrations run on first start.

Brief K5 — `K5-deploy-smoke-test-e2e`
  Branch: claude-K5-deploy-smoke-test-e2e
  Status: BLOCKED on K1 + J1 + J4 + J5 + E1-a + E2-a MERGE.
  Files (NEW):
    - .github/workflows/deploy-smoke.yml
    - deploy/smoke/run.sh
    - deploy/smoke/checks.ts
  Workflow: build docker images → docker-compose up → run
  checks → tear down. Checks: /health 200, SPA serves, hire→
  approve happy path via API. First green run on main cuts the
  §5 stop-condition.

Commit pattern (5 briefs, ONE commit):
  `briefs: K1-K5 deployment recipe (Tier 5a/5b split)`

=== §4. SUPPRESS NORMAL REPLENISHMENT ===

Until §5 stop conditions hit:
  - Do NOT author new mission-* briefs from MISSION_BACKLOG.
  - Do NOT author new H/I track briefs.
  - Replenishment of queue ONLY from the 66 row-edits queued by
    this directive.

If queue depth (TODO + IN_REVIEW + CLAIMED) drops below 8 rows
AND fewer than 20 directive rows remain unMERGED, THEN you may
pull from MISSION_BACKLOG to keep workers busy — but only
mission-* rows that are NOT Track H/I.

Operator may rescind by editing the ledger header (remove the
DIR-v3 chip line) or by issuing a v4 directive.

=== §5. STOP CONDITIONS — "DEPLOYABLE END-TO-END" ===

Drop the directive when ALL boxes are checked:

  [ ] All 7 CONTRACT briefs MERGED:
      E0-frontend-foundation
      E4-a-worker-profile-shell
      E5-a-audit-timeline-feed
      J1-gateway-approvals-json-api
      J4-gateway-genesis-classify-endpoint
      J7-gateway-worker-detail-endpoint
      J14-gateway-audit-feed-endpoint
  [ ] All 18 J* gateway rows MERGED (J1-J18).
  [ ] All 38 E* frontend rows MERGED (E0-E5).
  [ ] G1, G2, G3 MERGED.
  [ ] Track F remaining 3 rows MERGED (after F-reconciliation
      flips: propagation-loop, publish-verified-ingest, session-
      boot-wire).
  [ ] K1-K5 MERGED.
  [ ] K5 deploy-smoke-test workflow has passed at least once
      on main.

When all 7 boxes check: post final status:

  DEPLOYMENT-PATH DIRECTIVE v3 COMPLETE.
  Stack is deployable end-to-end.

Then resume normal MISSION_BACKLOG replenishment.

=== §6. ESCALATION RULES ===

Escalate to operator (single line in ledger header) if:
  - Trailing-6h merge rate < 4.0/hr for 2 consecutive ticks.
  - Any CONTRACT brief BLOCKED > 2 hours (BLOCKED here means
    the row carries a BLOCKED ledger status, not the salvage-
    note convention).
  - K5 smoke fails after K1-K4 MERGED.
  - Worker reports CONTRACT brief contradicts gateway/runtime in
    a way that needs operator schema judgment.

=== §7. PROGRESS REPORTING ===

Every 10 merges (NOT every N ticks — rate-independent), append
to ledger header:

  DIR-v3: <merged>/66 · Tier0: <m>/7 · J: <m>/18 · E: <m>/38 ·
  G: <m>/3 · F: <m>/3 · K: <m>/5

Tier 0 (CONTRACT) is a subset of E + J; reported separately for
visibility. Sum: 7+18+38+3+3+5 = 74 raw rows; some E briefs are
CONTRACT and counted twice in the chip (intentional — gives the
swarm a visible Tier-0 burn-down).

When chip reads `66/66` and box-7 (K5 green on main): run §5
check.

=== §8. GO ===

Tick 1 (THIS TICK):
  - Apply F-reconciliation (2 commits: unblock the 2 skill rows).
  - Author K1-K5 (1 commit: 5 brief files + 5 ledger rows;
    K1-K3 TODO, K4-K5 BLOCKED per dependency table).
  - Update PRODUCT_ROADMAP.md with Track K row + Track F status.

Tick 2 onward:
  - Insert E0-E1, E2-E5, G-J advisories (3 commits — one per
    advisory; F already applied tick 1).
  - Then serve worker claims by tier order. Cascade-unblock
    dependents on CONTRACT merges per §2 dependency table.
  - Report chip every 10 merges per §7.
  - Escalate per §6.
  - Check §5 each tick.

End directive.
```

---

## Notes (not pasted to conductor)

- v3 differs from v2 in four ways: (1) advisories confirmed
  to exist + insertion is mechanical; (2) tier enforcement uses
  the BLOCKED ledger status (proven pattern from the 2 skill
  rows) rather than salvage-note convention; (3) K-brief paths
  verified against the live tree (`crates/argo-gateway/`
  doesn't have a `handlers/` subdir; routes live in `router.rs`);
  (4) row-count math is internally consistent (66 edits, 74
  rows with intentional Tier-0 double-counting in §7).

- The cascade-unblock mechanism in §2 is the load-bearing
  change. Workers need a real signal to wait — BLOCKED ledger
  status is that signal. The conductor pays a small cost (extra
  flips on CONTRACT merges) for rock-solid ordering.

- Tier 5a (K1-K3) can start the moment this directive lands.
  They package what builds today; no Tier-0 dependency. This
  shaves multiple swarm-hours off the v2 schedule.

- K3's SLA was loosened from "≤ 10 min" to "≤ 15 min" because
  no release artifacts are published yet — v1 builds from source.
