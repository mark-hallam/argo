use crate::archetype::Archetype;
use crate::error::GenesisError;
use argo_vertical::MemorySection;
use std::path::Path;

pub const SCHEMA_PATH: &str = "memory/schema.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedMemorySchema {
    pub namespaces: Vec<MemoryNamespace>,
    pub layers: Vec<MemoryLayer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryNamespace {
    pub id: String,
    pub scope: NamespaceScope,
    pub ttl_days: Option<u32>,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceScope {
    Worker,
    Team,
    Org,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryLayer {
    pub name: String,
    pub namespaces: Vec<String>,
    pub retrieval_policy: String,
}

pub fn generate(
    role_description: &str,
    archetype: Option<&Archetype>,
) -> Result<GeneratedMemorySchema, GenesisError> {
    let role = role_description.trim();
    if role.is_empty() {
        return Err(GenesisError::Validation {
            stage: "memory_schema.generate",
            detail: "role_description is empty".into(),
        });
    }
    let lowered = role.to_lowercase();
    let needs_org = contains_any(&lowered, &["company", "policy", "compliance"]);
    let needs_team = needs_org || contains_any(&lowered, &["team", "shared", "handoff"]);
    let label = archetype.map_or("worker", |a| a.label());
    let mut namespaces = vec![MemoryNamespace {
        id: "worker_context".into(),
        scope: NamespaceScope::Worker,
        ttl_days: Some(30),
        summary: format!("Role-specific memory for the {label} worker."),
    }];
    if needs_team {
        namespaces.push(MemoryNamespace {
            id: "team_context".into(),
            scope: NamespaceScope::Team,
            ttl_days: Some(90),
            summary: "Shared handoff notes, team preferences, and cross-worker context.".into(),
        });
    }
    if needs_org {
        namespaces.push(MemoryNamespace {
            id: "org_policy".into(),
            scope: NamespaceScope::Org,
            ttl_days: None,
            summary: "Company policy, compliance constraints, and durable org-wide rules.".into(),
        });
    }
    let layer = MemoryLayer {
        name: "default".into(),
        namespaces: namespaces.iter().map(|ns| ns.id.clone()).collect(),
        retrieval_policy: "recent_first_top_8".into(),
    };
    Ok(GeneratedMemorySchema {
        namespaces,
        layers: vec![layer],
    })
}

pub fn to_toml(schema: &GeneratedMemorySchema) -> String {
    let mut out = String::from("schema_version = \"1\"\n\n");
    for namespace in &schema.namespaces {
        out.push_str("[[entry]]\n");
        out.push_str(&format!("key = \"{}\"\n", escape(&namespace.id)));
        out.push_str("type = \"list-of-objects\"\n");
        out.push_str(&format!(
            "purpose = \"{}\"\n\n",
            escape(&entry_purpose(namespace, &schema.layers))
        ));
    }
    out
}

pub fn manifest_section() -> MemorySection {
    MemorySection {
        schema: Some(SCHEMA_PATH.into()),
        seed: None,
    }
}

pub fn write_schema_file(
    role_description: &str,
    archetype: Archetype,
    output: &Path,
) -> Result<(), GenesisError> {
    let memory_dir = output.join("memory");
    std::fs::create_dir_all(&memory_dir).map_err(|e| GenesisError::Io {
        what: "memory_schema.dir.create",
        source: e,
    })?;
    let schema = generate(role_description, Some(&archetype))?;
    std::fs::write(memory_dir.join("schema.toml"), to_toml(&schema)).map_err(|e| GenesisError::Io {
        what: "memory_schema.write",
        source: e,
    })
}

fn entry_purpose(namespace: &MemoryNamespace, layers: &[MemoryLayer]) -> String {
    let layer_names: Vec<&str> = layers
        .iter()
        .filter(|layer| layer.namespaces.iter().any(|id| id == &namespace.id))
        .map(|layer| layer.name.as_str())
        .collect();
    format!(
        "{} scope={} ttl_days={} layers={} retrieval={}",
        namespace.summary,
        scope_label(namespace.scope),
        namespace
            .ttl_days
            .map_or_else(|| "none".into(), |days| days.to_string()),
        layer_names.join(","),
        layers.first().map_or("recent_first_top_8", |layer| layer
            .retrieval_policy
            .as_str())
    )
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn scope_label(scope: NamespaceScope) -> &'static str {
    match scope {
        NamespaceScope::Worker => "worker",
        NamespaceScope::Team => "team",
        NamespaceScope::Org => "org",
    }
}

fn escape(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
#[path = "memory_schema_tests.rs"]
mod tests;
