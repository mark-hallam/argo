//! `argo vertical lint <id>` — best-practice hygiene checks
//! beyond the structural validator (#671).
//!
//! `validate` (#650) catches things the runtime *can't tolerate*:
//! malformed schema, missing referenced files, allow/deny
//! conflicts. Lint catches things the runtime *will tolerate but
//! the operator probably didn't mean*: typo'd tool names that
//! don't match any in-core tool, escalation entries that point at
//! tools the agent can never call (orphans), empty skill bundles
//! / eval suites, and other declarative mistakes a CI gate should
//! flag.
//!
//! Severity levels:
//!
//! - `error` — exit code 1; almost-certainly broken
//! - `warn`  — exit code 0; suspicious / likely-mistake
//! - `info`  — exit code 0; declarative completeness reminder
//!
//! Returning 0 on warn keeps lint from being a CI hard gate by
//! default; operators who want strict CI can grep the output for
//! `severity":"warn"` or use a future `--strict` flag.
use argo_tool::builtin_tools;
use argo_types::{ArgoError, ArgoResult};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    pub(crate) severity: Severity,
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Severity {
    Error,
    Warn,
    Info,
}

impl Severity {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
        }
    }
}

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errs) = argo_vertical::load_dir(&dir)?;
    let loaded = registry.get(id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{id}' not found in {} — run `argo vertical list` to see what's loaded",
            dir.display()
        ))
    })?;
    let findings = lint_loaded(loaded);

    if json {
        let arr: Vec<serde_json::Value> = findings
            .iter()
            .map(|f| {
                serde_json::json!({
                    "severity": f.severity.label(),
                    "code": f.code,
                    "message": f.message,
                })
            })
            .collect();
        let any_error = findings.iter().any(|f| f.severity == Severity::Error);
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": !any_error,
                "id": id,
                "findings": arr,
            }))
            .unwrap_or_default()
        );
    } else if findings.is_empty() {
        println!("vertical '{id}': lint clean");
    } else {
        println!("vertical '{id}' — {} finding(s):", findings.len());
        for f in &findings {
            println!("  [{}] {} — {}", f.severity.label(), f.code, f.message);
        }
    }
    if findings.iter().any(|f| f.severity == Severity::Error) {
        return Err(ArgoError::Provider(format!(
            "vertical '{id}' lint failed: {} error finding(s)",
            findings.iter().filter(|f| f.severity == Severity::Error).count()
        )));
    }
    Ok(())
}

pub(crate) fn lint_loaded(loaded: &argo_vertical::LoadedVertical) -> Vec<Finding> {
    let mut out = Vec::new();
    let m = &loaded.manifest;

    // Built-in tool name set, sourced live from argo_tool so
    // this stays correct as the in-core tool surface evolves.
    let known_tools: HashSet<String> = builtin_tools(PathBuf::from("/"))
        .iter()
        .map(|t| t.name().to_string())
        .collect();

    if let Some(tools) = &m.tools {
        for t in &tools.allowed {
            if !known_tools.contains(t.as_str()) {
                out.push(Finding {
                    severity: Severity::Warn,
                    code: "tools.allowed.unknown_name",
                    message: format!(
                        "tool '{t}' in [tools].allowed is not a known built-in tool — typo? known: {}",
                        sorted_csv(&known_tools)
                    ),
                });
            }
        }
        for t in &tools.denied {
            if !known_tools.contains(t.as_str()) {
                out.push(Finding {
                    severity: Severity::Warn,
                    code: "tools.denied.unknown_name",
                    message: format!(
                        "tool '{t}' in [tools].denied is not a known built-in tool — typo?"
                    ),
                });
            }
        }
    }

    if let Some(esc) = &m.escalation {
        let allowed: HashSet<&str> = m
            .tools
            .as_ref()
            .map(|t| t.allowed.iter().map(String::as_str).collect())
            .unwrap_or_default();
        let denied: HashSet<&str> = m
            .tools
            .as_ref()
            .map(|t| t.denied.iter().map(String::as_str).collect())
            .unwrap_or_default();
        for t in &esc.human_approval_required {
            // Orphan: gate fires on a tool that's neither
            // explicitly allowed nor explicitly denied. The
            // agent can't call it (not in allowed) AND it's not
            // proactively blocked (not in denied) — the gate is
            // dead code. Could be intentional defence-in-depth
            // but more likely a forgotten cleanup.
            if !allowed.contains(t.as_str()) && !denied.contains(t.as_str()) {
                out.push(Finding {
                    severity: Severity::Warn,
                    code: "escalation.orphan_tool",
                    message: format!(
                        "tool '{t}' in [escalation].human_approval_required is in neither [tools].allowed nor [tools].denied — gate may be dead code"
                    ),
                });
            }
            if !known_tools.contains(t.as_str()) {
                out.push(Finding {
                    severity: Severity::Warn,
                    code: "escalation.unknown_tool",
                    message: format!(
                        "tool '{t}' in [escalation].human_approval_required is not a known built-in tool — typo?"
                    ),
                });
            }
        }
    }

    if let Some(skills) = &m.skills {
        let bundle_dir = loaded.root.join(&skills.bundle);
        let count = std::fs::read_dir(&bundle_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| {
                        e.path()
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.ends_with(".skill.md"))
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0);
        if count == 0 {
            out.push(Finding {
                severity: Severity::Info,
                code: "skills.bundle_empty",
                message: format!(
                    "[skills].bundle = '{}' contains no `*.skill.md` files",
                    skills.bundle
                ),
            });
        }
    }

    if let Some(eval) = &m.evaluation {
        if eval.suites.is_empty() {
            out.push(Finding {
                severity: Severity::Info,
                code: "evaluation.no_suites",
                message: "[evaluation] is declared but [evaluation].suites is empty".into(),
            });
        }
    }

    if let Some(mem) = &m.memory {
        if let Some(schema_rel) = &mem.schema {
            // Parse the schema; if it has zero entries, the
            // operator either hasn't typed any rows yet or the
            // schema is documentation-only. Info, not warn.
            if let Ok(src) = std::fs::read_to_string(loaded.root.join(schema_rel)) {
                if let Ok(s) = argo_vertical::parse_memory_schema(&src) {
                    if s.entries.is_empty() {
                        out.push(Finding {
                            severity: Severity::Info,
                            code: "memory.schema_no_entries",
                            message: format!(
                                "[memory].schema = '{schema_rel}' declares zero typed entries"
                            ),
                        });
                    }
                }
            }
        }
    }

    out
}

fn sorted_csv(set: &HashSet<String>) -> String {
    let mut v: Vec<&str> = set.iter().map(String::as_str).collect();
    v.sort_unstable();
    v.join(", ")
}

#[cfg(test)]
#[path = "vertical_lint_tests.rs"]
mod tests;
