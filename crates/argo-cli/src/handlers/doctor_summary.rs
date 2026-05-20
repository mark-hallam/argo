#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SummaryState {
    Ok,
    Degraded,
    Fail,
}

impl SummaryState {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Degraded => "DEGRADED",
            Self::Fail => "FAIL",
        }
    }

    pub(crate) fn exit_code(self) -> i32 {
        match self {
            Self::Ok => 0,
            Self::Degraded => 1,
            Self::Fail => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DoctorSummary {
    pub(crate) state: SummaryState,
    pub(crate) passed: usize,
    pub(crate) warned: usize,
    pub(crate) failed: usize,
    issue: Option<&'static str>,
}

impl DoctorSummary {
    pub(crate) fn line(&self) -> String {
        let total = self.passed + self.warned + self.failed;
        let base = format!("{} {}/{} checks", self.state.label(), self.passed, total);
        match (self.issue, self.warned, self.failed) {
            (Some(issue), warned, 0) if warned > 0 => format!("{base} ({warned} warn: {issue})"),
            (Some(issue), _, failed) if failed > 0 => format!("{base} ({failed} fail: {issue})"),
            _ => base,
        }
    }

    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "state": self.state.label(),
            "passed": self.passed,
            "warned": self.warned,
            "failed": self.failed,
        })
    }
}

pub(crate) fn summarize(v: &serde_json::Value) -> DoctorSummary {
    let checks = [
        (
            "argo_toml",
            yes(
                v["argo_toml"]["ok"].as_bool().unwrap_or(false),
                SummaryState::Fail,
            ),
        ),
        (
            "argo_home",
            yes(
                v["argo_home"]["ok"].as_bool().unwrap_or(false),
                SummaryState::Degraded,
            ),
        ),
        (
            "bundle_dirs",
            yes(
                v["bundle_dirs_missing"]
                    .as_array()
                    .is_none_or(Vec::is_empty),
                SummaryState::Degraded,
            ),
        ),
        (
            "memory_files",
            yes(
                v["memory_files_missing"]
                    .as_array()
                    .is_none_or(Vec::is_empty),
                SummaryState::Degraded,
            ),
        ),
        ("db", db_state(&v["db"])),
        ("audit", audit_state(&v["audit"])),
        ("cron", SummaryState::Ok),
        (
            "creds",
            yes(
                v["creds"]["set"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty()),
                SummaryState::Degraded,
            ),
        ),
        (
            "gateway",
            yes(
                v["gateway"]["running"].as_bool().unwrap_or(false),
                SummaryState::Degraded,
            ),
        ),
        ("gateway_health", gateway_health_state(v)),
        (
            "routes",
            yes(
                v["routes"]["total"].as_u64().unwrap_or(0) > 0,
                SummaryState::Fail,
            ),
        ),
        ("auto_approval", status_group_state(&v["auto_approval"])),
        ("channels", status_group_state(&v["channels"])),
        ("network", network_state(&v["network_checks"])),
        ("audit_chain", status_item_state(&v["audit_chain"])),
        ("storage", status_item_state(&v["storage"])),
        ("workers", status_item_state(&v["workers"])),
    ];
    let mut out = DoctorSummary {
        state: SummaryState::Ok,
        passed: 0,
        warned: 0,
        failed: 0,
        issue: None,
    };
    for (name, state) in checks {
        match state {
            SummaryState::Ok => out.passed += 1,
            SummaryState::Degraded => out.warned += 1,
            SummaryState::Fail => out.failed += 1,
        }
        if state > out.state {
            out.state = state;
            out.issue = Some(name);
        }
    }
    out
}

/// Render the "Build Info" block printed above the existing health
/// checks. Silently no-ops if the payload lacks `build_info`.
pub(crate) fn print_build_info(v: &serde_json::Value) {
    if !v.is_object() {
        return;
    }
    let version = v["version"].as_str().unwrap_or("?");
    let git_sha = v["git_sha"].as_str().unwrap_or("?");
    let git_date = v["git_date"].as_str().unwrap_or("?");
    let target = v["target"].as_str().unwrap_or("?");
    let rustc = v["rustc"].as_str().unwrap_or("?");
    let profile = v["profile"].as_str().unwrap_or("?");
    let binary_path = v["binary_path"].as_str().unwrap_or("?");
    let features: Vec<&str> = v["features"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    let features_str = if features.is_empty() {
        "(none)".to_string()
    } else {
        features.join(",")
    };
    println!("argo doctor — Build Info");
    println!("  version:  {version} ({git_sha} {git_date})");
    println!("  target:   {target}");
    println!("  rustc:    {rustc}");
    println!("  profile:  {profile}");
    println!("  binary:   {binary_path}");
    println!("  features: {features_str}");
}

fn yes(ok: bool, otherwise: SummaryState) -> SummaryState {
    if ok {
        SummaryState::Ok
    } else {
        otherwise
    }
}

fn db_state(v: &serde_json::Value) -> SummaryState {
    if v["open_error"].is_string() || v["migrate_error"].is_string() {
        SummaryState::Fail
    } else if !v["present"].as_bool().unwrap_or(false)
        || !v["workspace_registered"].as_bool().unwrap_or(false)
    {
        SummaryState::Degraded
    } else {
        SummaryState::Ok
    }
}

fn audit_state(v: &serde_json::Value) -> SummaryState {
    if !v["available"].as_bool().unwrap_or(false) {
        SummaryState::Degraded
    } else {
        yes(v["ok"].as_bool().unwrap_or(false), SummaryState::Fail)
    }
}

fn gateway_health_state(v: &serde_json::Value) -> SummaryState {
    if !v["gateway"]["running"].as_bool().unwrap_or(false) {
        return SummaryState::Degraded;
    }
    yes(
        v["gateway_health"]["reachable"].as_bool().unwrap_or(false)
            && v["gateway_health"]["status_code"].as_u64().unwrap_or(0) < 500,
        SummaryState::Fail,
    )
}

fn status_group_state(v: &serde_json::Value) -> SummaryState {
    let Some(obj) = v.as_object() else {
        return SummaryState::Degraded;
    };
    let mut state = SummaryState::Ok;
    for status in obj
        .values()
        .filter_map(|item| item.get("status").and_then(|s| s.as_str()))
    {
        match status {
            "ERROR" | "FAIL" | "BROKEN" => return SummaryState::Fail,
            "WARN" | "MISSING" => state = SummaryState::Degraded,
            _ => {}
        }
    }
    state
}

fn status_item_state(v: &serde_json::Value) -> SummaryState {
    match v["status"].as_str().unwrap_or("WARN") {
        "OK" | "SKIP" => SummaryState::Ok,
        "FAIL" | "ERROR" | "BROKEN" => SummaryState::Fail,
        _ => SummaryState::Degraded,
    }
}

fn network_state(v: &serde_json::Value) -> SummaryState {
    if v["skipped"].as_bool().unwrap_or(false) {
        return SummaryState::Ok;
    }
    let failures = ["dns", "tls", "auth"]
        .into_iter()
        .filter_map(|key| v[key].as_array())
        .flat_map(|items| items.iter())
        .filter(|item| item["status"].as_str() == Some("FAIL"))
        .count();
    match failures {
        0 => SummaryState::Ok,
        1 => SummaryState::Degraded,
        _ => SummaryState::Fail,
    }
}
