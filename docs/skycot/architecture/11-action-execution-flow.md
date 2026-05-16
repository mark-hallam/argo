# 11 — Action execution flow

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Update when:** new `ExecutorKind` variant added, or the resolver in `worker_actions_executor.rs` is restructured.

How a vertical manifest's named action becomes real execution: resolver picks an executor, executor runs, outcome is recorded.

## Diagram

```mermaid
flowchart LR
    MANIFEST[Vertical manifest<br/>action.executor.kind]

    RESOLVE[worker_actions_executor::<br/>resolve_executor]

    KIND{ExecutorKind}

    subgraph EXECS[Executors]
        CU_MAC[ComputerUseMacOS<br/>argo-executor-computer-use-macos<br/>autonomous run loop]
        CU_LX[ComputerUseLinux<br/>argo-executor-computer-use-linux<br/>DEFERRED-v1.x]
        CU_WIN[ComputerUseWindows<br/>argo-executor-computer-use-windows<br/>DEFERRED-v1.x]
        CU_E2B[ComputerUse<br/>argo-executor-computer-use<br/>E2B sandbox]
        MCP_EXEC[MCP<br/>argo-mcp-client<br/>via pool]
        API_EXEC[API<br/>http call]
        BROWSER[Browser<br/>headless]
        CODE[CodeExecution<br/>argo-executor-code<br/>Local fail-closed in prod]
        DOC[Document<br/>argo-executor-document<br/>+ csv + gsuite]
        FS[Filesystem<br/>argo-executor-filesystem]
        STUB[Schema-only stubs<br/>error: not wired]
    end

    OUTCOME[ActionOutcome]
    O_COMPLETED[Completed<br/>+ bundle path]
    O_HANDOFF[HumanHandoff<br/>+ bundle path<br/>preflight failed]
    O_FAILED[Failed<br/>+ error]
    O_APPROVAL[NeedsApproval<br/>routed via ask_operator]

    BUNDLE[Per-action bundle on disk<br/>.skycot/outputs/]
    AUDIT_ROW[AuditSink::write_with_tenant<br/>ActionExecuted row<br/>+ chained SHA]

    MANIFEST --> RESOLVE
    RESOLVE --> KIND
    KIND --> CU_MAC
    KIND --> CU_LX
    KIND --> CU_WIN
    KIND --> CU_E2B
    KIND --> MCP_EXEC
    KIND --> API_EXEC
    KIND --> BROWSER
    KIND --> CODE
    KIND --> DOC
    KIND --> FS
    KIND -.->|kind not recognised| STUB

    CU_MAC --> OUTCOME
    CU_LX --> OUTCOME
    CU_WIN --> OUTCOME
    CU_E2B --> OUTCOME
    MCP_EXEC --> OUTCOME
    API_EXEC --> OUTCOME
    BROWSER --> OUTCOME
    CODE --> OUTCOME
    DOC --> OUTCOME
    FS --> OUTCOME
    STUB --> OUTCOME

    OUTCOME --> O_COMPLETED
    OUTCOME --> O_HANDOFF
    OUTCOME --> O_FAILED
    OUTCOME --> O_APPROVAL

    O_COMPLETED --> BUNDLE
    O_HANDOFF --> BUNDLE
    BUNDLE --> AUDIT_ROW
    O_FAILED --> AUDIT_ROW
    O_APPROVAL --> AUDIT_ROW

    classDef macos fill:#dcfce7,stroke:#16a34a
    classDef deferred fill:#f5f5f4,stroke:#a3a3a3,stroke-dasharray:4 4
    classDef shipped fill:#dbeafe,stroke:#2563eb
    classDef outcome fill:#fef3c7,stroke:#d97706
    classDef stub fill:#fee2e2,stroke:#dc2626

    class CU_MAC macos
    class CU_LX,CU_WIN deferred
    class CU_E2B,MCP_EXEC,API_EXEC,BROWSER,CODE,DOC,FS shipped
    class STUB stub
    class O_COMPLETED,O_HANDOFF,O_FAILED,O_APPROVAL outcome
```

## Key invariants

- **macOS-first for v1 demo.** `ComputerUseMacOS` returns `Completed` on `stop_success` (F-001b shipped #1722). `HumanHandoff` only on preflight failure (missing macOS Accessibility/Screen Recording permission).
- **Code execution fail-closed in production.** `Local` backend rejected when tenant policy says `allow_local_code_execution = false` (F-12 in flight, brief `codex-code-executor-sandbox-failclose`).
- **Every action emits an audit row.** Even failed dispatches. Chain stays continuous.

## Source-of-truth pointers

- Resolver: `crates/argo-cli/src/handlers/worker_actions_executor.rs::resolve_executor`
- macOS executor: `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs` + `crates/argo-executor-computer-use-macos/`
- Real macOS smoke: `crates/argo-cli/tests/cu_macos_dispatch_smoke.rs` (post-F-10, no more `argo --help`)
- Bundle layout: `.skycot/outputs/<job_id>/<action_id>/`
