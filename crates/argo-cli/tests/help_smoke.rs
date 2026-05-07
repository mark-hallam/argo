//! Smoke tests for the `--help` surface.
//!
//! Catches regressions where a subcommand goes missing from clap's
//! generated help (which is what users see when they type
//! `argo --help`) — particularly relevant after the cli.rs ↔
//! cli_subcommands.rs split landed in #122.
use std::process::Command;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn help(args: &[&str]) -> String {
    let out = Command::new(argo_bin())
        .args(args)
        .args(["--help"])
        .output()
        .unwrap_or_else(|e| panic!("spawn {args:?} --help: {e}"));
    assert!(
        out.status.success(),
        "{args:?} --help exited non-zero: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("help is utf-8")
}

#[test]
fn root_help_lists_every_top_level_subcommand() {
    let h = help(&[]);
    // Pin a representative slice — full list is in cli.rs.
    for cmd in [
        "audit",
        "cron",
        "costs",
        "db",
        "doctor",
        "memory",
        "provider",
        "session",
        "skill",
        "workspace",
    ] {
        assert!(
            h.contains(cmd),
            "`argo --help` should advertise '{cmd}', got:\n{h}"
        );
    }
}

#[test]
fn family_help_pages_render_without_crashing() {
    // One rep per Subcommand enum lifted into cli_subcommands.rs.
    for family in [
        "agents",
        "audit",
        "cron",
        "costs",
        "db",
        "eval",
        "forge",
        "gateway",
        "judge",
        "mcp",
        "memory",
        "model",
        "plugin",
        "provider",
        "route",
        "session",
        "skill",
        "workspace",
    ] {
        let h = help(&[family]);
        assert!(
            !h.is_empty(),
            "`argo {family} --help` produced empty stdout"
        );
        // clap always emits `Usage:` somewhere in the help body.
        assert!(
            h.contains("Usage:"),
            "`argo {family} --help` lacks 'Usage:' header:\n{h}"
        );
    }
}

#[test]
fn new_session_subcommands_render_help() {
    // Pins the recently-shipped session subcommands so a clap
    // wiring regression (missing variant, mis-extracted enum)
    // surfaces immediately.
    for sub in [
        "messages",
        "title",
        "clear-skills",
        "show",
        "list",
        "diff",
        "search",
        "export",
        "replay",
    ] {
        let h = help(&["session", sub]);
        assert!(
            h.contains("Usage:"),
            "`argo session {sub} --help` lacks 'Usage:' header:\n{h}"
        );
    }
}

#[test]
fn format_markdown_flag_pins() {
    // Pins the markdown-format family across the read-side
    // surfaces. Catches regressions where a `--format` flag
    // gets dropped from a clap variant.
    for args in [
        &["session", "list"][..],
        &["session", "show"][..],
        &["session", "diff"][..],
        &["session", "messages"][..],
        &["session", "search"][..],
        &["session", "export"][..],
        &["skill", "list"][..],
        &["memory", "list"][..],
        &["forge", "runs", "list"][..],
        &["audit", "show"][..],
        &["costs", "show"][..],
        &["doctor"][..],
        &["health"][..],
        &["tools", "list"][..],
    ] {
        let h = help(args);
        assert!(
            h.contains("--format"),
            "`argo {} --help` is missing --format:\n{h}",
            args.join(" ")
        );
    }
}

#[test]
fn forge_runs_abort_all_renders_help() {
    let h = help(&["forge", "runs", "abort-all"]);
    assert!(
        h.contains("Usage:"),
        "`argo forge runs abort-all --help` lacks 'Usage:' header:\n{h}"
    );
}

#[test]
fn mcp_tools_renders_help() {
    // Pins claude-mcp-tool-discovery-vis — `argo mcp tools` for
    // discovering tools an MCP server exposes.
    let h = help(&["mcp", "tools"]);
    assert!(h.contains("Usage:"), "mcp tools --help: {h}");
    assert!(h.contains("--vertical"), "mcp tools missing --vertical: {h}");
    assert!(h.contains("--filter"), "mcp tools missing --filter: {h}");
    assert!(
        h.contains("--schema-only"),
        "mcp tools missing --schema-only: {h}"
    );
}

#[test]
fn tools_list_show_render_help() {
    // Pins #525 — the new `argo tools list / show` surface.
    let h = help(&["tools", "list"]);
    assert!(h.contains("Usage:"), "tools list --help: {h}");
    assert!(h.contains("--format"), "tools list missing --format: {h}");
    assert!(
        h.contains("--name-contains"),
        "tools list missing --name-contains: {h}"
    );

    let h = help(&["tools", "show"]);
    assert!(h.contains("Usage:"), "tools show --help: {h}");
    assert!(h.contains("<NAME>"), "tools show missing <NAME>: {h}");
}

#[test]
fn forge_runs_sweep_renders_help_and_advertises_older_than_minutes() {
    // Pins #520 — the sweep subcommand + its --older-than-minutes
    // flag (default 60). Catches a clap regression where the
    // variant gets dropped or the flag is removed.
    let h = help(&["forge", "runs", "sweep"]);
    assert!(
        h.contains("Usage:"),
        "`argo forge runs sweep --help` lacks 'Usage:' header:\n{h}"
    );
    assert!(
        h.contains("--older-than-minutes"),
        "`argo forge runs sweep --help` is missing --older-than-minutes:\n{h}"
    );
}

#[test]
fn version_flag_round_trips_through_main() {
    let out = Command::new(argo_bin())
        .arg("--version")
        .output()
        .expect("spawn argo --version");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("argo "));
}
