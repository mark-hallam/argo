# 22 — Capability + RBAC

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Update when:** `Capability` enum gains/loses variants, or default-role assignments change.

How SKYCOT enforces "this principal can do this thing" — capability declarations on tools/actions matched against role grants.

## Diagram

```mermaid
flowchart TB
    PRINCIPAL[Principal<br/>argo-types::capability::Principal<br/>Worker/Human/System/Anonymous]
    
    ROLE[Role<br/>CapabilityRole<br/>inherits_from + capabilities]
    
    subgraph DEFAULT_ROLES[Default roles<br/>argo-capability::default_roles_for_workspace]
        OWNER[owner<br/>full MessageSend<br/>+ all admin caps]
        ADMIN[admin<br/>full MessageSend<br/>+ subset admin]
        USER[user<br/>slack/email MessageSend only<br/>FsRead/Write workspace-scoped<br/>F-06 fixed #1723]
        GUEST[guest<br/>minimal read]
    end
    
    subgraph CAPS[Capability variants]
        FS_R[FsRead path_glob<br/>workspace-scoped post-F-06]
        FS_W[FsWrite path_glob]
        TOOL_D[ToolDispatch tool_name_glob]
        MSG_S[MessageSend channel, recipient_scope<br/>F-07 added #1729]
        SPAWN[Spawn]
        MEM_W[MemoryWrite]
        SKILL_W[SkillWrite]
        AUDIT_W[AuditWrite]
        SECRET_R[SecretRead name]
        NET[NetEgress host_glob, port]
        LOCAL_CODE[LocalCodeExecution<br/>F-12 in flight]
    end
    
    GATE[Capability gate<br/>argo-capability::CapabilityEngine<br/>has + has_glob]
    
    TOOL_CALL[Tool dispatch attempt<br/>tool.required_capabilities args]
    ACTION_CALL[Action execution<br/>manifest.required_capabilities]
    
    DECISION{Match?}
    ALLOW[Allow → execute<br/>audit row emitted]
    DENY[Deny → 403<br/>+ audit ApprovalAsked OR Failed]
    
    PRINCIPAL --> ROLE
    ROLE --> DEFAULT_ROLES
    OWNER --> CAPS
    ADMIN --> CAPS
    USER --> CAPS
    GUEST --> CAPS
    
    TOOL_CALL --> GATE
    ACTION_CALL --> GATE
    GATE --> CAPS
    GATE --> DECISION
    DECISION -->|allow| ALLOW
    DECISION -->|deny| DENY

    classDef principal fill:#fef3c7,stroke:#d97706
    classDef role fill:#dbeafe,stroke:#2563eb
    classDef cap fill:#dcfce7,stroke:#16a34a
    classDef gate fill:#f3e8ff,stroke:#9333ea
    classDef allow fill:#d1fae5,stroke:#059669
    classDef deny fill:#fee2e2,stroke:#dc2626

    class PRINCIPAL principal
    class DEFAULT_ROLES,OWNER,ADMIN,USER,GUEST role
    class CAPS,FS_R,FS_W,TOOL_D,MSG_S,SPAWN,MEM_W,SKILL_W,AUDIT_W,SECRET_R,NET,LOCAL_CODE cap
    class GATE,DECISION gate
    class ALLOW allow
    class DENY deny
```

## Capability lifecycle

```mermaid
sequenceDiagram
    participant T as Tool / Executor
    participant E as CapabilityEngine
    participant R as Role registry
    participant P as Principal session
    participant A as AuditSink

    T->>E: tool.required_capabilities(args)
    Note over T,E: Returns CapabilitySet<br/>e.g. [ToolDispatch{send_message},<br/>      MessageSend{slack, "*"}]
    
    E->>P: get principal's role
    P-->>E: role = "user"
    E->>R: lookup role
    R-->>E: Role { capabilities: [...] }
    
    E->>E: each required cap → match against role caps
    Note over E: glob matching:<br/>"slack" matches "slack"<br/>"sms" doesn't match "slack/email"<br/>(F-07 wired this #1729)
    
    alt All required caps granted
        E-->>T: ✅ Allow
        T->>A: audit row: ToolDispatch (allowed)
    else Missing capability
        E-->>T: ❌ Deny + reason
        T->>A: audit row: ToolDispatch (denied) OR ApprovalAsked
    end
```

## Key invariants

- **Workspace-scoped fs defaults** — F-06 (#1723) replaced the permissive `"**"` placeholder with `{workspace_root}/**` interpolation. Default `user` role can only read/write inside its workspace.
- **Per-channel send capability** — F-07 (#1729) added `Capability::MessageSend{channel, recipient_scope}`. `ToolDispatch{send_message}` alone is no longer enough; high-risk channels need explicit grant.
- **No `Capability::LocalCodeExecution` in default roles** — F-12 (in flight) ensures Local sandbox backend fails closed unless tenant explicitly grants.
- **Glob matching is conservative** — `FsRead{path_glob: "/workspace/**"}` does NOT match `/etc/passwd`.

## Source-of-truth pointers

- Capability enum: `crates/argo-types/src/capability.rs::Capability`
- Engine: `crates/argo-capability/src/lib.rs::CapabilityEngine`
- Default roles: `crates/argo-capability/src/lib.rs::default_roles_for_workspace`
- Send-message gate: `crates/argo-tool/src/tools/send_message.rs::required_capabilities`
- Auto-approval gate: `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs`
