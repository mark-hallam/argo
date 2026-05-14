use super::retry::{
    run_skill_with_retry, RetryBackoff, RetryFailure, RetryFailureClass, RetryPolicyStore,
    SkillRetryPolicy,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

async fn store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

fn fast_policy(tenant: TenantId) -> SkillRetryPolicy {
    let mut policy = SkillRetryPolicy::new(tenant, "invoice.extract");
    policy.initial_delay_ms = 0;
    policy.max_delay_ms = 0;
    policy
}

#[tokio::test]
async fn policy_storage_round_trips() {
    let store = RetryPolicyStore::new(store().await);
    let tenant = TenantId::new();
    let mut policy = SkillRetryPolicy::new(tenant, "invoice.extract");
    policy.ir_version = Some("v3".into());
    policy.max_attempts = 4;
    policy.retry_on = vec![RetryFailureClass::Timeout, RetryFailureClass::RateLimited];

    store.upsert(&policy).await.unwrap();
    let fetched = store.get(tenant, &policy.id).await.unwrap().unwrap();

    assert_eq!(fetched.skill_id, "invoice.extract");
    assert_eq!(fetched.ir_version.as_deref(), Some("v3"));
    assert_eq!(fetched.max_attempts, 4);
    assert_eq!(fetched.retry_on, policy.retry_on);
}

#[tokio::test]
async fn list_for_skill_is_tenant_scoped() {
    let store = RetryPolicyStore::new(store().await);
    let tenant = TenantId::new();
    let other = TenantId::new();
    let policy = SkillRetryPolicy::new(tenant, "invoice.extract");
    let other_policy = SkillRetryPolicy::new(other, "invoice.extract");
    store.upsert(&policy).await.unwrap();
    store.upsert(&other_policy).await.unwrap();

    let policies = store
        .list_for_skill(tenant, "invoice.extract")
        .await
        .unwrap();

    assert_eq!(policies.len(), 1);
    assert_eq!(policies[0].tenant_id, tenant);
}

#[tokio::test]
async fn failing_skill_retries_until_success() {
    let tenant = TenantId::new();
    let policy = fast_policy(tenant);
    let attempts = Arc::new(AtomicU32::new(0));
    let seen = attempts.clone();

    let result = run_skill_with_retry(&policy, || {
        let seen = seen.clone();
        async move {
            let attempt = seen.fetch_add(1, Ordering::SeqCst) + 1;
            if attempt < 3 {
                Err(RetryFailure::new(
                    RetryFailureClass::Transient,
                    "temporary upstream failure",
                ))
            } else {
                Ok("done")
            }
        }
    })
    .await
    .unwrap();

    assert_eq!(result, "done");
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn non_matching_failure_class_does_not_retry() {
    let tenant = TenantId::new();
    let mut policy = fast_policy(tenant);
    policy.retry_on = vec![RetryFailureClass::Timeout];
    let attempts = Arc::new(AtomicU32::new(0));
    let seen = attempts.clone();

    let err = run_skill_with_retry::<_, _, ()>(&policy, || {
        let seen = seen.clone();
        async move {
            seen.fetch_add(1, Ordering::SeqCst);
            Err(RetryFailure::new(
                RetryFailureClass::Internal,
                "deterministic bug",
            ))
        }
    })
    .await
    .unwrap_err();

    assert_eq!(err.class, RetryFailureClass::Internal);
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[test]
fn shared_backoff_math_is_honored() {
    let tenant = TenantId::new();
    let mut policy = SkillRetryPolicy::new(tenant, "invoice.extract");
    policy.initial_delay_ms = 100;
    policy.max_delay_ms = 250;
    policy.backoff = RetryBackoff::Exponential;

    assert_eq!(policy.delay_ms_for_attempt(1), 100);
    assert_eq!(policy.delay_ms_for_attempt(2), 200);
    assert_eq!(policy.delay_ms_for_attempt(3), 250);

    policy.backoff = RetryBackoff::None;
    assert_eq!(policy.delay_ms_for_attempt(1), 0);
}
