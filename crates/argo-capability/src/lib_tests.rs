//! Tests for [`super::CapabilityEngine`] — extracted from `lib.rs` to
//! keep the production-code file under the 500-LOC ceiling.
#![cfg(test)]
use super::*;
use argo_types::SpawnKind;

fn local(user: &str) -> Principal {
    Principal::Local {
        os_user: user.into(),
    }
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

#[test]
fn role_inheritance_walks_parents() {
    let e = CapabilityEngine::new();
    e.add_role(Role {
        name: "base".into(),
        inherits_from: None,
        capabilities: CapabilitySet(vec![Capability::ToolDispatch {
            tool_name_glob: "memory".into(),
        }]),
    })
    .unwrap();
    e.add_role(Role {
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
    e.add_role(Role {
        name: "a".into(),
        inherits_from: None,
        capabilities: CapabilitySet::empty(),
    })
    .unwrap();
    e.add_role(Role {
        name: "b".into(),
        inherits_from: Some("a".into()),
        capabilities: CapabilitySet::empty(),
    })
    .unwrap();
    // Now try to redefine 'a' to inherit from 'b' → cycle.
    let res = e.add_role(Role {
        name: "a".into(),
        inherits_from: Some("b".into()),
        capabilities: CapabilitySet::empty(),
    });
    assert!(res.is_err());
}

#[test]
fn unknown_parent_rejected() {
    let e = CapabilityEngine::new();
    let res = e.add_role(Role {
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
