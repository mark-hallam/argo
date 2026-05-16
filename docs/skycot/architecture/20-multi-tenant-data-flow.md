# 20 — Multi-tenant data flow

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Update when:** new primitive added that crosses tenant boundaries, or `TenantId` plumbing changes.

How `TenantId` flows through every layer that touches per-tenant state. F-04 (#1727) closed the last nil-tenant gap last session.

## Diagram

```mermaid
flowchart TB
    REQ[Incoming HTTP / WS request]

    AUTH[JWT decode<br/>claims.tnt]

    SESSION_CTX[Session::open<br/>tenant_id pinned]

    subgraph PROPAGATE[TenantId propagation]
        TC[ToolContext.tenant_id<br/>F-04 #1727 closed nil gap]
        DC[build_dispatch_ctx tenant_id]
        MNS[MemoryNs { tenant, worker }<br/>F-03 #1737]
        SPEND[SpendGate<T>.tenant<br/>budget bucket]
        AUDIT_TENANT[AuditSink::new_with_tenant<br/>F-05b #1728]
    end

    subgraph STORE_LAYER[argo-store row-scoped queries]
        SQ[WHERE tenant_id = ?<br/>indexed]
        AUDIT_ROW[audit_record.tenant_id]
        MEMORY_ROW[memory_row.tenant_id]
        SKILL_ROW[skill.tenant_id]
        ACTION_ROW[worker_action.tenant_id]
        DUTY_ROW[duty.tenant_id]
    end

    subgraph MCP[MCP per-tenant isolation]
        POOL[McpClientPool<br/>key = tenant_id]
        SUBPROC[Per-tenant subprocesses<br/>+ env vars + creds]
    end

    subgraph CHANNEL[Per-tenant channels]
        WS_BRDC[ws_inbox_events<br/>tenant_key = claims.tnt<br/>F-03 #1720 — no more *]
        SLACK_TENANT[Slack webhook routed<br/>by tenant config]
    end

    subgraph POLICY[Per-tenant policy]
        POLICY_RESOLVE[resolve_effective_policy<br/>F-06 #1742]
        ROSTER[roster table<br/>tenant-scoped]
        ALLOW_LIST[capability allow_list<br/>per-tenant]
    end

    REQ --> AUTH
    AUTH --> SESSION_CTX
    SESSION_CTX --> TC
    TC --> DC
    DC --> MNS
    DC --> SPEND
    DC --> AUDIT_TENANT
    
    MNS --> MEMORY_ROW
    SPEND --> AUDIT_ROW
    AUDIT_TENANT --> AUDIT_ROW
    
    DC --> POOL
    POOL --> SUBPROC
    
    TC --> POLICY_RESOLVE
    POLICY_RESOLVE --> ROSTER
    POLICY_RESOLVE --> ALLOW_LIST
    
    REQ --> WS_BRDC
    SESSION_CTX --> SLACK_TENANT
    
    SQ --> AUDIT_ROW
    SQ --> MEMORY_ROW
    SQ --> SKILL_ROW
    SQ --> ACTION_ROW
    SQ --> DUTY_ROW

    classDef closed fill:#dcfce7,stroke:#16a34a
    classDef store fill:#dbeafe,stroke:#2563eb
    classDef mcp fill:#fef3c7,stroke:#d97706
    classDef chan fill:#cffafe,stroke:#0891b2
    classDef policy fill:#f3e8ff,stroke:#9333ea

    class TC,DC,MNS,AUDIT_TENANT,WS_BRDC closed
    class STORE_LAYER store
    class MCP mcp
    class CHANNEL chan
    class POLICY policy
```

## What changed this session

Five audit findings closed the multi-tenant gaps:

| Finding | Fix | PR |
|---|---|---|
| F-02 P0 | Migration slot collisions → monotonic renumber + validator | #1735 |
| F-03 P1 | Memory tools bound to MemoryNs (per-worker/team) | #1737 |
| F-04 P1 | `ToolContext.tenant_id` plumbed; nil dispatch_tenant() deleted | #1727 |
| F-05b P1 | `AuditSink::new` deprecated; production must use `new_with_tenant` | #1728 |
| F-03 P1 | WS inbox no longer broadcasts to `"*"` (global) — per-tenant only | #1720 |

## Key invariants

- **No nil tenant in production code.** `rg 'TenantId\(Uuid::nil\(\)\)|dispatch_tenant\(\)' crates/argo-agent-runner crates/argo-cli/src/handlers` returns 0 production hits (F-04 acceptance).
- **`AuditSink::new` is `#[deprecated]`.** CI `-D warnings` fails any production use. Tests opt-in via `#[allow(deprecated)]`. (F-05b acceptance.)
- **MCP pool keyed by tenant.** Different tenants never share subprocess state — credential isolation enforced.
- **Migration slots are strictly monotonic.** `duplicate_slot_validator` panics at startup if a duplicate is detected (post-F-02).

## Source-of-truth pointers

- ToolContext: `crates/argo-tool/src/tool.rs:53`
- dispatch_ctx: `crates/argo-agent-runner/src/dispatch.rs::build_dispatch_ctx`
- MemoryNs propagation: `crates/argo-agent-runner/src/turn_helpers.rs:154`
- AuditSink: `crates/argo-audit/src/sink.rs` + `sink_tenant.rs` (split per F-005a)
- WS inbox: `crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs::WsAuth`
- Migration slot validator: `crates/argo-store/src/migrations/duplicate_slot_validator.rs`
