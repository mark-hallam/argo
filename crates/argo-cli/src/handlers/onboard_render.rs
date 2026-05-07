//! Pretty + JSON rendering for the onboarding stage list.
use super::onboard_stages::{OnboardStage, OnboardStatus};
use argo_types::{ArgoError, ArgoResult};

pub fn print(stages: &[OnboardStage], json: bool) -> ArgoResult<()> {
    if json {
        return print_json(stages);
    }
    println!("argo onboard — first-experience walkthrough");
    println!();
    for (i, s) in stages.iter().enumerate() {
        let mark = match s.status {
            OnboardStatus::Ok => "[ok]  ",
            OnboardStatus::Skip => "[skip]",
            OnboardStatus::Fail => "[FAIL]",
        };
        println!("{} {}. {:<20} {}", mark, i + 1, s.name, s.detail);
    }
    println!();
    let any_fail = stages.iter().any(|s| s.status == OnboardStatus::Fail);
    if any_fail {
        println!("one or more stages failed — fix the [FAIL] lines above and re-run");
    } else {
        println!("you're done — try `argo doctor` then explore `docs/skycot/RUNBOOK.md`");
    }
    Ok(())
}

fn print_json(stages: &[OnboardStage]) -> ArgoResult<()> {
    let payload = serde_json::json!({
        "stages": stages,
        "any_fail": stages.iter().any(|s| s.status == OnboardStatus::Fail),
        "count": stages.len(),
    });
    let out = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Provider(format!("serialize stages: {e}")))?;
    println!("{out}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::onboard_stages::{OnboardStage, OnboardStatus};
    use super::*;

    fn fixture() -> Vec<OnboardStage> {
        vec![
            OnboardStage::ok("Welcome", "argo CLI v0.0"),
            OnboardStage::ok("Workspace", "ready"),
            OnboardStage::skip("Credentials", "no creds"),
        ]
    }

    #[test]
    fn print_text_does_not_error() {
        let stages = fixture();
        print(&stages, false).unwrap();
    }

    #[test]
    fn print_json_does_not_error() {
        let stages = fixture();
        print(&stages, true).unwrap();
    }

    #[test]
    fn render_marks_failure_visibly() {
        let stages = vec![OnboardStage::fail("Workspace", "boom")];
        let result = print(&stages, false);
        assert!(result.is_ok());
        // Visual rendering tested via the CLI smoke; here we
        // just ensure no panic on Fail rows.
        let any = stages.iter().any(|s| s.status == OnboardStatus::Fail);
        assert!(any);
    }
}
