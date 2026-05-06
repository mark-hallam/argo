//! Rendering helpers for the guided `argo setup` wizard.
use super::setup_steps::{SetupStep, StepStatus, GENESIS_STEP};
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;

#[derive(Serialize)]
struct SetupEnvelope<'a> {
    steps: &'a [SetupStep],
    ready: bool,
}

pub fn print(steps: &[SetupStep], json: bool) -> ArgoResult<()> {
    if json {
        let serialized = serde_json::to_string(&envelope(steps))
            .map_err(|e| ArgoError::Provider(format!("serialize setup: {e}")))?;
        println!("{serialized}");
        return Ok(());
    }
    print_table(steps);
    Ok(())
}

pub fn ready(steps: &[SetupStep]) -> bool {
    steps.iter().all(|step| {
        step.status == StepStatus::Ok
            || (step.name == "Doctor invariants" && step.status == StepStatus::Skip)
    })
}

fn envelope(steps: &[SetupStep]) -> SetupEnvelope<'_> {
    SetupEnvelope {
        steps,
        ready: ready(steps),
    }
}

fn print_table(steps: &[SetupStep]) {
    println!("argo setup wizard");
    println!("=================");
    println!("{:<3} {:<32} {:<8} detail", "#", "step", "status");
    for (idx, step) in steps.iter().enumerate() {
        println!(
            "{:<3} {:<32} {:<8} {}",
            idx + 1,
            step.name,
            status_label(step.status),
            step.detail
        );
        if step.name == GENESIS_STEP {
            print_genesis_suggestion_table();
        }
    }
    println!();
    println!("ready: {}", ready(steps));
}

fn print_genesis_suggestion_table() {
    println!("      archetype                 command");
    for archetype in argo_genesis::archetype::list_archetypes().into_iter().take(3) {
        println!(
            "      {:<25} argo genesis suggest --charter \"{}\"",
            archetype.name, archetype.description
        );
    }
}

fn status_label(status: StepStatus) -> &'static str {
    match status {
        StepStatus::Ok => "OK ✓",
        StepStatus::Fail => "FAIL ✗",
        StepStatus::Skip => "SKIP",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_is_false_when_any_step_fails() {
        let steps = vec![
            SetupStep {
                name: "one".into(),
                status: StepStatus::Ok,
                detail: "done".into(),
            },
            SetupStep {
                name: "two".into(),
                status: StepStatus::Fail,
                detail: "missing".into(),
            },
        ];
        assert!(!ready(&steps));
    }

    #[test]
    fn json_envelope_has_steps_and_ready() {
        let steps = vec![SetupStep {
            name: "one".into(),
            status: StepStatus::Ok,
            detail: "done".into(),
        }];
        let value = serde_json::to_value(envelope(&steps)).unwrap();
        assert_eq!(value["ready"], true);
        assert_eq!(value["steps"][0]["status"], "ok");
    }
}
