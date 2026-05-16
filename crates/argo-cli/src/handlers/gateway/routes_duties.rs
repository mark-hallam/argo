//! Per-worker duty endpoints (Stream J13): classify, commit, preview,
//! PATCH, DELETE under `/api/v1/workers/<id>/duties/`. Wired via
//! `dispatch_duties` in `routes_dispatch_post`. Sensible deviations
//! (hard delete, label→description, 5-field wire/6-field disk cron) are
//! documented in the PR body.
//!
//! Body parsing, worker resolution, cron/label validation, conflict
//! detection, audit emission, and error-envelope formatting live in
//! [`helpers`] so this file stays under the 500-LOC ceiling.

#[path = "routes_duties_helpers.rs"]
mod helpers;

use argo_genesis::{classify_charter, Archetype};
use argo_orchestrator::{DutyRecord, DutyStore};
use argo_store::SessionStore;
use chrono::Utc;
use cron::Schedule;
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;

use helpers::{
    apply_patch, collect_conflicts, defaults_for, delete_conflicting, duty_to_json, emit_audit,
    err, parse_body, resolve_worker, sample_outputs_for, validate_5_field_cron, validate_label,
};

const PREVIEW_DEFAULT_SAMPLES: usize = 3;
const PREVIEW_MAX_SAMPLES: usize = 3;

pub(super) async fn classify_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, worker_id, _) = match resolve_worker(worker_id_raw, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let parsed = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let input = match parsed.get("input").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => {
            return err(
                "400 Bad Request",
                "bad_request",
                "`input` (non-empty string) is required",
            )
        }
    };
    let archetype = classify_charter(&input).unwrap_or(Archetype::OpsOnCall);
    let defaults = defaults_for(archetype);
    let proposed_5 = parsed
        .get("cron_hint")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or(defaults.cron, |s| s)
        .to_string();
    let stored_cron = match validate_5_field_cron(&proposed_5) {
        Ok(s) => s,
        Err(msg) => return err("400 Bad Request", "bad_request", &msg),
    };
    let duty_store = DutyStore::new(Arc::clone(store));
    let conflicts = match collect_conflicts(&duty_store, &worker_id, &stored_cron).await {
        Ok(v) => v,
        Err(e) => {
            return err(
                "500 Internal Server Error",
                "internal_error",
                &format!("conflicts probe: {e}"),
            )
        }
    };
    let samples = sample_outputs_for(archetype);
    let body = json!({
        "duty_proposal": {
            "label": defaults.label, "schedule": proposed_5, "cron_stored": stored_cron,
            "entry_action": defaults.entry_action, "archetype": archetype.slug(),
        },
        "sample_outputs": samples,
        "conflicts": conflicts,
    });
    emit_audit(store, req, "duty.classify", &worker_id, samples.len()).await;
    ("200 OK", body.to_string())
}

pub(super) async fn commit_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, worker_id, tenant) = match resolve_worker(worker_id_raw, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let parsed = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let proposal = match parsed.get("duty_proposal") {
        Some(v) if v.is_object() => v,
        _ => {
            return err(
                "400 Bad Request",
                "bad_request",
                "`duty_proposal` (object) is required",
            )
        }
    };
    let label = match validate_label(proposal.get("label").and_then(Value::as_str).unwrap_or("")) {
        Ok(s) => s,
        Err(msg) => return err("400 Bad Request", "bad_request", &msg),
    };
    let schedule_raw = proposal
        .get("schedule")
        .and_then(Value::as_str)
        .unwrap_or("");
    let stored_cron = match validate_5_field_cron(schedule_raw) {
        Ok(s) => s,
        Err(msg) => return err("400 Bad Request", "bad_request", &msg),
    };
    let entry_action = proposal
        .get("entry_action")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(|| "ping".to_string(), str::to_string);
    let duty_store = DutyStore::new(Arc::clone(store));
    let resolution = parsed
        .get("accepted_conflict_resolution")
        .and_then(Value::as_str)
        .unwrap_or("");
    let existing_conflicts = match collect_conflicts(&duty_store, &worker_id, &stored_cron).await {
        Ok(v) => v,
        Err(e) => {
            return err(
                "500 Internal Server Error",
                "internal_error",
                &format!("conflicts probe: {e}"),
            )
        }
    };
    if !existing_conflicts.is_empty() {
        match resolution {
            "cancel" => return err("409 Conflict", "duty_conflict", "commit cancelled by conflict resolution"),
            "replace" => if let Err(e) = delete_conflicting(store, &existing_conflicts).await {
                return err("500 Internal Server Error", "internal_error", &format!("replace conflicts: {e}"));
            },
            "keep_both" => {}
            _ => return err("409 Conflict", "duty_conflict", "duty cron overlaps existing duties; supply `accepted_conflict_resolution` (replace|keep_both|cancel)"),
        }
    }
    let duty = DutyRecord::new(tenant, worker_id, stored_cron.clone(), entry_action, label);
    if let Err(e) = duty_store.insert(&duty).await {
        return err(
            "500 Internal Server Error",
            "internal_error",
            &format!("insert duty: {e}"),
        );
    }
    let first_fire = Schedule::from_str(&stored_cron)
        .ok()
        .and_then(|s| s.after(&Utc::now()).next());
    emit_audit(store, req, "duty.commit", &worker_id, 1).await;
    (
        "201 Created",
        json!({
            "duty_id": duty.id,
            "scheduled_first_fire": first_fire.map(|t| t.to_rfc3339()),
        })
        .to_string(),
    )
}

pub(super) async fn preview_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, _, _) = match resolve_worker(worker_id_raw, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let parsed = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let duty_id = match parsed.get("duty_id").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => {
            return err(
                "400 Bad Request",
                "bad_request",
                "`duty_id` (string) is required",
            )
        }
    };
    let sample_size = parsed
        .get("sample_size")
        .and_then(Value::as_u64)
        .map_or(PREVIEW_DEFAULT_SAMPLES, |n| n as usize)
        .clamp(1, PREVIEW_MAX_SAMPLES);
    let duty_store = DutyStore::new(Arc::clone(store));
    let duty = match duty_store.get(&duty_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return err(
                "404 Not Found",
                "not_found",
                &format!("duty '{duty_id}' not found"),
            )
        }
        Err(e) => {
            return err(
                "500 Internal Server Error",
                "internal_error",
                &format!("get duty: {e}"),
            )
        }
    };
    let arch = classify_charter(&duty.description).unwrap_or(Archetype::OpsOnCall);
    let outputs = sample_outputs_for(arch);
    let samples: Vec<Value> = outputs
        .iter()
        .take(sample_size)
        .enumerate()
        .map(|(idx, out)| {
            json!({
                "inbound_context": format!("synthesized fixture #{}", idx + 1),
                "would_send": out,
            })
        })
        .collect();
    emit_audit(store, req, "duty.preview", &duty.worker_id, samples.len()).await;
    ("200 OK", json!({ "samples": samples }).to_string())
}

pub(super) async fn patch_response(
    worker_id_raw: &str,
    duty_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, worker_id, _) = match resolve_worker(worker_id_raw, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let duty_id = duty_id_raw.split('?').next().unwrap_or(duty_id_raw);
    if duty_id.is_empty() {
        return err("400 Bad Request", "bad_request", "missing duty_id");
    }
    let parsed = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let duty_store = DutyStore::new(Arc::clone(store));
    let existing = match duty_store.get(duty_id).await {
        Ok(Some(d)) if d.worker_id == worker_id => d,
        Ok(_) => {
            return err(
                "404 Not Found",
                "not_found",
                &format!("duty '{duty_id}' not found"),
            )
        }
        Err(e) => {
            return err(
                "500 Internal Server Error",
                "internal_error",
                &format!("get duty: {e}"),
            )
        }
    };
    let new_cron = match parsed.get("schedule").and_then(Value::as_str) {
        Some(s) => match validate_5_field_cron(s) {
            Ok(stored) => Some(stored),
            Err(msg) => return err("400 Bad Request", "bad_request", &msg),
        },
        None => None,
    };
    let new_label = match parsed.get("label").and_then(Value::as_str) {
        Some(s) => match validate_label(s) {
            Ok(v) => Some(v),
            Err(msg) => return err("400 Bad Request", "bad_request", &msg),
        },
        None => None,
    };
    let new_enabled = parsed.get("enabled").and_then(Value::as_bool);
    if new_cron.is_none() && new_label.is_none() && new_enabled.is_none() {
        return err(
            "400 Bad Request",
            "bad_request",
            "at least one of `enabled`, `schedule`, `label` is required",
        );
    }
    if let Err(e) = apply_patch(
        store,
        duty_id,
        new_cron.as_deref(),
        new_label.as_deref(),
        new_enabled,
    )
    .await
    {
        return err(
            "500 Internal Server Error",
            "internal_error",
            &format!("update duty: {e}"),
        );
    }
    emit_audit(store, req, "duty.patch", &worker_id, 1).await;
    let refreshed = duty_store
        .get(duty_id)
        .await
        .ok()
        .flatten()
        .unwrap_or(existing);
    (
        "200 OK",
        json!({ "duty": duty_to_json(&refreshed) }).to_string(),
    )
}

pub(super) async fn delete_response(
    worker_id_raw: &str,
    duty_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, worker_id, _) = match resolve_worker(worker_id_raw, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let duty_id = duty_id_raw.split('?').next().unwrap_or(duty_id_raw);
    if duty_id.is_empty() {
        return err("400 Bad Request", "bad_request", "missing duty_id");
    }
    let duty_store = DutyStore::new(Arc::clone(store));
    match duty_store.get(duty_id).await {
        Ok(Some(d)) if d.worker_id == worker_id => {}
        Ok(_) => {
            return err(
                "404 Not Found",
                "not_found",
                &format!("duty '{duty_id}' not found"),
            )
        }
        Err(e) => {
            return err(
                "500 Internal Server Error",
                "internal_error",
                &format!("get duty: {e}"),
            )
        }
    }
    if let Err(e) = sqlx::query("DELETE FROM role_duty WHERE id = ?")
        .bind(duty_id)
        .execute(store.pool())
        .await
    {
        return err(
            "500 Internal Server Error",
            "internal_error",
            &format!("delete duty: {e}"),
        );
    }
    emit_audit(store, req, "duty.delete", &worker_id, 1).await;
    ("204 No Content", String::new())
}

#[cfg(test)]
#[path = "routes_duties_tests.rs"]
mod tests;
