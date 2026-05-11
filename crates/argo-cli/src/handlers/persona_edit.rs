//! `argo persona edit` — `$EDITOR`-driven persona update.
//!
//! Stream `claude-persona-edit-cli`. Personas evolve: voice gets
//! refined, bios get rewritten, avatar URLs change. Today the only
//! path is "read the JSON, edit by hand, re-create the persona,
//! re-attach to workers" — a five-step dance no operator does.
//! `argo persona edit <name>` collapses it to one command.
//!
//! Flow:
//! 1. resolve persona by `name` from the `persona` table
//!    (disambiguate with `--tenant <uuid>` if multiple match)
//! 2. dump editable fields to a TOML snippet
//! 3. write to a temp file, spawn `$EDITOR` (or read from
//!    `--in-place <path>` for tests + scripts)
//! 4. on editor exit, re-read, parse, validate
//! 5. on validation failure, leave the temp file in place and
//!    print its path so the operator can recover their unsaved
//!    edit; exit non-zero
//! 6. on success, UPDATE the persona row in place

use crate::cli::PersonaCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::Persona;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::path::{Path, PathBuf};

/// Voice (`voice_profile.preamble`) hard cap. Matches the brief's
/// "voice ≤ 4 KB" validation.
pub const MAX_VOICE_BYTES: usize = 4 * 1024;

/// Bio cap. The brief calls this "signature ≤ 1 KB"; the actual
/// data model exposes `bio` as the operator-editable free-text
/// field, so we apply the limit there.
pub const MAX_BIO_BYTES: usize = 1024;

/// Top-level dispatch from `Command::Persona { cmd }` in
/// `main.rs`.
pub async fn run(cmd: PersonaCmd) -> ArgoResult<()> {
    match cmd {
        PersonaCmd::Edit {
            name,
            tenant,
            in_place,
            json,
        } => edit(&name, tenant.as_deref(), in_place.as_deref(), json).await,
        PersonaCmd::History {
            worker_id,
            limit,
            json,
        } => crate::handlers::persona_history::run(&worker_id, limit, json).await,
        PersonaCmd::Revert {
            rev_id,
            yes,
            json,
        } => crate::handlers::persona_revert::run(&rev_id, yes, json).await,
    }
}

/// Wire-format TOML view of an editable persona. The schema is
/// part of the public surface — adding a field is fine, renaming
/// or removing one breaks operator scripts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PersonaToml {
    /// Top-level `[persona]` block.
    pub persona: PersonaTomlBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PersonaTomlBody {
    /// Human-facing name. Required.
    pub name: String,
    /// Voice / system-prompt preamble. Required.
    pub voice: String,
    /// Optional public biography (≤ 1 KB).
    #[serde(default)]
    pub bio: Option<String>,
    /// Optional avatar URL or asset id.
    #[serde(default)]
    pub avatar: Option<String>,
}

impl PersonaTomlBody {
    fn from_persona(p: &Persona) -> Self {
        Self {
            name: p.name.clone(),
            voice: p.voice_profile.preamble.clone(),
            bio: p.bio.clone(),
            avatar: p.avatar.clone(),
        }
    }
}

/// Serialise an editable view of `persona` as TOML.
///
/// `builder_attribution` and `provenance_chain` are deliberately
/// excluded — they are audit / identity fields, not "edit me"
/// fields. Round-tripping `dump_toml` through `parse_toml`
/// preserves equality of the editable subset only.
pub fn dump_toml(persona: &Persona) -> String {
    let view = PersonaToml {
        persona: PersonaTomlBody::from_persona(persona),
    };
    toml::to_string_pretty(&view).unwrap_or_else(|_| String::new())
}

/// Parse a TOML snippet into the editable persona view.
pub fn parse_toml(s: &str) -> ArgoResult<PersonaTomlBody> {
    let parsed: PersonaToml = toml::from_str(s)
        .map_err(|e| ArgoError::Schema(format!("invalid persona TOML: {e}")))?;
    validate(&parsed.persona)?;
    Ok(parsed.persona)
}

/// Validate name regex + size limits. Returns
/// `ArgoError::Schema` so the dispatch path bubbles a clean exit
/// code.
pub fn validate(body: &PersonaTomlBody) -> ArgoResult<()> {
    if !is_valid_persona_name(&body.name) {
        return Err(ArgoError::Schema(format!(
            "persona name '{}' is not valid (allowed: 1-64 chars, letters/digits/'_'/'-'/space, must start with a letter)",
            body.name
        )));
    }
    if body.voice.len() > MAX_VOICE_BYTES {
        return Err(ArgoError::Schema(format!(
            "voice exceeds {MAX_VOICE_BYTES} bytes (got {})",
            body.voice.len()
        )));
    }
    if let Some(bio) = body.bio.as_deref() {
        if bio.len() > MAX_BIO_BYTES {
            return Err(ArgoError::Schema(format!(
                "bio exceeds {MAX_BIO_BYTES} bytes (got {})",
                bio.len()
            )));
        }
    }
    Ok(())
}

/// Allowed: 1-64 chars, must start with a letter, then letters /
/// digits / underscore / hyphen / space. No leading/trailing
/// whitespace.
pub fn is_valid_persona_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    if name.starts_with(' ') || name.ends_with(' ') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ' ')
}

/// `$EDITOR` resolution: `$ARGO_EDITOR` > `$VISUAL` > `$EDITOR` >
/// `vi`.
pub fn resolve_editor() -> String {
    for var in ["ARGO_EDITOR", "VISUAL", "EDITOR"] {
        if let Ok(val) = std::env::var(var) {
            if !val.trim().is_empty() {
                return val;
            }
        }
    }
    "vi".into()
}

/// `argo persona edit <name> [--tenant <id>] [--in-place <path>]
/// [--json]` entry point.
pub async fn edit(
    name: &str,
    tenant: Option<&str>,
    in_place: Option<&Path>,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    let row = lookup_persona(&store, name, tenant).await?;

    let new_body = match in_place {
        Some(path) => read_in_place(path)?,
        None => spawn_editor_round_trip(&row.persona)?,
    };

    apply_update(&store, &row, new_body.clone()).await?;
    print_outcome(&new_body, json);
    Ok(())
}

fn read_in_place(path: &Path) -> ArgoResult<PersonaTomlBody> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    parse_toml(&body)
}

fn spawn_editor_round_trip(persona: &Persona) -> ArgoResult<PersonaTomlBody> {
    let dump = dump_toml(persona);
    let temp = tempfile::Builder::new()
        .prefix("argo-persona-edit-")
        .suffix(".toml")
        .tempfile()
        .map_err(|e| ArgoError::Io(format!("create temp file: {e}")))?;
    std::fs::write(temp.path(), &dump)
        .map_err(|e| ArgoError::Io(format!("write temp file: {e}")))?;

    let editor = resolve_editor();
    let status = std::process::Command::new(&editor)
        .arg(temp.path())
        .status()
        .map_err(|e| ArgoError::Io(format!("spawn $EDITOR ({editor}): {e}")))?;
    if !status.success() {
        return Err(ArgoError::Provider(format!(
            "$EDITOR ({editor}) exited with {status}"
        )));
    }

    let edited = std::fs::read_to_string(temp.path())
        .map_err(|e| ArgoError::Io(format!("re-read edited file: {e}")))?;
    match parse_toml(&edited) {
        Ok(body) => Ok(body),
        Err(err) => {
            // Persist the temp file so the operator can recover
            // their unsaved edit.
            let kept = match temp.keep() {
                Ok((_, path)) => path,
                Err(persist_err) => {
                    return Err(ArgoError::Provider(format!(
                        "{err} (and could not preserve temp file: {})",
                        persist_err.error
                    )));
                }
            };
            Err(ArgoError::Schema(format!(
                "{err} — your edit was preserved at {}",
                kept.display()
            )))
        }
    }
}

#[derive(Debug, Clone)]
struct PersonaRow {
    id: String,
    persona: Persona,
}

async fn lookup_persona(
    store: &OwnedStore,
    name: &str,
    tenant: Option<&str>,
) -> ArgoResult<PersonaRow> {
    let pool = store.pool();
    let rows = match tenant {
        Some(tenant) => {
            let tenant_id = parse_tenant(tenant)?;
            sqlx::query(PERSONA_SELECT_BY_NAME_TENANT)
                .bind(name)
                .bind(tenant_id.to_string())
                .fetch_all(pool)
                .await
        }
        None => sqlx::query(PERSONA_SELECT_BY_NAME)
            .bind(name)
            .fetch_all(pool)
            .await,
    }
    .map_err(|e| ArgoError::Provider(format!("lookup persona '{name}': {e}")))?;

    match rows.len() {
        0 => Err(ArgoError::Provider(format!(
            "persona '{name}' not found{}",
            tenant.map(|t| format!(" in tenant {t}")).unwrap_or_default()
        ))),
        1 => row_to_persona(&rows[0]),
        n => {
            let tenants: Vec<String> = rows
                .iter()
                .map(|row| row.get::<String, _>("tenant_id"))
                .collect();
            Err(ArgoError::Provider(format!(
                "persona '{name}' is ambiguous ({n} matches across tenants {tenants:?}); pass --tenant <uuid> to disambiguate"
            )))
        }
    }
}

fn row_to_persona(row: &sqlx::sqlite::SqliteRow) -> ArgoResult<PersonaRow> {
    let provenance_raw: String = row.get("provenance_chain_json");
    let provenance_chain: Vec<String> = serde_json::from_str(&provenance_raw)
        .map_err(|e| ArgoError::Provider(format!("parse provenance chain: {e}")))?;
    let persona = Persona {
        name: row.get("name"),
        voice_profile: argo_identity::VoiceProfile {
            preamble: row.get("voice_preamble"),
            ..Default::default()
        },
        avatar: row.get("avatar"),
        bio: row.get("bio"),
        builder_attribution: row.get("builder_attribution"),
        provenance_chain,
    };
    Ok(PersonaRow {
        id: row.get("id"),
        persona,
    })
}

async fn apply_update(
    store: &OwnedStore,
    row: &PersonaRow,
    body: PersonaTomlBody,
) -> ArgoResult<()> {
    sqlx::query(PERSONA_UPDATE)
        .bind(&body.name)
        .bind(&body.voice)
        .bind(&body.avatar)
        .bind(&body.bio)
        .bind(&row.id)
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("update persona: {e}")))?;
    Ok(())
}

fn print_outcome(body: &PersonaTomlBody, json: bool) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "name": body.name,
            "voice_bytes": body.voice.len(),
            "bio_bytes": body.bio.as_deref().map(str::len).unwrap_or(0),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!(
            "persona {}: updated (voice {} bytes, bio {} bytes)",
            body.name,
            body.voice.len(),
            body.bio.as_deref().map(str::len).unwrap_or(0),
        );
    }
}

fn parse_tenant(raw: &str) -> ArgoResult<TenantId> {
    uuid::Uuid::parse_str(raw)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("invalid tenant id '{raw}': {e}")))
}

async fn open_store() -> ArgoResult<OwnedStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(OwnedStore { store, _home: home })
}

pub(crate) struct OwnedStore {
    store: SessionStore,
    _home: PathBuf,
}

impl OwnedStore {
    pub(crate) fn pool(&self) -> &sqlx::Pool<sqlx::Sqlite> {
        self.store.pool()
    }
}

const PERSONA_SELECT_BY_NAME: &str = "SELECT id, tenant_id, name, voice_preamble, avatar, bio, builder_attribution, provenance_chain_json FROM persona WHERE name = ?";

const PERSONA_SELECT_BY_NAME_TENANT: &str = "SELECT id, tenant_id, name, voice_preamble, avatar, bio, builder_attribution, provenance_chain_json FROM persona WHERE name = ? AND tenant_id = ?";

const PERSONA_UPDATE: &str =
    "UPDATE persona SET name = ?, voice_preamble = ?, avatar = ?, bio = ? WHERE id = ?";

#[cfg(test)]
#[path = "persona_edit_tests.rs"]
mod tests;
