# 23 — Policy-as-code resolution

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Update when:** policy layer order changes, or new effective-policy field added.

Auto-approval policy is layered — action-specific → worker → roster → workspace → defaults — and resolves to one `EffectivePolicy` per request. F-06 (#1742) made this real (was previously paper-only with empty role/tenant strings).

## Diagram

```mermaid
flowchart TB
    REQ[Action dispatch attempt<br/>worker_actions_auto_approval]
    
    CTX[Resolution context<br/>tenant + role + worker + roster]
    
    L1[Layer 1: Action-specific rules<br/>vertical manifest action.gates]
    L2[Layer 2: Worker policy<br/>worker.toml]
    L3[Layer 3: Roster policy<br/>roster table<br/>team-level inheritance]
    L4[Layer 4: Workspace policy<br/>workspace.toml]
    L5[Layer 5: Workspace defaults<br/>argo-config defaults]
    
    MERGE[resolve_effective_policy<br/>argo-identity::policy<br/>walks 1→5, first-match wins]
    
    EFFECTIVE[EffectivePolicy<br/>auto_approval<br/>quiet_hours<br/>require_signatures<br/>audit_chain_required<br/>budget caps]
    
    GATE_CHECK{Gate check}
    
    QH{quiet_hours<br/>in window?}
    SIGS{require_signatures<br/>N-of-M met?}
    AC{audit_chain_required<br/>chain verified?}
    BUDGET{within budget +<br/>auto_approval=true?}
    
    AUTO_APPROVE[✅ Auto-approve<br/>audit row emitted]
    ASK[Ask operator<br/>see 12-approval-flow]
    FAIL_CLOSED[❌ Fail closed<br/>policy violated]
    
    REQ --> CTX
    CTX --> L1
    L1 --> L2
    L2 --> L3
    L3 --> L4
    L4 --> L5
    L5 --> MERGE
    MERGE --> EFFECTIVE
    EFFECTIVE --> GATE_CHECK
    
    GATE_CHECK --> QH
    QH -->|outside| FAIL_CLOSED
    QH -->|inside| SIGS
    
    SIGS -->|missing| FAIL_CLOSED
    SIGS -->|met| AC
    
    AC -->|unwired| FAIL_CLOSED
    AC -->|verified| BUDGET
    
    BUDGET -->|under + allowed| AUTO_APPROVE
    BUDGET -->|over OR auto_approval=false| ASK

    classDef layer fill:#dbeafe,stroke:#2563eb
    classDef resolve fill:#fef3c7,stroke:#d97706
    classDef gate fill:#f3e8ff,stroke:#9333ea
    classDef approve fill:#dcfce7,stroke:#16a34a
    classDef ask fill:#fed7aa,stroke:#ea580c
    classDef fail fill:#fee2e2,stroke:#dc2626

    class L1,L2,L3,L4,L5 layer
    class MERGE,EFFECTIVE resolve
    class GATE_CHECK,QH,SIGS,AC,BUDGET gate
    class AUTO_APPROVE approve
    class ASK ask
    class FAIL_CLOSED fail
```

## EffectivePolicy fields (enforced post F-06)

| Field | Type | Effect when set |
|---|---|---|
| `auto_approval` | bool | If true AND within budget+time → auto-approve |
| `quiet_hours` | `TimeWindow` | Outside window → fail closed (no auto-approve) |
| `require_signatures` | `{ n: 2, of: 3 }` | If non-zero → must have N approvers; below = fail closed |
| `audit_chain_required` | bool | If true AND audit chain not verifiable → fail closed |
| `budget` | `{ max_per_action, max_per_day, currency }` | Numeric gate |
| `allowed_channels` | `Vec<String>` | Restricts which channels can fire approval asks |
| `prohibited_actions` | `Vec<String>` | Manifest action names that are always refused |

## Pre-F-06 vs post-F-06

**Before F-06 (paper-only):**
- `resolve_effective_policy("", "", worker_id, None)` — empty role + tenant
- Roster never consulted (always None)
- `quiet_hours` / `require_signatures` / `audit_chain_required` parsed but never gate-checked
- V1 success scenario #4 ("auto-approve weekday business hours") was inert

**After F-06 (#1742):**
- Real tenant_id (from session), real role (from worker.role_name), real roster (from `Worker::resolved_policy_with_rosters`)
- All effective fields enforced fail-closed
- Test coverage: 5 regression tests in `worker_actions_auto_approval_tests.rs`

## Key invariants

- **Fail closed on any unsupported field.** If a field is set but the gate doesn't know how to enforce, action is denied (escalate to manual).
- **Quiet hours respect tenant timezone.** Resolved from `workspace.timezone` or operator config.
- **N-of-M signatures persist.** Approval responses are signed (HMAC) — F-07 channel response adds explicit signature row to audit.

## Source-of-truth pointers

- Policy module: `crates/argo-identity/src/policy.rs::resolve_effective_policy` (`:17`)
- Worker policy resolver: `crates/argo-identity/src/roster.rs::Worker::resolved_policy_with_rosters`
- Approval gate: `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` (post F-06)
- Regression tests: `crates/argo-cli/src/handlers/worker_actions_auto_approval_tests.rs`
