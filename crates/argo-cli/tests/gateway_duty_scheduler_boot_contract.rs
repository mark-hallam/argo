#[test]
fn gateway_boot_wires_duty_scheduler_runner_sink_and_message() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/handlers/gateway/boot_spawns.rs"),
    )
    .expect("read boot_spawns.rs");

    assert!(
        source.contains("GatewayDutyAgentRunner::new"),
        "gateway boot must construct the real duty agent runner"
    );
    assert!(
        source.contains(".with_agent_runner(runner)"),
        "gateway duty scheduler must inject a runner"
    );
    assert!(
        source.contains(".with_audit_sink(sink)"),
        "gateway duty scheduler must inject an audit sink"
    );
    assert!(
        source.contains("gate=closed, runner=wired, sink=wired"),
        "startup message must expose the closed gate plus wired runner/sink"
    );
}
