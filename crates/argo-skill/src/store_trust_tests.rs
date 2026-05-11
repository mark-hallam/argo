//! Unit tests for [`SkillStore::set_min_trust_grade`] /
//! [`SkillStore::get_min_trust_grade`] — Stream
//! `mission-skill-min-trust-grade-column`.
#![cfg(test)]

use crate::store::{trust_grade_as_u8, trust_grade_from_u8};
use crate::trust::TrustGrade;
use crate::SkillStore;
use argo_store::SessionStore;
use argo_types::ArgoError;
use std::sync::Arc;

const SKILL_MD: &str = "---\n\
name: trust-test\n\
version: \"0.1.0\"\n\
description: \"fixture\"\n\
provenance:\n  \
source: builtin\n  \
hash: sha256-x\n\
---\nbody";

async fn fresh_store() -> SkillStore {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    SkillStore::new(store)
}

#[tokio::test]
async fn set_min_trust_grade_round_trips_some_value() {
    let skills = fresh_store().await;
    skills.install(SKILL_MD, None).await.unwrap();
    skills
        .set_min_trust_grade("trust-test", Some(TrustGrade::PublishedVerified))
        .await
        .unwrap();
    let got = skills.get_min_trust_grade("trust-test").await.unwrap();
    assert_eq!(got, Some(TrustGrade::PublishedVerified));
}

#[tokio::test]
async fn get_min_trust_grade_defaults_to_none_on_fresh_row() {
    let skills = fresh_store().await;
    skills.install(SKILL_MD, None).await.unwrap();
    let got = skills.get_min_trust_grade("trust-test").await.unwrap();
    assert_eq!(got, None);
}

#[tokio::test]
async fn set_min_trust_grade_none_clears_to_null() {
    let skills = fresh_store().await;
    skills.install(SKILL_MD, None).await.unwrap();
    skills
        .set_min_trust_grade("trust-test", Some(TrustGrade::Tenant))
        .await
        .unwrap();
    assert_eq!(
        skills.get_min_trust_grade("trust-test").await.unwrap(),
        Some(TrustGrade::Tenant),
    );
    skills
        .set_min_trust_grade("trust-test", None)
        .await
        .unwrap();
    assert_eq!(skills.get_min_trust_grade("trust-test").await.unwrap(), None);
}

#[tokio::test]
async fn set_min_trust_grade_unknown_name_errors() {
    let skills = fresh_store().await;
    let err = skills
        .set_min_trust_grade("nonexistent", Some(TrustGrade::Builtin))
        .await
        .expect_err("missing skill must error");
    match err {
        ArgoError::Provider(msg) => {
            assert!(
                msg.contains("nonexistent"),
                "error should mention the missing name, got {msg:?}"
            );
        }
        other => panic!("expected Provider error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_min_trust_grade_rejects_out_of_range_discriminant() {
    // Round-trip the discriminant helpers in isolation and verify
    // an out-of-range value fails-loud rather than silently mapping
    // to a default. This guards the corruption path the
    // production read takes when a future writer encodes the
    // column with an unrecognised discriminant.
    for grade in [
        TrustGrade::Builtin,
        TrustGrade::PublishedVerified,
        TrustGrade::Tenant,
        TrustGrade::PublishedCommunity,
        TrustGrade::LearnedUnreviewed,
    ] {
        let byte = trust_grade_as_u8(grade);
        assert_eq!(trust_grade_from_u8(byte), Some(grade));
    }
    assert_eq!(trust_grade_from_u8(5), None);
    assert_eq!(trust_grade_from_u8(255), None);
}
