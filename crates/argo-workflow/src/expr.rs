//! `${tasks.<id>.out.<key>}` expression resolver.
//!
//! Tasks declare inputs as `serde_json::Value`. When a string
//! input contains a `${...}` reference, the executor resolves it
//! against the running execution context's `task_outputs` map
//! before launching the task.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

const PREFIX: &str = "${tasks.";
const SUFFIX: &str = "}";

/// Parsed reference into another task's output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expr {
    /// Upstream task id.
    pub task_id: String,
    /// Output key inside the upstream task's result map.
    pub output_key: String,
}

/// Errors surfaced while resolving expressions.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExprError {
    /// The expression is malformed.
    #[error("malformed expression '{0}': expected `${{tasks.<id>.out.<key>}}`")]
    Malformed(String),
    /// Upstream task hasn't produced the requested output yet (or
    /// it failed and never produced one).
    #[error("upstream task '{task}' has no output '{key}'")]
    Missing {
        /// Upstream task id.
        task: String,
        /// Output key.
        key: String,
    },
    /// Upstream task hasn't run yet (or doesn't exist).
    #[error("upstream task '{0}' has no recorded outputs")]
    NoSuchUpstream(String),
}

impl Expr {
    /// Parse a `${tasks.<id>.out.<key>}` string. Returns `None`
    /// when the input doesn't look like an expression at all (so
    /// the caller can keep the original literal); errors when it
    /// looks like one but is malformed.
    pub fn parse(s: &str) -> Result<Option<Self>, ExprError> {
        let trimmed = s.trim();
        if !trimmed.starts_with(PREFIX) || !trimmed.ends_with(SUFFIX) {
            return Ok(None);
        }
        let inner = &trimmed[PREFIX.len()..trimmed.len() - SUFFIX.len()];
        // inner = "<id>.out.<key>"
        let Some((task_id, rest)) = inner.split_once(".out.") else {
            return Err(ExprError::Malformed(s.to_string()));
        };
        if task_id.is_empty() || rest.is_empty() {
            return Err(ExprError::Malformed(s.to_string()));
        }
        Ok(Some(Self {
            task_id: task_id.to_string(),
            output_key: rest.to_string(),
        }))
    }

    /// Resolve this expression against the running execution
    /// context's task-outputs map.
    pub fn resolve<'a>(
        &self,
        outputs: &'a BTreeMap<String, BTreeMap<String, serde_json::Value>>,
    ) -> Result<&'a serde_json::Value, ExprError> {
        let upstream = outputs
            .get(&self.task_id)
            .ok_or_else(|| ExprError::NoSuchUpstream(self.task_id.clone()))?;
        upstream.get(&self.output_key).ok_or_else(|| ExprError::Missing {
            task: self.task_id.clone(),
            key: self.output_key.clone(),
        })
    }
}

/// Walk an inputs map: for each string value that's a `${...}`
/// expression, resolve it; leave other values alone. Returns the
/// transformed inputs ready to hand to a task runner.
pub fn resolve_inputs(
    raw_inputs: &BTreeMap<String, serde_json::Value>,
    task_outputs: &BTreeMap<String, BTreeMap<String, serde_json::Value>>,
) -> Result<BTreeMap<String, serde_json::Value>, ExprError> {
    let mut out = BTreeMap::new();
    for (k, v) in raw_inputs {
        let resolved = match v.as_str() {
            Some(s) => match Expr::parse(s)? {
                Some(expr) => expr.resolve(task_outputs)?.clone(),
                None => v.clone(),
            },
            None => v.clone(),
        };
        out.insert(k.clone(), resolved);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_returns_none_for_plain_strings() {
        assert!(Expr::parse("hello").unwrap().is_none());
        assert!(Expr::parse("${not.tasks}").unwrap().is_none());
    }

    #[test]
    fn parse_accepts_well_formed_reference() {
        let e = Expr::parse("${tasks.foo.out.bar}").unwrap().unwrap();
        assert_eq!(e.task_id, "foo");
        assert_eq!(e.output_key, "bar");
    }

    #[test]
    fn parse_rejects_malformed() {
        assert!(matches!(
            Expr::parse("${tasks.}").err(),
            Some(ExprError::Malformed(_))
        ));
        assert!(matches!(
            Expr::parse("${tasks..out.x}").err(),
            Some(ExprError::Malformed(_))
        ));
        assert!(matches!(
            Expr::parse("${tasks.foo.out.}").err(),
            Some(ExprError::Malformed(_))
        ));
    }

    #[test]
    fn resolve_returns_upstream_output() {
        let mut outs = BTreeMap::new();
        let mut foo_outs = BTreeMap::new();
        foo_outs.insert("bar".into(), json!("hello"));
        outs.insert("foo".into(), foo_outs);
        let e = Expr {
            task_id: "foo".into(),
            output_key: "bar".into(),
        };
        assert_eq!(e.resolve(&outs).unwrap(), &json!("hello"));
    }

    #[test]
    fn resolve_errors_when_upstream_absent() {
        let outs = BTreeMap::new();
        let e = Expr {
            task_id: "ghost".into(),
            output_key: "any".into(),
        };
        assert!(matches!(e.resolve(&outs), Err(ExprError::NoSuchUpstream(_))));
    }

    #[test]
    fn resolve_errors_when_output_key_absent() {
        let mut outs = BTreeMap::new();
        outs.insert("foo".into(), BTreeMap::new());
        let e = Expr {
            task_id: "foo".into(),
            output_key: "missing".into(),
        };
        assert!(matches!(
            e.resolve(&outs),
            Err(ExprError::Missing { .. })
        ));
    }

    #[test]
    fn resolve_inputs_passes_through_non_string_values() {
        let mut raw = BTreeMap::new();
        raw.insert("count".into(), json!(42));
        raw.insert("ok".into(), json!(true));
        let outs = BTreeMap::new();
        let resolved = resolve_inputs(&raw, &outs).unwrap();
        assert_eq!(resolved.get("count"), Some(&json!(42)));
        assert_eq!(resolved.get("ok"), Some(&json!(true)));
    }

    #[test]
    fn resolve_inputs_substitutes_expressions() {
        let mut raw = BTreeMap::new();
        raw.insert("greeting".into(), json!("${tasks.a.out.echo}"));
        raw.insert("literal".into(), json!("plain"));
        let mut outs = BTreeMap::new();
        let mut a_outs = BTreeMap::new();
        a_outs.insert("echo".into(), json!("howdy"));
        outs.insert("a".into(), a_outs);
        let resolved = resolve_inputs(&raw, &outs).unwrap();
        assert_eq!(resolved.get("greeting"), Some(&json!("howdy")));
        assert_eq!(resolved.get("literal"), Some(&json!("plain")));
    }
}
