use argo_mcp::{CallToolParams, McpServerService};
use argo_mcp_connector_gcal::{GcalClient, GcalConfig, GcalMcpService};

#[tokio::test]
async fn lists_events_when_gcal_test_token_is_present() {
    let token = match std::env::var("GCAL_TEST_TOKEN") {
        Ok(token) => token,
        Err(_) => return,
    };
    let service = GcalMcpService::new(GcalClient::new(GcalConfig {
        access_token: token,
        calendar_id: std::env::var("GCAL_CALENDAR_ID").unwrap_or_else(|_| "primary".into()),
        base_url: "https://www.googleapis.com/calendar/v3".into(),
    }));
    let result = service
        .call_tool(CallToolParams {
            name: "gcal.list_events".into(),
            arguments: serde_json::json!({"max_results": 1}),
        })
        .await
        .expect("gcal.list_events should call Calendar API");
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}
