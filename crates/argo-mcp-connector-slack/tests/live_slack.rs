use argo_mcp::{CallToolParams, McpContent, McpServerService};
use argo_mcp_connector_slack::{SlackClient, SlackService};

#[tokio::test]
async fn live_post_message_hits_test_channel_when_env_present() {
    let token = match std::env::var("SLACK_BOT_TOKEN") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return,
    };
    let channel = match std::env::var("SLACK_TEST_CHANNEL") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return,
    };
    let client = match SlackClient::with_token(token) {
        Ok(client) => client,
        Err(_) => return,
    };
    let service = SlackService::new(client);
    let result = service
        .call_tool(CallToolParams {
            name: "slack.post_message".into(),
            arguments: serde_json::json!({
                "channel": channel,
                "text": "argo-mcp-connector-slack live integration test"
            }),
        })
        .await
        .expect("Slack live post_message should succeed");
    assert!(!result.is_error);
    assert!(matches!(
        result.content.first(),
        Some(McpContent::Text { .. })
    ));
}
