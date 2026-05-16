# 13 — Skill loop (Claim 4: shared and learned)

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Update when:** new skill state added to `SkillStore`, or the layered-lookup priority order changes.

End-to-end loop: a worker learns a skill from repetition → operator approves → skill becomes available to other workers (Claim 4 of MISSION.md).

## Diagram

```mermaid
flowchart TB
    subgraph TEACH[Phase 1: Teach]
        ACTION[Worker runs an action repeatedly]
        TRACE[Trajectory captured<br/>session events + outcomes]
        RATING[skill_rating row written<br/>tenant + worker + action]
    end

    subgraph PROPOSE[Phase 2: Propose]
        CRON[per_worker_learning_cron<br/>spawned from gateway boot<br/>F-04 #1736]
        AGGREGATE[Aggregate ratings<br/>threshold check<br/>configurable interval]
        PROPOSAL[Pending overlay proposal<br/>skill_overlay table<br/>state=proposed]
    end

    subgraph REVIEW[Phase 3: Review]
        OPERATOR[Operator reviews]
        APPROVE_GATE[capability check<br/>SkillWrite required]
        APPROVED{Approve?}
        REJECT[state=rejected]
        APPROVED_OVERLAY[state=approved<br/>publish=true if marketplace]
    end

    subgraph PUBLISH[Phase 4: Publish]
        IR[Skill IR<br/>argo-skill::ir<br/>SkillGraph + nodes]
        TRUST[TrustGrade<br/>Untrusted → Verified → Trusted]
        MARKETPLACE[skill table<br/>shared=true<br/>tenant-private library]
        AB_TEST[Optional A/B test<br/>argo-skill::ab_test]
    end

    subgraph RELOAD[Phase 5: Reload]
        ANOTHER[Different worker, same vertical or related]
        LAYERED[LayeredLookup::with_library<br/>argo-skill::library]
        SELECTOR[SkillRelevanceSelector<br/>action-type + vertical +<br/>trust + tags + cap<br/>F-11 in flight]
        PROMPT[load_prompt_skills_with_library<br/>+ filter + cap @ 5 default]
        SYSTEM_PROMPT[skill rendered into<br/>system prompt of new turn]
    end

    subgraph ACT[Phase 6: Act]
        NEW_WORKER[New worker dispatches<br/>using inherited skill]
        SUCCESS[Outcome → new audit row]
        FORGE[argo-forge<br/>self-improvement signal]
    end

    ACTION --> TRACE
    TRACE --> RATING

    RATING --> CRON
    CRON --> AGGREGATE
    AGGREGATE --> PROPOSAL

    PROPOSAL --> OPERATOR
    OPERATOR --> APPROVE_GATE
    APPROVE_GATE --> APPROVED
    APPROVED -->|no| REJECT
    APPROVED -->|yes| APPROVED_OVERLAY

    APPROVED_OVERLAY --> IR
    IR --> TRUST
    TRUST --> MARKETPLACE
    MARKETPLACE --> AB_TEST

    MARKETPLACE --> ANOTHER
    ANOTHER --> LAYERED
    LAYERED --> SELECTOR
    SELECTOR --> PROMPT
    PROMPT --> SYSTEM_PROMPT

    SYSTEM_PROMPT --> NEW_WORKER
    NEW_WORKER --> SUCCESS
    SUCCESS --> FORGE
    FORGE -.->|signals next overlay proposal| CRON

    classDef teach fill:#fef3c7,stroke:#d97706
    classDef propose fill:#dbeafe,stroke:#2563eb
    classDef review fill:#dcfce7,stroke:#16a34a
    classDef publish fill:#f3e8ff,stroke:#9333ea
    classDef reload fill:#cffafe,stroke:#0891b2
    classDef act fill:#fee2e2,stroke:#dc2626

    class TEACH teach
    class PROPOSE propose
    class REVIEW review
    class PUBLISH publish
    class RELOAD reload
    class ACT act
```

## State machine for a single skill

```mermaid
stateDiagram-v2
    [*] --> draft: teach phase
    draft --> proposed: cron aggregate threshold
    proposed --> approved: operator approves
    proposed --> rejected: operator rejects
    approved --> published: shared=true (marketplace)
    approved --> private: tenant-scoped only
    published --> deprecated: superseded by new version
    private --> deprecated: same
    rejected --> [*]
    deprecated --> [*]
```

## Key invariants

- **Cross-tenant isolation.** Marketplace skills carry provenance; private library skills (`shared=false`) cannot leak across tenants. Enforced in `LayeredLookup::with_library` filter — verified by `end_to_end_claim4_loop.rs` test.
- **Trust grading.** `TrustGrade::{Untrusted, Verified, Trusted}` gates which skills make it into prompts under tenant policy. Default minimum: `Verified`.
- **Prompt budget cap.** Max 5 marketplace skills per turn (configurable). Prevents prompt pollution at scale.
- **Per-worker learning cron is now ACTUALLY runtime-spawned** (F-04 #1736 — pre-shipping it was a substrate gap; now lives in gateway boot).

## Source-of-truth pointers

- E2E proof: `crates/argo-skill/tests/end_to_end_claim4_loop.rs`
- Layered lookup: `crates/argo-skill/src/library.rs::LayeredLookup::with_library` (84)
- Learning cron: `crates/argo-skill/src/per_worker_learning_cron.rs::spawn_per_worker_learning_cron`
- Skill IR: `crates/argo-skill/src/ir/`
- Relevance selector: `crates/argo-skill/src/relevance_selector.rs` (F-11 in flight)
- Forge feedback: `crates/argo-forge/`
