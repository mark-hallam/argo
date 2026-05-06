use argo_mcp::{CallToolParams, McpContent, McpServerService};
use argo_mcp_connector_gmail::GmailService;

#[tokio::test]
async fn live_gmail_tools_when_token_is_set() {
    let Ok(token) = std::env::var("GMAIL_TEST_TOKEN") else {
        eprintln!("skipping live Gmail test: GMAIL_TEST_TOKEN unset");
        return;
    };
    let svc = GmailService::with_access_token_for_tests(token);
    let search = call(&svc, "gmail.search", serde_json::json!({"limit": 1})).await;
    let first_id = search
        .get("messages")
        .and_then(|v| v.as_array())
        .and_then(|messages| messages.first())
        .and_then(|message| message.get("id"))
        .and_then(|id| id.as_str())
        .map(str::to_string);
    if let Some(id) = first_id {
        let _ = call(
            &svc,
            "gmail.read",
            serde_json::json!({"id": id, "format": "metadata"}),
        )
        .await;
        let _ = call(
            &svc,
            "gmail.label",
            serde_json::json!({"id": id, "add_label_ids": ["STARRED"], "remove_label_ids": ["STARRED"]}),
        )
        .await;
    }
    let Ok(to) = std::env::var("GMAIL_TEST_TO") else {
        eprintln!("skipping live Gmail draft/send mutation: GMAIL_TEST_TO unset");
        return;
    };
    let suffix = unique_suffix();
    let draft = compose(&to, &format!("Argo Gmail draft {suffix}"));
    let _ = call(&svc, "gmail.draft", draft).await;
    let send = compose(&to, &format!("Argo Gmail send {suffix}"));
    let _ = call(&svc, "gmail.send", send).await;
}

async fn call(svc: &GmailService, name: &str, arguments: serde_json::Value) -> serde_json::Value {
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
    match &result.content[0] {
        McpContent::Text { text } => serde_json::from_str(text).expect("json tool result"),
        _ => panic!("expected text"),
    }
}

fn compose(to: &str, subject: &str) -> serde_json::Value {
    serde_json::json!({
        "to": to,
        "subject": subject,
        "body_text": "Automated Argo Gmail MCP connector smoke test."
    })
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
