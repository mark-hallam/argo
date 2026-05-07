//! `argo plan show` handler — render a previously-generated YAML
//! + reasoning sidecar.

use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

pub async fn run(target: &str, json: bool) -> ArgoResult<()> {
    let yaml_path = resolve_target(target)?;
    if !yaml_path.exists() {
        return Err(ArgoError::Provider(format!(
            "plan not found at {}",
            yaml_path.display()
        )));
    }
    let yaml = std::fs::read_to_string(&yaml_path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", yaml_path.display())))?;
    let workflow = argo_workflow::parse_yaml_str(&yaml)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", yaml_path.display())))?;
    let reasoning = read_sidecar(&yaml_path);

    if json {
        let envelope = serde_json::json!({
            "path": yaml_path.display().to_string(),
            "tasks": workflow.tasks.len(),
            "edges": workflow.edges.len(),
            "reasoning": reasoning,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else {
        println!("plan: {}", yaml_path.display());
        println!("tasks: {}", workflow.tasks.len());
        println!("edges: {}", workflow.edges.len());
        if let Some(r) = reasoning {
            println!("reasoning:\n{r}");
        }
        println!("---");
        print!("{yaml}");
    }
    Ok(())
}

fn resolve_target(target: &str) -> ArgoResult<PathBuf> {
    let direct = PathBuf::from(target);
    if direct.exists() {
        return Ok(direct);
    }
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let candidate = cwd
        .join(".argo")
        .join("plan_runs")
        .join(format!("{target}.yaml"));
    Ok(candidate)
}

fn read_sidecar(yaml_path: &Path) -> Option<String> {
    let mut sidecar = yaml_path.to_path_buf();
    let new_ext = match yaml_path.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{ext}.reasoning"),
        None => "reasoning".to_string(),
    };
    sidecar.set_extension(new_ext);
    std::fs::read_to_string(&sidecar).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_sidecar_returns_none_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = dir.path().join("plan.yaml");
        std::fs::write(&yaml, "id: t").unwrap();
        assert!(read_sidecar(&yaml).is_none());
    }

    #[test]
    fn read_sidecar_returns_body_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = dir.path().join("plan.yaml");
        std::fs::write(&yaml, "id: t").unwrap();
        std::fs::write(dir.path().join("plan.yaml.reasoning"), "because").unwrap();
        assert_eq!(read_sidecar(&yaml).as_deref(), Some("because"));
    }
}
