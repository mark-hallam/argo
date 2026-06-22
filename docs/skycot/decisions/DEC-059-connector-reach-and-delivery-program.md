# DEC-059 — Connector-reach + delivery program (Wave 1 + Wave 2)

**Date**: 2026-06-21 · **Status**: active · **Operator-locked**: yes ("yeh make the execution plan and go, use ultra code", 2026-06-21; "keep progressing, do each wave one after the other", 2026-06-22)

## Decision

SKYCOT runs a focused, three-wave **connector-reach + delivery program** to close
the single biggest gap between "a customer hands a teammate a job" and "a real
product is delivered." The program is **wiring and surfacing, not greenfield** —
the capabilities were already built; three load-bearing wires were simply not
connected in the running product.

## Context — the limiter is tool REACH, not reasoning

A 6-agent, code-grounded strategy audit
(`docs/skycot/strategy-full-report-2026-06-21.html`) reached one core insight:
the teammates already **reason** well. When a teammate stops short of delivering,
it is because it cannot **reach** the tool it needs — it drafts the right answer,
then has nowhere to send it. Three concrete wires were missing in production:

1. **(①) Connector binaries weren't in the prod image.** The root `Dockerfile`
   built only `--bin argo`, so the `argo-mcp-connector-*` subprocess binaries —
   real, wired, tested — were absent from the deployed container. A connected
   connector degraded to a dry-run/no-op for lack of a binary on `PATH`.
2. **(②) Customer keys never reached the connector process.** `argo-mcp/
   client.rs:121` spawned the connector subprocess with no `.env()`, so even a
   stored tenant credential never made it into the process that needed it.
3. **(③) No live chat→delivery bridge.** `send_message.rs:103` used the
   `UnwiredDispatcher`, and `materialize.rs:198` minted every action with
   `executor = human` — so an in-turn "go do this" had no real path to a tool
   and no way to actually fire.

The audit framed an 11-item, 3-wave program against this. The waves below were
authorized and executed in order.

## Wave 1 — the unlock (SHIPPED + LIVE in prod, gateway task-def :13, 2026-06-21)

- **#3004 — ship the 17 wired connector binaries into the prod image + inject
  per-tenant creds at spawn.** Closes ① and ②. Adversarially reviewed for
  cross-tenant isolation and found sound: the connector pool is keyed
  `(tenant_id, manifest_id)`, and `load_tenant` reads **only** that tenant's
  credential section under that tenant's own master key — one tenant can never
  spawn a connector carrying another tenant's secret.
- **#3007 — durable credential path.** `argo_config::cred::workspace_root()`
  resolves the credential vault under `$ARGO_WORKSPACE` (= `/data`, the durable
  volume), not the ephemeral `/app` — so a stored secret survives a task
  replacement instead of vanishing with the container.
- **#3008 — credentials ride the durable snapshot.** Capture
  `INCLUDE_CREDENTIALS=1` in Terraform so the periodic S3 snapshot
  (the same path as DEC-056) backs up the **encrypted** credential vault, and
  the snapshot survives task replacement; plus an image-tag reconcile.
- **#3005 — hire→duty bridge fix.** The SPA hire form's `WorkerDuty{schedule}` /
  `successCriteria` now map to the canonical `HireDraft{cron, kpis}`, killing the
  silent no-op-duty churn (a scheduled responsibility that never persisted).
- **#3006 — honest connector roster.** The 12 unwired connectors are surfaced as
  "coming soon" rather than presented as ready (SPA redeploy of this roster is
  still pending).

**LIVE proof (DEC-029 truth-bounded):** gateway task-def **:13**, image
`wave1-371292fb3`, `INCLUDE_CREDENTIALS=1`; the 17 `argo-mcp-connector-*`
binaries are on `PATH` in the container; `/health` returns 200; rollout reported
0 failed tasks. The **17 token-auth connectors do REAL work end-to-end** for any
tenant who has stored a credential. The **5 OAuth connectors (gmail, gcal,
hubspot, notion, salesforce) remain 🟡 Partial** — there is no `/connect/<id>`
interactive-OAuth writer route yet, and they require operator OAuth-app
registration before a tenant can self-connect.

## Wave 2 — deliver + hire right (MERGED; deployed gateway task-def :14, 2026-06-22)

- **#3009 — per-tenant scheduled-autonomy opt-in, default OFF.** A tenant can opt
  a teammate into firing its scheduled duties autonomously. This is **decoupled
  from auto-approval**: a fired duty's actions still hit the approval gate.
  Adversarially reviewed SAFE. The operator standing rule (per-tenant autonomy
  defaults OFF) is honored in code.
- **#3010 — live in-turn `enqueue_action` delivery tool.** Closes ③ via the
  **dispatcher-callback pattern**: the tool + its trait live in the low-level
  `argo-tool` crate, and the real `BridgeDeliverDispatcher` backend lives in
  `argo-cli` and is injected **downward** — so there is no upward crate
  dependency. Adversarially reviewed SAFE on every path: the bridge always
  **parks the action pending** with `requires_approval` stamped **from the
  manifest** (the model cannot smuggle in approval fields), the default executor
  is `human` so it **fails closed**, the tool is **NOT** in the always-on tool
  set (it is dormant until a vertical profile declares it), and the tool result
  is truth-bounded (`delivered: false`).
- **#3011 — Genesis richening.** Broad-role, from-scratch synthesis now emits
  **responsibilities + typed KPIs + a cron cadence** (matching the #3005
  hire→duty bridge) + a **least-privilege access whitelist scoped to the 17
  wired connectors**, all **bound draft-only**. This kills the weak
  "(derive your responsibilities from the charter)" fallback. The LLM-gate and
  the fail-closed hire-gate are untouched.

**Remaining Wave-2 item (deferred — SPA-heavy):** ⑤ **upfront-capture** — make
Access wirable during hire, fold the Brand Kit (DEC-058) into the hire flow, and
show a completeness preview before the teammate is minted.

## Wave 3 — monetize, harden, surface the moat (PLANNED)

- **③ Overage-billing scheduled caller.** The 10% markup and real-$ overage
  metering already shipped (#2996 / #2999); the **only** missing piece is the
  EventBridge → ECS cron that runs `argo billing report-overage` — pure infra.
- **⑨ Customer-facing audit trust surface.** Promote the existing hash-chained
  audit into a product surface: a verified-delivery certificate, a "prove this
  chain is intact" check, and a compliance export.
- **⑩ Per-worker `TruthBoundaryPolicy` routing.** Load the truth-boundary policy
  per-teammate at session boot, replacing the current process-global policy.
- **⑪ Truth-bound the surface.** Roster honesty is done (#3006); the rest is
  wiring or removing the missing connector authorize specs and exposing the
  `/health` git-SHA via a build-arg.

## Guardrails (locked, non-negotiable)

- The **approval gate is NEVER bypassed.** The delivery bridge parks the action
  pending; **autonomy ≠ auto-approve**.
- **Least-privilege connector access** — a teammate is whitelisted to the
  connectors it needs, no more.
- **Truth-bounded tool results** — the delivery tool reports `delivered: false`
  until the action actually fires; no theater (DEC-029).
- **Per-tenant scheduled autonomy defaults OFF** — the operator standing rule.
- **Per-tenant credential isolation** — pool keyed `(tenant_id, manifest_id)`;
  each tenant's secrets read only under that tenant's master key.

## Operational lesson

The prod gateway image (`argo` + 17 connectors) needs a **clean-memory machine**
to build. The M1 Max / 32 GB host thrashes on the `argo-cli` release link when
host memory is saturated from sustained builds — the build stalled 3× across
2026-06-21/22, and a **reboot cleared it each time**. Reboot before heavy
gateway builds; a cloud / CI builder is the long-term fix. Separately, the local
post-merge main-verify sweep had silently died on a stale
`/tmp/argo-main-verify.lock` (now fixed).

## Cross-links

- [DEC-025](DEC-025-genesis-done-right-full-synthesis.md) — fail-closed hire gate
  (untouched by the #3011 Genesis richening).
- [DEC-029](DEC-029-truth-bounded-status-reporting.md) — NO THEATER /
  truth-bounded results (the delivery tool's `delivered:false` and the LIVE-proof
  discipline above are direct applications).
- [DEC-042](DEC-042-continuous-self-improvement-loop.md) — the self-improvement
  spine (every loop ends in a checkable verifier).
- [DEC-056](DEC-056-durable-gateway-db-snapshot-restore.md) — the durable DB
  snapshot; the encrypted-credential snapshot rides the same path
  (`INCLUDE_CREDENTIALS=1`).
- [DEC-058](DEC-058-tenant-brand-kit.md) — the Brand Kit; the deferred ⑤
  Brand-Kit-into-hire capture depends on it.

## Consequences

- **Short-term cost (named honestly):** Wave 1 forced a heavier prod image (the
  17 connector binaries) and a more careful credential-durability path than
  base-casing creds into the container — paid because the shortcut would have
  lost a customer's stored secret on the next task replacement. The gateway
  build also now demands a clean-memory machine (see Operational lesson).
- The OAuth connectors stay honestly 🟡 Partial until the `/connect` writer route
  + operator OAuth-app registration land — they are surfaced as such, never
  presented as ready.
- All core invariants hold: multi-tenant by construction, capability-based
  security, hash-chained audit on writes, additive migrations, <500 LOC/file,
  tests for every feature, truth-bounded reporting.
