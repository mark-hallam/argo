# 01 — Mission claim → crate map

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Update when:** a mission claim is added/removed/reworded, or ownership of a claim shifts crates.

Maps SKYCOT's 6 mission claims to the crates that deliver them. Useful when reasoning about which code path serves which user-visible promise.

## Diagram

```mermaid
flowchart LR
    subgraph M1[Claim 1 — Teammate, not tool]
        IDENTITY1[argo-identity<br/>worker · persona · role · vertical]
        MEM1[argo-memory<br/>per-worker namespace]
        VERT1[argo-vertical<br/>manifests]
    end

    subgraph M2[Claim 2 — Delegation is the unit]
        ORCH2[argo-orchestrator<br/>duty scheduler · roster · handoff]
        TOOL2[argo-tool<br/>delegate_task]
        POLICY2[argo-identity::policy<br/>auto_approval_policy]
    end

    subgraph M3[Claim 3 — Platform builds workers]
        GEN3[argo-genesis<br/>plain-English → vertical]
        VERT3[argo-vertical<br/>vertical-pack distribution]
        SDK3[argo-skill-sdk<br/>TypeScript SDK]
        PACKS3[packs/<br/>9 vertical packs shipped]
    end

    subgraph M4[Claim 4 — Skills shared and learned]
        SKILL4[argo-skill<br/>library · marketplace · IR]
        LEARN4[argo-skill::per_worker_learning_cron]
        FORGE4[argo-forge<br/>self-improvement]
        EVAL4[argo-eval<br/>A/B + regression]
    end

    subgraph M5[Claim 5 — Real PC/Mac execution]
        CU5[argo-executor-computer-use<br/>+ -macos -linux -windows]
        MCP5[argo-mcp + 37 connectors]
        DOC5[argo-executor-document<br/>+ -csv -gsuite]
        FS5[argo-executor-filesystem]
    end

    subgraph M6[Claim 6 — Multi-channel approvals]
        CHANS6[argo-channels<br/>slack · email · sms · push · voice]
        ROUTES6[gateway/routes_channels_*<br/>+ webinbox]
        APPROVAL6[handlers/worker_actions_auto_approval]
    end

    subgraph FOUNDATION[Foundational cross-cutting]
        STORE[argo-store · 73 migrations · multi-tenant]
        AUDIT[argo-audit · hash-chained]
        CAP[argo-capability + argo-types::capability]
        PROV[argo-provider · transport · spend gate]
        AR[argo-agent-runner · turn loop]
        TEL[argo-telemetry · siem · billing]
    end

    M1 --> FOUNDATION
    M2 --> FOUNDATION
    M3 --> FOUNDATION
    M4 --> FOUNDATION
    M5 --> FOUNDATION
    M6 --> FOUNDATION

    classDef c1 fill:#fef3c7,stroke:#d97706
    classDef c2 fill:#dbeafe,stroke:#2563eb
    classDef c3 fill:#dcfce7,stroke:#16a34a
    classDef c4 fill:#f3e8ff,stroke:#9333ea
    classDef c5 fill:#fee2e2,stroke:#dc2626
    classDef c6 fill:#cffafe,stroke:#0891b2
    classDef found fill:#f5f5f4,stroke:#525252

    class M1 c1
    class M2 c2
    class M3 c3
    class M4 c4
    class M5 c5
    class M6 c6
    class FOUNDATION found
```

## Claim coverage snapshot (per 2026-05-16 codex audit)

| Claim | Verdict | Key gaps closed this weekend |
|---|---|---|
| 1 — Teammate | PARTIAL → mostly DELIVERED | F-03 memory tools now bound to worker/team namespace (#1737) |
| 2 — Delegation | PARTIAL → mostly DELIVERED | F-06 policy-as-code enforcement with real tenant/role/roster (#1742) |
| 3 — Platform builds workers | PARTIAL | Onboarding wizard (#1748) — first hire via plain English |
| 4 — Skills shared and learned | DELIVERED | F-04 learning cron now runtime-spawned from boot (#1736); F-11 trust-aware library selector (in flight) |
| 5 — Real PC/Mac execution | PARTIAL | F-01 macOS CU autonomous loop (#1722) shipped; F-10 smoke now real (#1733) |
| 6 — Multi-channel approvals | PARTIAL | F-05 fallback risk filter (#1738); F-07 voice channel wiring (in flight) |

## Source-of-truth pointers

- Mission claim verbatim: `docs/skycot/MISSION.md:26`
- Per-claim acceptance: `docs/skycot/V1_DEFINITION.md:41`
- Codex audit per-claim verdict: `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md` §"Coverage by mission claim"
