use argo_mcp::{CallToolParams, McpServerService};
use argo_mcp_connector_hubspot::HubspotService;

#[tokio::test]
async fn live_search_and_pipeline_when_token_is_set() {
    if std::env::var("HUBSPOT_PRIVATE_APP_TOKEN").is_err() {
        eprintln!("skipping live HubSpot test: HUBSPOT_PRIVATE_APP_TOKEN unset");
        return;
    }
    let svc = HubspotService;
    for (name, args) in [
        (
            "hubspot.search_contacts",
            serde_json::json!({"limit": 1, "properties": ["email", "firstname", "lastname"]}),
        ),
        (
            "hubspot.get_pipeline",
            serde_json::json!({"object_type": "deals"}),
        ),
    ] {
        let result = svc
            .call_tool(CallToolParams {
                name: name.into(),
                arguments: args,
            })
            .await
            .expect("mcp call");
        assert!(
            !result.is_error,
            "{name} returned error: {:?}",
            result.content
        );
    }
}

#[tokio::test]
async fn live_mutations_when_explicitly_enabled() {
    if std::env::var("HUBSPOT_PRIVATE_APP_TOKEN").is_err()
        || std::env::var("HUBSPOT_TEST_MUTATE").as_deref() != Ok("1")
    {
        eprintln!("skipping live HubSpot mutation test");
        return;
    }
    let svc = HubspotService;
    let suffix = unique_suffix();
    let calls = [
        (
            "hubspot.create_contact",
            serde_json::json!({"properties": {"email": format!("argo-test-{suffix}@example.com")}}),
        ),
        (
            "hubspot.create_deal",
            serde_json::json!({"properties": {"dealname": format!("Argo test {suffix}")}}),
        ),
        (
            "hubspot.log_activity",
            serde_json::json!({"note_body": format!("Argo MCP connector smoke test {suffix}")}),
        ),
    ];
    for (name, arguments) in calls {
        let result = svc
            .call_tool(CallToolParams {
                name: name.into(),
                arguments,
            })
            .await
            .expect("mcp call");
        assert!(
            !result.is_error,
            "{name} returned error: {:?}",
            result.content
        );
    }
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
