//! WS1 tenant + vertical isolation regression pins for the in-core
//! `memory` tool, in the agent-runner tool path.
//!
//! Brief: `fix-agent-runtime-memory-tool-tenant-and-vertical-scope`.
//!
//! Live e2e finding (2026-06-10): a freshly-signed-up account's
//! `customer-support` teammate read a `completed_jobs` memory record
//! belonging to a different role from a different prior session (a
//! `gtm-engineer` draft). Two layered defects:
//!
//! - **3a** — `build_turn_dispatch_ctx` bound the memory tool to
//!   `store.first_workspace()` (the globally-oldest demo/cwd workspace),
//!   ignoring `session.tenant_id`. In multi-tenant that is a cross-TENANT
//!   read+write. The fix binds via `first_workspace_for_tenant`.
//! - **3b** — `list`/`read`/supersede used `list_active` (all verticals),
//!   ignoring the bound vertical. The fix routes them through
//!   `list_for_vertical` when a vertical is bound.
//!
//! These pins exercise the production tool constructor
//! (`builtin_tools_with_runtime`) — the same path `build_dispatch_ctx`
//! consumes — so they cover the agent-runner gap the CLI-layer
//! `cli_memory_list_with_vertical_excludes_other_verticals_rows` test did
//! not. Both FAIL on the pre-fix behaviour (global-workspace binding /
//! `list_active` reads).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use argo_memory::{MemoryFile, MemoryStore};
use argo_orchestrator::OrchestratorStore;
use argo_store::SessionStore;
use argo_tool::{builtin_tools_with_runtime, Tool, ToolContext};
use argo_types::{ContentBlock, Principal, TenantId, ToolCallResult, WorkspaceId};

fn memory_tool(
    memory: &Arc<MemoryStore>,
    orchestrator: &Arc<OrchestratorStore>,
    session_store: &Arc<SessionStore>,
    workspace: WorkspaceId,
    vertical: Option<String>,
) -> Arc<dyn Tool> {
    builtin_tools_with_runtime(
        PathBuf::from("/tmp/argo-memory-tenant-vertical-isolation"),
        Some((Arc::clone(memory), workspace, vertical)),
        None,
        Arc::clone(orchestrator),
        Arc::clone(session_store),
    )
    .into_iter()
    .find(|t| t.name() == "memory")
    .expect("memory tool registered")
}

fn ctx() -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "isolation-it".into(),
        },
        PathBuf::from("/tmp/argo-memory-tenant-vertical-isolation"),
    )
}

fn text_body(result: &ToolCallResult) -> String {
    result
        .content
        .iter()
        .find_map(|b| match b {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// 3a setup: two tenants (A created first → globally-oldest workspace, the
/// demo/cwd analogue), tenant A seeded with a sensitive memory row, plus the
/// constructed memory/orchestrator stores. Distinct creation order (1100ms
/// gap) makes `ORDER BY created_at ASC` unambiguous so the pre-fix global
/// binding deterministically resolves to ws_a.
async fn setup_two_tenants() -> (
    Arc<SessionStore>,
    Arc<MemoryStore>,
    Arc<OrchestratorStore>,
    WorkspaceId,
    TenantId,
    WorkspaceId,
) {
    let session_store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    session_store.migrate().await.expect("migrate");

    // Tenant A first → globally-oldest workspace (the demo/cwd analogue).
    let (_tenant_a, ws_a) = session_store
        .ensure_workspace("tenant-a", "/tmp/argo-iso-a")
        .await
        .expect("ensure tenant A");
    // Distinct second so ORDER BY created_at ASC is unambiguous and the
    // pre-fix global binding deterministically resolves to ws_a.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let (tenant_b, ws_b) = session_store
        .ensure_workspace("tenant-b", "/tmp/argo-iso-b")
        .await
        .expect("ensure tenant B");
    assert_ne!(ws_a, ws_b, "two tenants must have distinct workspaces");

    let memory = Arc::new(MemoryStore::new(Arc::clone(&session_store)));
    let orchestrator = Arc::new(OrchestratorStore::new(Arc::clone(&session_store)));

    // Seed tenant A's workspace with a sensitive memory row.
    memory
        .append(
            ws_a,
            MemoryFile::Memory,
            "TENANT-A-SECRET completed_jobs".to_string(),
            None,
        )
        .await
        .expect("seed tenant A memory");

    (session_store, memory, orchestrator, ws_a, tenant_b, ws_b)
}

/// 3b setup: one workspace seeded with a `gtm-engineer` MEMORY.md row (the
/// prior role's data the leak surfaced), plus the constructed stores. The
/// cross-vertical pin asserts a `customer-support`-bound tool neither reads
/// nor supersedes it.
async fn setup_seeded_vertical_workspace() -> (
    Arc<SessionStore>,
    Arc<MemoryStore>,
    Arc<OrchestratorStore>,
    WorkspaceId,
) {
    let session_store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    session_store.migrate().await.expect("migrate");
    let (_tenant, ws) = session_store
        .ensure_workspace("vert-iso", "/tmp/argo-iso-vert")
        .await
        .expect("ensure_workspace");
    let memory = Arc::new(MemoryStore::new(Arc::clone(&session_store)));
    let orchestrator = Arc::new(OrchestratorStore::new(Arc::clone(&session_store)));

    // Seed a gtm-engineer MEMORY.md row (the prior role's data).
    memory
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            "GTM-OUTREACH-DRAFT".to_string(),
            None,
            "gtm-engineer",
        )
        .await
        .expect("seed gtm-engineer memory");

    (session_store, memory, orchestrator, ws)
}

// ───────────────── 3a: cross-tenant binding ─────────────────

/// A second tenant's worker run must bind to its OWN (empty) workspace,
/// not the globally-oldest demo/cwd one. Tenant A's workspace is created
/// first (so it is the global-oldest) and seeded with a memory row; the
/// buggy `first_workspace()` binding would point tenant B's tool at it.
#[tokio::test]
async fn cross_tenant_run_does_not_read_other_tenants_memory() {
    let (session_store, memory, orchestrator, ws_a, tenant_b, ws_b) =
        setup_two_tenants().await;

    // Regression pin: the buggy binding used `first_workspace()` (global
    // oldest = ws_a) for every session; the fix uses the tenant-scoped one.
    let global_oldest = session_store
        .first_workspace()
        .await
        .expect("first_workspace")
        .expect("some workspace");
    assert_eq!(
        global_oldest, ws_a,
        "global-oldest must be tenant A's ws — the leak the old binding hit"
    );
    let tenant_b_resolved = session_store
        .first_workspace_for_tenant(tenant_b)
        .await
        .expect("first_workspace_for_tenant")
        .expect("tenant B has its own workspace");
    assert_eq!(
        tenant_b_resolved, ws_b,
        "tenant B must resolve to its OWN workspace, not the global oldest"
    );

    // Tenant B's tool, bound via the FIXED resolution, sees nothing.
    let tool_b = memory_tool(
        &memory,
        &orchestrator,
        &session_store,
        tenant_b_resolved,
        None,
    );
    let list = tool_b
        .dispatch(serde_json::json!({"action": "list"}), &ctx())
        .await
        .expect("list dispatch");
    assert!(
        text_body(&list).contains("no memory entries"),
        "tenant B must see an empty workspace, got: {}",
        text_body(&list)
    );
    let read = tool_b
        .dispatch(
            serde_json::json!({"action": "read", "entry": "MEMORY.md"}),
            &ctx(),
        )
        .await
        .expect_err("tenant B reading MEMORY.md must error — no entries");
    assert!(read.to_string().contains("no entries"), "got: {read}");

    // Sanity: tenant A's own workspace still holds the seeded row, so the
    // isolation above is meaningful (not just an empty database).
    let tool_a = memory_tool(&memory, &orchestrator, &session_store, ws_a, None);
    let read_a = tool_a
        .dispatch(
            serde_json::json!({"action": "read", "entry": "MEMORY.md"}),
            &ctx(),
        )
        .await
        .expect("tenant A read");
    assert!(
        text_body(&read_a).contains("TENANT-A-SECRET"),
        "tenant A must still read its own row, got: {}",
        text_body(&read_a)
    );
}

// ───────────────── 3b: cross-vertical reads + supersede ─────────────────

/// Within one workspace, a `customer-support`-bound memory tool must not
/// read a `gtm-engineer` MEMORY.md row (the observed leak), and writing
/// `customer-support`'s own MEMORY.md must NOT supersede the gtm row.
#[tokio::test]
async fn cross_vertical_read_and_write_are_isolated_in_agent_runner_path() {
    let (session_store, memory, orchestrator, ws) = setup_seeded_vertical_workspace().await;

    // customer-support-bound tool: list + read must not surface gtm's row.
    let cs_tool = memory_tool(
        &memory,
        &orchestrator,
        &session_store,
        ws,
        Some("customer-support".to_string()),
    );
    let list = cs_tool
        .dispatch(serde_json::json!({"action": "list"}), &ctx())
        .await
        .expect("list dispatch");
    assert!(
        text_body(&list).contains("no memory entries"),
        "customer-support must not see gtm-engineer rows, got: {}",
        text_body(&list)
    );
    let read_err = cs_tool
        .dispatch(
            serde_json::json!({"action": "read", "entry": "MEMORY.md"}),
            &ctx(),
        )
        .await
        .expect_err("customer-support read must error — no entries for its vertical");
    assert!(
        read_err.to_string().contains("no entries"),
        "got: {read_err}"
    );

    // customer-support writes its OWN MEMORY.md — must CREATE (append for its
    // vertical), NOT supersede gtm-engineer's same-named row.
    cs_tool
        .dispatch(
            serde_json::json!({"action": "write", "entry": "MEMORY.md", "content": "CS-NOTE"}),
            &ctx(),
        )
        .await
        .expect("customer-support write");

    // gtm-engineer's row is intact, not superseded.
    let gtm_rows = memory
        .list_for_vertical(ws, MemoryFile::Memory, "gtm-engineer")
        .await
        .expect("list gtm-engineer");
    assert_eq!(
        gtm_rows.len(),
        1,
        "gtm-engineer row must survive the cs write"
    );
    assert_eq!(gtm_rows[0].body, "GTM-OUTREACH-DRAFT");
    assert!(
        gtm_rows[0].superseded_by.is_none(),
        "gtm-engineer row must NOT be superseded by the customer-support write"
    );

    // customer-support now sees its own row, and only its own.
    let cs_rows = memory
        .list_for_vertical(ws, MemoryFile::Memory, "customer-support")
        .await
        .expect("list customer-support");
    assert_eq!(cs_rows.len(), 1);
    assert_eq!(cs_rows[0].body, "CS-NOTE");
}
