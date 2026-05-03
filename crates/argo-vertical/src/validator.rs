//! Cross-reference validator for [`VerticalManifest`]. Once the
//! manifest is structurally valid (parser stage), this stage
//! checks the *semantic* invariants:
//!
//! - the `id` slug is well-formed (lowercase, dashes, no slashes);
//! - file references in role / memory / skills / workflows /
//!   evaluation actually exist on disk relative to the vertical
//!   root;
//! - tool allow/deny lists don't conflict;
//! - duplicate gateway endpoints aren't declared.
//!
//! Returns the *first* failure as an [`ArgoError::Schema`]. The
//! loader runs validation per-vertical so each error is scoped to
//! its source manifest.
use crate::manifest::VerticalManifest;
use argo_types::{ArgoError, ArgoResult};
use std::collections::HashSet;
use std::path::Path;

/// Validate the manifest against on-disk references.
///
/// `root` is the directory containing `vertical.toml`; relative
/// paths in the manifest resolve against it.
pub fn validate(manifest: &VerticalManifest, root: &Path) -> ArgoResult<()> {
    validate_id(&manifest.vertical.id)?;
    validate_role(manifest, root)?;
    validate_memory(manifest, root)?;
    validate_skills(manifest, root)?;
    validate_workflows(manifest, root)?;
    validate_evaluation(manifest, root)?;
    validate_tools(manifest)?;
    validate_gateway(manifest)?;
    Ok(())
}

fn validate_id(id: &str) -> ArgoResult<()> {
    if id.is_empty() {
        return Err(ArgoError::Schema(
            "vertical.id must not be empty".into(),
        ));
    }
    let ok = id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok {
        return Err(ArgoError::Schema(format!(
            "vertical.id '{id}' must be lowercase ASCII letters, digits, or '-'"
        )));
    }
    if id.starts_with('-') || id.ends_with('-') {
        return Err(ArgoError::Schema(format!(
            "vertical.id '{id}' must not start or end with '-'"
        )));
    }
    Ok(())
}

fn check_file(root: &Path, rel: &str, label: &str) -> ArgoResult<()> {
    let p = root.join(rel);
    if !p.exists() {
        return Err(ArgoError::Schema(format!(
            "{label}: referenced file '{rel}' does not exist (resolved to {})",
            p.display()
        )));
    }
    if !p.is_file() {
        return Err(ArgoError::Schema(format!(
            "{label}: referenced path '{rel}' is not a file"
        )));
    }
    Ok(())
}

fn check_dir(root: &Path, rel: &str, label: &str) -> ArgoResult<()> {
    let p = root.join(rel);
    if !p.exists() {
        return Err(ArgoError::Schema(format!(
            "{label}: referenced dir '{rel}' does not exist (resolved to {})",
            p.display()
        )));
    }
    if !p.is_dir() {
        return Err(ArgoError::Schema(format!(
            "{label}: referenced path '{rel}' is not a directory"
        )));
    }
    Ok(())
}

fn validate_role(manifest: &VerticalManifest, root: &Path) -> ArgoResult<()> {
    if let Some(role) = &manifest.role {
        check_file(root, &role.system_prompt, "[role].system_prompt")?;
        if let Some(persona) = &role.persona {
            check_file(root, persona, "[role].persona")?;
        }
    }
    Ok(())
}

fn validate_memory(manifest: &VerticalManifest, root: &Path) -> ArgoResult<()> {
    if let Some(mem) = &manifest.memory {
        if let Some(schema) = &mem.schema {
            check_file(root, schema, "[memory].schema")?;
            // Parse + validate the schema file (#663). Catches
            // typos, duplicate keys, unknown types, and version
            // drift at vertical-load time so the operator sees
            // the failure once at startup instead of at first
            // memory write. Pure additive — older manifests that
            // pointed at a hand-written schema continue to load
            // as long as the schema's well-formed.
            let path = root.join(schema);
            let src = std::fs::read_to_string(&path)
                .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
            crate::memory_schema::parse_memory_schema(&src).map_err(|e| {
                ArgoError::Schema(format!("[memory].schema: {schema}: {e}"))
            })?;
        }
        if let Some(seed) = &mem.seed {
            check_dir(root, seed, "[memory].seed")?;
        }
    }
    Ok(())
}

fn validate_skills(manifest: &VerticalManifest, root: &Path) -> ArgoResult<()> {
    if let Some(skills) = &manifest.skills {
        check_dir(root, &skills.bundle, "[skills].bundle")?;
    }
    Ok(())
}

fn validate_workflows(manifest: &VerticalManifest, root: &Path) -> ArgoResult<()> {
    if let Some(wf) = &manifest.workflows {
        let mut seen_names: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for f in &wf.files {
            check_file(root, f, "[workflows].files")?;
            // Parse + validate the workflow file (#658). Catches
            // schema drift at vertical-load time rather than at
            // run-time. Per-workflow names must be unique within
            // the vertical so the runner can resolve by name.
            let path = root.join(f);
            let src = std::fs::read_to_string(&path)
                .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
            let workflow = crate::workflow::parse_workflow(&src).map_err(|e| {
                ArgoError::Schema(format!("[workflows].files: {f}: {e}"))
            })?;
            if !seen_names.insert(workflow.workflow.name.clone()) {
                return Err(ArgoError::Schema(format!(
                    "[workflows]: workflow name '{}' is declared by more than one file",
                    workflow.workflow.name
                )));
            }
        }
    }
    Ok(())
}

fn validate_evaluation(manifest: &VerticalManifest, root: &Path) -> ArgoResult<()> {
    if let Some(eval) = &manifest.evaluation {
        for s in &eval.suites {
            check_file(root, s, "[evaluation].suites")?;
        }
    }
    Ok(())
}

fn validate_tools(manifest: &VerticalManifest) -> ArgoResult<()> {
    if let Some(tools) = &manifest.tools {
        let allowed: HashSet<&str> = tools.allowed.iter().map(String::as_str).collect();
        let denied: HashSet<&str> = tools.denied.iter().map(String::as_str).collect();
        let conflict: Vec<&&str> = allowed.intersection(&denied).collect();
        if !conflict.is_empty() {
            let mut names: Vec<&str> = conflict.iter().map(|&&s| s).collect();
            names.sort();
            return Err(ArgoError::Schema(format!(
                "[tools]: tool(s) {names:?} appear in both allowed and denied lists"
            )));
        }
    }
    Ok(())
}

fn validate_gateway(manifest: &VerticalManifest) -> ArgoResult<()> {
    if let Some(gw) = &manifest.gateway {
        let mut seen: HashSet<&str> = HashSet::new();
        for ep in &gw.public_endpoints {
            if !seen.insert(ep.as_str()) {
                return Err(ArgoError::Schema(format!(
                    "[gateway]: duplicate public_endpoint '{ep}'"
                )));
            }
            if !ep.starts_with('/') {
                return Err(ArgoError::Schema(format!(
                    "[gateway]: public_endpoint '{ep}' must start with '/'"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::parse;
    use std::fs;
    use tempfile::tempdir;

    fn write_minimal(root: &Path) {
        fs::write(
            root.join("vertical.toml"),
            "schema_version = \"1\"\n\n[vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
    }

    #[test]
    fn minimal_manifest_validates() {
        let dir = tempdir().unwrap();
        write_minimal(dir.path());
        let src = fs::read_to_string(dir.path().join("vertical.toml")).unwrap();
        let m = parse(&src).unwrap();
        validate(&m, dir.path()).expect("minimal manifest must validate");
    }

    #[test]
    fn rejects_uppercase_id() {
        let mut m = parse(
            "schema_version = \"1\"\n[vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        m.vertical.id = "DemoBad".into();
        let dir = tempdir().unwrap();
        let err = validate(&m, dir.path()).unwrap_err();
        assert!(format!("{err}").contains("must be lowercase"));
    }

    #[test]
    fn rejects_id_with_underscore() {
        let mut m = parse(
            "schema_version = \"1\"\n[vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        m.vertical.id = "bad_id".into();
        let err = validate(&m, tempdir().unwrap().path()).unwrap_err();
        assert!(format!("{err}").contains("lowercase"));
    }

    #[test]
    fn rejects_missing_role_file() {
        let dir = tempdir().unwrap();
        let src = "schema_version = \"1\"\n[vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n[role]\nsystem_prompt = \"prompts/system.md\"\n";
        let m = parse(src).unwrap();
        let err = validate(&m, dir.path()).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("[role].system_prompt"), "got: {msg}");
        assert!(msg.contains("does not exist"), "got: {msg}");
    }

    #[test]
    fn accepts_valid_role_file() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("prompts")).unwrap();
        fs::write(dir.path().join("prompts/system.md"), "you are…").unwrap();
        let src = "schema_version = \"1\"\n[vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n[role]\nsystem_prompt = \"prompts/system.md\"\n";
        let m = parse(src).unwrap();
        validate(&m, dir.path()).expect("valid role passes");
    }

    #[test]
    fn rejects_skills_pointing_at_a_file_not_a_dir() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("skills"), "not a dir").unwrap();
        let src = "schema_version = \"1\"\n[vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n[skills]\nbundle = \"skills\"\n";
        let m = parse(src).unwrap();
        let err = validate(&m, dir.path()).unwrap_err();
        assert!(format!("{err}").contains("not a directory"));
    }

    #[test]
    fn rejects_tool_allow_deny_conflict() {
        let src = r#"
schema_version = "1"
[vertical]
id = "demo"
name = "D"
version = "0.1.0"
[tools]
allowed = ["a", "b"]
denied  = ["b", "c"]
"#;
        let m = parse(src).unwrap();
        let err = validate(&m, tempdir().unwrap().path()).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("[tools]"), "got: {msg}");
        assert!(msg.contains("\"b\""), "got: {msg}");
    }

    #[test]
    fn rejects_duplicate_gateway_endpoints() {
        let src = r#"
schema_version = "1"
[vertical]
id = "demo"
name = "D"
version = "0.1.0"
[gateway]
public_endpoints = ["/v1/x", "/v1/x"]
"#;
        let m = parse(src).unwrap();
        let err = validate(&m, tempdir().unwrap().path()).unwrap_err();
        assert!(format!("{err}").contains("duplicate"));
    }

    #[test]
    fn rejects_memory_schema_with_unknown_type() {
        // Lock #663: schema content is parsed at validate-time,
        // not just file-existence-checked.
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("memory.schema.toml"),
            "schema_version = \"1\"\n[[entry]]\nkey=\"x\"\ntype=\"telepathic\"\n",
        )
        .unwrap();
        let src = "schema_version = \"1\"\n[vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n[memory]\nschema = \"memory.schema.toml\"\n";
        let m = parse(src).unwrap();
        let err = validate(&m, dir.path()).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("[memory].schema"), "got: {msg}");
        assert!(msg.contains("unknown type 'telepathic'"), "got: {msg}");
    }

    #[test]
    fn accepts_well_formed_memory_schema() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("memory.schema.toml"),
            "schema_version = \"1\"\n[[entry]]\nkey=\"faqs\"\ntype=\"list-of-objects\"\npurpose=\"freq qns\"\n",
        )
        .unwrap();
        let src = "schema_version = \"1\"\n[vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n[memory]\nschema = \"memory.schema.toml\"\n";
        let m = parse(src).unwrap();
        validate(&m, dir.path()).expect("well-formed schema must validate");
    }

    #[test]
    fn rejects_gateway_endpoint_without_leading_slash() {
        let src = r#"
schema_version = "1"
[vertical]
id = "demo"
name = "D"
version = "0.1.0"
[gateway]
public_endpoints = ["v1/x"]
"#;
        let m = parse(src).unwrap();
        let err = validate(&m, tempdir().unwrap().path()).unwrap_err();
        assert!(format!("{err}").contains("must start with '/'"));
    }
}
