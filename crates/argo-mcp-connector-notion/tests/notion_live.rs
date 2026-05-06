use argo_mcp::{CallToolParams, McpServerService};
use argo_mcp_connector_notion::{NotionClient, NotionConfig, NotionMcpService};

#[tokio::test]
async fn searches_when_notion_test_token_is_present() {
    let token = match std::env::var("NOTION_TEST_TOKEN") {
        Ok(token) => token,
        Err(_) => return,
    };
    let service = NotionMcpService::new(NotionClient::new(NotionConfig {
        integration_token: token,
        base_url: "https://api.notion.com/v1".into(),
        notion_version: "2022-06-28".into(),
    }));
    let result = service
        .call_tool(CallToolParams {
            name: "notion.search".into(),
            arguments: serde_json::json!({"page_size": 1}),
        })
        .await
        .expect("notion.search should call Notion API");
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}
