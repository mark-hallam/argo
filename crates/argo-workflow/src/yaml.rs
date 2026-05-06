//! YAML parser for workflow documents. Pure deserialise → topology
//! validate. The `Workflow` schema lives in [`crate::dag`] and
//! derives `serde::Deserialize` so this module is mostly an
//! ergonomics wrapper.
use crate::dag::Workflow;
use std::path::Path;
use thiserror::Error;

/// Errors surfaced by [`parse_yaml`] / [`parse_yaml_str`].
#[derive(Debug, Error)]
pub enum YamlError {
    /// I/O failed reading the file.
    #[error("read {path}: {error}")]
    Io {
        /// Path the caller asked us to read.
        path: String,
        /// Underlying I/O error.
        error: String,
    },
    /// YAML parse failed.
    #[error("parse: {0}")]
    Parse(String),
    /// YAML parsed but the workflow is malformed (cycle / unknown
    /// task / etc).
    #[error("invalid workflow: {0}")]
    Invalid(crate::dag::ValidationError),
}

/// Parse a YAML file from disk into a [`Workflow`]. Runs the
/// topology validator on the result; returns an error if the
/// graph is malformed.
pub fn parse_yaml(path: &Path) -> Result<Workflow, YamlError> {
    let body = std::fs::read_to_string(path).map_err(|e| YamlError::Io {
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    parse_yaml_str(&body)
}

/// Parse a YAML string into a [`Workflow`]. Same shape as
/// [`parse_yaml`] but skips the file read.
pub fn parse_yaml_str(yaml: &str) -> Result<Workflow, YamlError> {
    let workflow: Workflow =
        serde_yaml::from_str(yaml).map_err(|e| YamlError::Parse(e.to_string()))?;
    workflow.validate().map_err(YamlError::Invalid)?;
    Ok(workflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::ValidationError;

    #[test]
    fn parse_simple_linear_workflow() {
        let yaml = r#"
id: demo
tasks:
  - id: a
    worker: support-agent
    inputs:
      greeting: hello
  - id: b
    worker: support-agent
    inputs:
      upstream: "${tasks.a.out.echo}"
    depends_on: [a]
"#;
        let w = parse_yaml_str(yaml).expect("parses");
        assert_eq!(w.id, "demo");
        assert_eq!(w.tasks.len(), 2);
        assert_eq!(w.tasks[0].id, "a");
        assert_eq!(w.tasks[1].depends_on, vec!["a".to_string()]);
    }

    #[test]
    fn parse_diamond_workflow_via_explicit_edges() {
        let yaml = r#"
id: diamond
tasks:
  - id: a
    worker: w
  - id: b
    worker: w
  - id: c
    worker: w
  - id: d
    worker: w
edges:
  - { from_task: a, to_task: b }
  - { from_task: a, to_task: c }
  - { from_task: b, to_task: d }
  - { from_task: c, to_task: d }
"#;
        let w = parse_yaml_str(yaml).expect("parses");
        let order = w.topo_order().unwrap();
        let ids: Vec<&str> = order.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids[0], "a");
        assert_eq!(ids[3], "d");
    }

    #[test]
    fn parse_rejects_cycle() {
        let yaml = r#"
id: bad
tasks:
  - id: a
    worker: w
    depends_on: [b]
  - id: b
    worker: w
    depends_on: [a]
"#;
        let err = parse_yaml_str(yaml).expect_err("errors");
        match err {
            YamlError::Invalid(ValidationError::Cycle(_)) => {}
            other => panic!("expected cycle, got {other:?}"),
        }
    }

    #[test]
    fn parse_rejects_garbage_yaml() {
        let err = parse_yaml_str("@@not yaml@@").expect_err("errors");
        match err {
            YamlError::Parse(_) => {}
            other => panic!("expected parse error, got {other:?}"),
        }
    }

    #[test]
    fn parse_yaml_file_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("flow.yaml");
        std::fs::write(
            &path,
            r#"
id: file-roundtrip
tasks:
  - id: a
    worker: w
"#,
        )
        .unwrap();
        let w = parse_yaml(&path).unwrap();
        assert_eq!(w.id, "file-roundtrip");
    }
}
