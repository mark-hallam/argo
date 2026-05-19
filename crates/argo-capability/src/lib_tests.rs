//! Tests for [`super::CapabilityEngine`] — extracted from `lib.rs` to
//! keep the production-code file under the 500-LOC ceiling.
#![cfg(test)]
use super::*;
use argo_audit::{AuditKind, AuditSink, SYSTEM_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use argo_types::SpawnKind;
use serde_json::Value;
use std::{sync::Arc, time::Duration};

fn local(user: &str) -> Principal {
    Principal::Local {
        os_user: user.into(),
    }
}

async fn audit_fixture() -> (Arc<SessionStore>, Arc<AuditSink>) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let sink = Arc::new(AuditSink::new_with_tenant(
        Arc::clone(&store),
        SYSTEM_AUDIT_TENANT_ID.to_string(),
    ));
    (store, sink)
}

async fn audit_rows(store: &SessionStore) -> Vec<(String, Value)> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT kind, payload_json FROM audit_record ORDER BY seq",
    )
    .fetch_all(store.pool())
    .await
    .unwrap()
    .into_iter()
    .map(|(kind, payload)| (kind, serde_json::from_str(&payload).unwrap()))
    .collect()
}

async fn wait_for_rows(
    sink: &AuditSink,
    store: &SessionStore,
    expected: usize,
) -> Vec<(String, Value)> {
    for _ in 0..50 {
        sink.flush_pending().await.unwrap();
        let rows = audit_rows(store).await;
        if rows.len() >= expected {
            return rows;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let rows = audit_rows(store).await;
    panic!("expected at least {expected} audit rows, got {rows:?}");
}

#[test]
fn defaults_grant_owner_full_dispatch() {
    let e = CapabilityEngine::with_defaults();
    let alice = local("alice");
    e.bind(alice.clone(), "owner".into()).unwrap();
    assert!(e
        .check(
            &alice,
            &Capability::ToolDispatch {
                tool_name_glob: "anything".into(),
            },
        )
        .is_ok());
}

#[test]
fn workspace_defaults_scope_user_filesystem_caps() {
    let roles = default_roles_for_workspace(std::path::Path::new("/tmp/ws"));
    let user = roles
        .iter()
        .find(|role| role.name == "user")
        .expect("user role exists");
    let globs: Vec<&str> = user
        .capabilities
        .0
        .iter()
        .filter_map(|cap| match cap {
            Capability::FsRead { path_glob } | Capability::FsWrite { path_glob } => {
                Some(path_glob.as_str())
            }
            _ => None,
        })
        .collect();

    assert_eq!(globs, vec!["/tmp/ws/**", "/tmp/ws/**"]);
}

#[test]
fn guest_cannot_terminal() {
    let e = CapabilityEngine::with_defaults();
    let g = local("guest1");
    e.bind(g.clone(), "guest".into()).unwrap();
    let err = e
        .check(
            &g,
            &Capability::ToolDispatch {
                tool_name_glob: "terminal".into(),
            },
        )
        .unwrap_err();
    match err {
        ArgoError::RbacDenied { role, .. } => assert_eq!(role, "guest"),
        other => panic!("expected RbacDenied; got {other:?}"),
    }
}

#[test]
fn unbound_principal_is_capability_denied_not_rbac_denied() {
    let e = CapabilityEngine::with_defaults();
    let stranger = local("noone");
    let err = e
        .check(
            &stranger,
            &Capability::ToolDispatch {
                tool_name_glob: "web_search".into(),
            },
        )
        .unwrap_err();
    assert!(matches!(err, ArgoError::CapabilityDenied(_)));
}

#[test]
fn dynamic_grant_widens() {
    let e = CapabilityEngine::with_defaults();
    let g = local("g");
    e.bind(g.clone(), "guest".into()).unwrap();
    let want = Capability::ToolDispatch {
        tool_name_glob: "terminal".into(),
    };
    assert!(e.check(&g, &want).is_err());
    e.grant_dynamic(g.clone(), want.clone());
    assert!(e.check(&g, &want).is_ok());
}

#[tokio::test]
async fn bind_without_sink_leaves_audit_chain_empty() {
    let (store, sink) = audit_fixture().await;
    let e = CapabilityEngine::with_defaults();
    let alice = local("alice");
    e.bind(alice, "owner".into()).unwrap();

    tokio::time::sleep(Duration::from_millis(20)).await;
    sink.flush_pending().await.unwrap();
    assert!(audit_rows(&store).await.is_empty());
}

#[tokio::test]
async fn capability_audit_rows_bracket_dispatch_outcomes() {
    let (store, sink) = audit_fixture().await;
    let e = CapabilityEngine::with_defaults().with_audit_sink(Arc::clone(&sink));
    let principal = local("audited");
    let requested = Capability::ToolDispatch {
        tool_name_glob: "web_search".into(),
    };

    e.bind_with_actor(principal.clone(), "guest".into(), Some("alice@acme"))
        .unwrap();
    let rows = wait_for_rows(&sink, &store, 1).await;
    assert_eq!(rows[0].0, AuditKind::RoleBindingChanged.label());
    assert_eq!(rows[0].1["role_or_capability"], "guest");
    assert_eq!(rows[0].1["actor"], "alice@acme");
    assert_eq!(rows[0].1["action"], "bind");
    assert!(rows[0].1["at"].as_u64().is_some());

    e.check(&principal, &requested).unwrap();
    sink.write(
        None,
        AuditKind::ToolCall,
        serde_json::json!({"tool": "web_search"}),
        None,
    )
    .await
    .unwrap();
    sink.flush_pending().await.unwrap();

    e.unbind(&principal, Some("alice@acme"));
    let rows = wait_for_rows(&sink, &store, 3).await;
    assert_eq!(rows[2].0, AuditKind::RoleBindingChanged.label());
    assert_eq!(rows[2].1["role_or_capability"], "guest");
    assert_eq!(rows[2].1["action"], "unbind");

    assert!(e.check(&principal, &requested).is_err());
    sink.write(
        None,
        AuditKind::RbacDeny,
        serde_json::json!({"tool": "web_search"}),
        None,
    )
    .await
    .unwrap();
    let rows = wait_for_rows(&sink, &store, 4).await;
    let kinds: Vec<&str> = rows.iter().map(|(kind, _)| kind.as_str()).collect();
    assert_eq!(
        kinds,
        vec![
            "role_binding_changed",
            "tool_call",
            "role_binding_changed",
            "rbac.deny",
        ]
    );
}

#[tokio::test]
async fn dynamic_grant_and_revoke_emit_audit_rows() {
    let (store, sink) = audit_fixture().await;
    let e = CapabilityEngine::with_defaults().with_audit_sink(Arc::clone(&sink));
    let principal = local("dynamic");
    let cap = Capability::ToolDispatch {
        tool_name_glob: "terminal".into(),
    };

    e.grant_dynamic_with_actor(principal.clone(), cap.clone(), Some("approver"));
    assert!(e.check(&principal, &cap).is_ok());
    assert!(e.revoke_dynamic(&principal, &cap, Some("approver")));
    assert!(e.check(&principal, &cap).is_err());

    let rows = wait_for_rows(&sink, &store, 2).await;
    assert_eq!(rows[0].0, AuditKind::DynamicCapabilityGranted.label());
    assert_eq!(
        rows[0].1["role_or_capability"]["tool_name_glob"],
        "terminal"
    );
    assert_eq!(rows[0].1["actor"], "approver");
    assert_eq!(rows[0].1["action"], "grant");
    assert_eq!(rows[1].0, AuditKind::DynamicCapabilityGranted.label());
    assert_eq!(
        rows[1].1["role_or_capability"]["tool_name_glob"],
        "terminal"
    );
    assert_eq!(rows[1].1["action"], "revoke");
}

#[test]
fn role_inheritance_walks_parents() {
    let e = CapabilityEngine::new();
    e.add_role(CapabilityRole {
        name: "base".into(),
        inherits_from: None,
        capabilities: CapabilitySet(vec![Capability::ToolDispatch {
            tool_name_glob: "memory".into(),
        }]),
    })
    .unwrap();
    e.add_role(CapabilityRole {
        name: "extended".into(),
        inherits_from: Some("base".into()),
        capabilities: CapabilitySet(vec![Capability::ToolDispatch {
            tool_name_glob: "skill_view".into(),
        }]),
    })
    .unwrap();
    let p = local("p");
    e.bind(p.clone(), "extended".into()).unwrap();
    assert!(e
        .check(
            &p,
            &Capability::ToolDispatch {
                tool_name_glob: "memory".into(),
            },
        )
        .is_ok());
    assert!(e
        .check(
            &p,
            &Capability::ToolDispatch {
                tool_name_glob: "skill_view".into(),
            },
        )
        .is_ok());
}

#[test]
fn cycle_detection_rejects_two_node_cycle() {
    let e = CapabilityEngine::new();
    e.add_role(CapabilityRole {
        name: "a".into(),
        inherits_from: None,
        capabilities: CapabilitySet::empty(),
    })
    .unwrap();
    e.add_role(CapabilityRole {
        name: "b".into(),
        inherits_from: Some("a".into()),
        capabilities: CapabilitySet::empty(),
    })
    .unwrap();
    // Now try to redefine 'a' to inherit from 'b' → cycle.
    let res = e.add_role(CapabilityRole {
        name: "a".into(),
        inherits_from: Some("b".into()),
        capabilities: CapabilitySet::empty(),
    });
    assert!(res.is_err());
}

#[test]
fn unknown_parent_rejected() {
    let e = CapabilityEngine::new();
    let res = e.add_role(CapabilityRole {
        name: "x".into(),
        inherits_from: Some("does-not-exist".into()),
        capabilities: CapabilitySet::empty(),
    });
    assert!(res.is_err());
}

#[test]
fn unknown_role_binding_rejected() {
    let e = CapabilityEngine::with_defaults();
    let res = e.bind(local("x"), "non-existent-role".into());
    assert!(res.is_err());
}

#[test]
fn spawn_kinds_distinct() {
    let e = CapabilityEngine::with_defaults();
    let g = local("g");
    e.bind(g.clone(), "guest".into()).unwrap();
    // Guest gets no Spawn caps.
    assert!(e
        .check(
            &g,
            &Capability::Spawn {
                spawn_kind: SpawnKind::Subprocess,
            },
        )
        .is_err());
}
