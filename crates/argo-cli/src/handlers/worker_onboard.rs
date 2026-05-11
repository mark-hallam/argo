//! One-command worker onboarding from a TOML persona/role spec.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_cron::CronExpr;
use argo_identity::{
    Duty, IdentityStore, MAX_CHARTER_BYTES, MemoryNs, MemoryScope, NotifyChannelRef, Persona, Role,
    Worker, WorkerId,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_CHANNELS: usize = 8;
const BUILDER: &str = "argo worker onboard";
const SUPPORTED_CHANNELS: &[&str] = &["email", "slack", "push", "webinbox", "sms", "discord"];
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnboardSpec {
    #[serde(default)]
    vertical_id: Option<String>,
    #[serde(default)]
    persona: PersonaSpec,
    #[serde(default)]
    role: RoleSpec,
    #[serde(default)]
    channels: Vec<ChannelSpec>,
    #[serde(default)]
    duties: Vec<DutySpec>,
    #[serde(default)]
    memory_namespace: Option<String>,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersonaSpec {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    voice: Option<String>,
    #[serde(default)]
    signature: Option<String>,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleSpec {
    #[serde(default, alias = "charter")]
    description: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChannelSpec {
    #[serde(default, alias = "kind")]
    channel: Option<String>,
    target: String,
    #[serde(default)]
    risk_level: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum DutySpec {
    Table(DutyTable),
    Shorthand(String),
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DutyTable {
    cron: String,
    entry_action: String,
    description: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct OnboardOutput { worker_id: String, tenant_id: String, vertical_id: String, persona: String, memory_namespace: String, channels: usize, duties: usize }
pub async fn run(
    persona_spec: &Path,
    id: Option<&str>,
    role_name: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let raw = std::fs::read_to_string(persona_spec)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", persona_spec.display())))?;
    let spec = parse_spec_str(&raw)?;
    let worker_id = id.map(parse_worker_id).transpose()?;
    let (store, tenant_id) = open_workspace_store().await?;
    let out = create_from_spec(&store, tenant_id, spec, worker_id, role_name).await?;
    print_output(&out, json)
}
pub fn parse_spec_str(raw: &str) -> ArgoResult<OnboardSpec> {
    toml::from_str(raw).map_err(|e| ArgoError::Provider(format!("parse onboarding TOML: {e}")))
}
pub fn validate(spec: &OnboardSpec) -> ArgoResult<()> {
    required(spec.vertical_id.as_deref(), "vertical_id")?;
    let vertical = spec.vertical_id.as_deref().unwrap_or_default();
    if !valid_vertical_id(vertical) {
        return Err(ArgoError::Provider(format!(
            "vertical_id '{vertical}' must contain only letters, numbers, '-' or '_'"
        )));
    }
    required(spec.persona.name.as_deref(), "persona.name")?;
    required(spec.persona.voice.as_deref(), "persona.voice")?;
    required(spec.persona.signature.as_deref(), "persona.signature")?;
    let charter = required(spec.role.description.as_deref(), "role.description")?;
    if charter.len() > MAX_CHARTER_BYTES {
        return Err(ArgoError::Provider(format!(
            "role.description exceeds {MAX_CHARTER_BYTES} bytes"
        )));
    }
    if spec.channels.len() > MAX_CHANNELS {
        return Err(ArgoError::Provider(format!(
            "onboarding spec has {} channels; at most {MAX_CHANNELS} are allowed",
            spec.channels.len()
        )));
    }
    Ok(())
}
pub async fn create_from_spec(
    store: &SessionStore,
    tenant_id: TenantId,
    spec: OnboardSpec,
    id: Option<WorkerId>,
    role_name: Option<&str>,
) -> ArgoResult<OnboardOutput> {
    validate(&spec)?;
    let identities = IdentityStore::new(store);
    identities.ensure_schema().await?;
    let duties = parse_duties(&spec.duties)?;
    let mut worker = build_worker(tenant_id, &spec, duties);
    if let Some(id) = id {
        worker.id = id;
        worker.memory_namespace = MemoryNs::new(tenant_id, id, MemoryScope::WorkerSelf);
    }
    if identities.get_worker(tenant_id, worker.id).await?.is_some() {
        return Err(ArgoError::Provider(format!(
            "worker id {} already exists; onboarding does not mutate existing workers",
            worker.id
        )));
    }
    identities.create_worker(&worker).await?;
    if let Some(name) = role_name {
        identities
            .set_worker_role_name(tenant_id, worker.id, Some(name))
            .await?;
    }
    let channels = apply_channels(store, tenant_id, worker.id, &spec.channels).await?;
    let memory_namespace = namespace_label(&worker, spec.memory_namespace.as_deref());
    Ok(OnboardOutput {
        worker_id: worker.id.to_string(),
        tenant_id: tenant_id.to_string(),
        vertical_id: worker.vertical_id.clone(),
        persona: worker.persona.name.clone(),
        memory_namespace,
        channels: channels.len(),
        duties: worker.role.duties.len(),
    })
}
fn build_worker(tenant_id: TenantId, spec: &OnboardSpec, duties: Vec<Duty>) -> Worker {
    let name = spec.persona.name.as_deref().unwrap_or_default();
    let mut persona = Persona::named(name, BUILDER);
    persona.voice_profile.preamble = voice_preamble(&spec.persona);
    let mut role = Role::new(spec.role.description.clone().unwrap_or_default());
    role.duties = duties;
    let mut worker = Worker::draft(
        tenant_id,
        spec.vertical_id.clone().unwrap_or_default(),
        persona,
        role,
    );
    worker.charter = Some(worker.role.charter.clone());
    worker
}
async fn apply_channels(
    store: &SessionStore,
    tenant_id: TenantId,
    worker_id: WorkerId,
    specs: &[ChannelSpec],
) -> ArgoResult<Vec<NotifyChannelRef>> {
    let mut applied = Vec::with_capacity(specs.len());
    for raw in specs {
        let channel = parse_channel(raw)?;
        if !SUPPORTED_CHANNELS.contains(&channel.channel.as_str()) {
            return Err(ArgoError::Provider(format!(
                "channel '{}' is not supported for worker {worker_id}",
                label(raw)
            )));
        }
        applied.push(channel);
        persist_channels(store, tenant_id, worker_id, &applied).await?;
    }
    Ok(applied)
}
async fn persist_channels(
    store: &SessionStore,
    tenant_id: TenantId,
    worker_id: WorkerId,
    channels: &[NotifyChannelRef],
) -> ArgoResult<()> {
    let raw = serde_json::to_string(channels)
        .map_err(|e| ArgoError::Provider(format!("serialize channels: {e}")))?;
    let changed = sqlx::query("UPDATE worker SET channels_json = ? WHERE id = ? AND tenant_id = ?")
        .bind(raw)
        .bind(worker_id.to_string())
        .bind(tenant_id.to_string())
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("update worker channels: {e}")))?;
    if changed.rows_affected() == 0 {
        return Err(ArgoError::Provider(format!(
            "worker {worker_id} not found while adding channels"
        )));
    }
    Ok(())
}
fn parse_channel(raw: &ChannelSpec) -> ArgoResult<NotifyChannelRef> {
    let channel = required(raw.channel.as_deref(), "channels[].channel")?;
    let target = required(Some(raw.target.as_str()), "channels[].target")?;
    let mut ref_ = NotifyChannelRef::new(channel, target);
    ref_.risk_level = raw.risk_level.clone().filter(|v| !v.trim().is_empty());
    Ok(ref_)
}
fn parse_duties(specs: &[DutySpec]) -> ArgoResult<Vec<Duty>> {
    specs
        .iter()
        .map(|raw| {
            let (cron, entry, desc) = match raw {
                DutySpec::Table(t) => (t.cron.as_str(), t.entry_action.as_str(), t.description.as_str()),
                DutySpec::Shorthand(s) => {
                    let parts: Vec<&str> = s.splitn(3, "::").collect();
                    if parts.len() != 3 {
                        return Err(ArgoError::Provider(
                            "duty must be '<cron>::<entry_action>::<description>'".into(),
                        ));
                    }
                    (parts[0], parts[1], parts[2])
                }
            };
            CronExpr::parse(cron)
                .map_err(|e| ArgoError::Provider(format!("duty cron '{cron}': {e}")))?;
            Ok(Duty::new(cron, entry, desc))
        })
        .collect()
}
async fn open_workspace_store() -> ArgoResult<(SessionStore, TenantId)> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant_id, _workspace) = store.ensure_workspace(name, &root).await?;
    Ok((store, tenant_id))
}
fn voice_preamble(persona: &PersonaSpec) -> String {
    format!("{}\n\nSignature: {}", persona.voice.as_deref().unwrap_or_default(), persona.signature.as_deref().unwrap_or_default())
}
fn namespace_label(worker: &Worker, seed: Option<&str>) -> String {
    match seed.map(str::trim).filter(|s| !s.is_empty()) {
        Some(seed) => format!("{seed}:{}", worker.id),
        None => format!("worker_self:{}", worker.id),
    }
}
fn valid_vertical_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64
        && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}
fn required<'a>(value: Option<&'a str>, label: &str) -> ArgoResult<&'a str> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ArgoError::Provider(format!("missing required {label}")))
}
fn parse_worker_id(value: &str) -> ArgoResult<WorkerId> {
    value.parse()
        .map_err(|e| ArgoError::Provider(format!("worker id '{value}': {e}")))
}
fn label(raw: &ChannelSpec) -> String {
    format!(
        "{}:{}",
        raw.channel.as_deref().unwrap_or("<missing>"),
        raw.target
    )
}
fn print_output(out: &OnboardOutput, json: bool) -> ArgoResult<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(out)
                .map_err(|e| ArgoError::Provider(format!("serialize onboard output: {e}")))?
        );
    } else {
        println!("worker_id: {}", out.worker_id);
        println!("tenant_id: {}", out.tenant_id);
        println!();
        println!("## Worker onboarding");
        println!("- persona: {}", out.persona);
        println!("- vertical_id: {}", out.vertical_id);
        println!("- memory_namespace: {}", out.memory_namespace);
        println!("- channels: {}", out.channels);
        println!("- duties: {}", out.duties);
    }
    Ok(())
}

#[cfg(test)]
#[path = "worker_onboard_tests.rs"]
mod tests;
