//! Browser MCP service — 12 spec tools per §28.AA Hermes-parity:
//! navigate, click, fill, submit, screenshot, read_page,
//! extract_links, wait_for, scroll, eval, cookies, pdf — plus
//! 2 HTTP-only diagnostic probes (list_tabs, chrome_version).
//!
//! All 14 are wired against the user's running Chrome via raw CDP
//! (HTTP `/json*` + WebSocket `Runtime.evaluate` / `Page.*` /
//! `Network.*`). Browserbase / BrowserUse cloud backends are still
//! opt-in via server.toml and not wired today.
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

const NOT_WIRED: &str = "browser MCP backend not yet wired — chromiumoxide + local Playwright \
    land per §12.2.1; Browserbase / BrowserUse cloud backends are opt-in via server.toml.";

const TOOL_NAMES: &[&str] = &[
    "navigate",
    "click",
    "fill",
    "submit",
    "screenshot",
    "read_page",
    "extract_links",
    "wait_for",
    "scroll",
    "eval",
    "cookies",
    "pdf",
];

/// Tools we wire today (the 12 spec tools still need chromiumoxide;
/// these are thin HTTP probes against Chrome's `/json*` endpoints so
/// the user can verify their attach setup before driving anything).
const HTTP_ONLY_TOOLS: &[&str] = &["list_tabs", "chrome_version"];

/// Browser MCP service.
pub struct BrowserService;

#[async_trait]
impl McpServerService for BrowserService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-browser".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        use crate::schemas::*;
        use crate::schemas_session::*;
        vec![
            navigate_schema(),
            click_schema(),
            fill_schema(),
            submit_schema(),
            screenshot_schema(),
            read_page_schema(),
            extract_links_schema(),
            wait_for_schema(),
            scroll_schema(),
            eval_schema(),
            cookies_schema(),
            pdf_schema(),
            list_tabs_schema(),
            chrome_version_schema(),
            session_open_schema(),
            session_close_schema(),
            session_eval_schema(),
            session_navigate_schema(),
            session_list_schema(),
            session_read_page_schema(),
            session_click_schema(),
            session_fill_schema(),
            session_submit_schema(),
            session_extract_links_schema(),
            session_screenshot_schema(),
            session_scroll_schema(),
            session_wait_for_schema(),
            session_pdf_schema(),
            session_cookies_schema(),
            session_get_url_schema(),
            session_reap_schema(),
            cloud_browserbase_open_schema(),
            cloud_browserbase_close_schema(),
        ]
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        if params.name.as_str() == "list_tabs" {
            return Ok(crate::cdp::list_tabs(&params.arguments).await);
        }
        if params.name.as_str() == "chrome_version" {
            return Ok(crate::cdp::chrome_version(&params.arguments).await);
        }
        if params.name.as_str() == "eval" {
            return Ok(crate::cdp::eval(&params.arguments).await);
        }
        if params.name.as_str() == "screenshot" {
            return Ok(crate::cdp::screenshot(&params.arguments).await);
        }
        if params.name.as_str() == "read_page" {
            return Ok(crate::cdp::read_page(&params.arguments).await);
        }
        if params.name.as_str() == "extract_links" {
            return Ok(crate::cdp::extract_links(&params.arguments).await);
        }
        if params.name.as_str() == "navigate" {
            return Ok(crate::cdp::navigate(&params.arguments).await);
        }
        if params.name.as_str() == "scroll" {
            return Ok(crate::cdp::scroll(&params.arguments).await);
        }
        if params.name.as_str() == "pdf" {
            return Ok(crate::cdp::pdf(&params.arguments).await);
        }
        if params.name.as_str() == "submit" {
            return Ok(crate::cdp::submit(&params.arguments).await);
        }
        if params.name.as_str() == "click" {
            return Ok(crate::cdp::click(&params.arguments).await);
        }
        if params.name.as_str() == "fill" {
            return Ok(crate::cdp::fill(&params.arguments).await);
        }
        if params.name.as_str() == "wait_for" {
            return Ok(crate::cdp::wait_for(&params.arguments).await);
        }
        if params.name.as_str() == "cookies" {
            return Ok(crate::cdp::cookies(&params.arguments).await);
        }
        if params.name.as_str() == "session_open" {
            return Ok(crate::cdp::session_open(&params.arguments).await);
        }
        if params.name.as_str() == "session_close" {
            return Ok(crate::cdp::session_close(&params.arguments).await);
        }
        if params.name.as_str() == "session_eval" {
            return Ok(crate::cdp::session_eval(&params.arguments).await);
        }
        if params.name.as_str() == "session_navigate" {
            return Ok(crate::cdp::session_navigate(&params.arguments).await);
        }
        if params.name.as_str() == "session_list" {
            return Ok(crate::cdp::session_list(&params.arguments).await);
        }
        if params.name.as_str() == "session_read_page" {
            return Ok(crate::cdp::session_read_page(&params.arguments).await);
        }
        if params.name.as_str() == "session_click" {
            return Ok(crate::cdp::session_click(&params.arguments).await);
        }
        if params.name.as_str() == "session_fill" {
            return Ok(crate::cdp::session_fill(&params.arguments).await);
        }
        if params.name.as_str() == "session_submit" {
            return Ok(crate::cdp::session_submit(&params.arguments).await);
        }
        if params.name.as_str() == "session_extract_links" {
            return Ok(crate::cdp::session_extract_links(&params.arguments).await);
        }
        if params.name.as_str() == "session_screenshot" {
            return Ok(crate::cdp::session_screenshot(&params.arguments).await);
        }
        if params.name.as_str() == "session_scroll" {
            return Ok(crate::cdp::session_scroll(&params.arguments).await);
        }
        if params.name.as_str() == "session_wait_for" {
            return Ok(crate::cdp::session_wait_for(&params.arguments).await);
        }
        if params.name.as_str() == "session_pdf" {
            return Ok(crate::cdp::session_pdf(&params.arguments).await);
        }
        if params.name.as_str() == "session_cookies" {
            return Ok(crate::cdp::session_cookies(&params.arguments).await);
        }
        if params.name.as_str() == "session_get_url" {
            return Ok(crate::cdp::session_get_url(&params.arguments).await);
        }
        if params.name.as_str() == "session_reap" {
            return Ok(crate::cdp::session_reap(&params.arguments).await);
        }
        if params.name.as_str() == "cloud_browserbase_open_session" {
            return Ok(crate::cloud_browserbase::open_session().await);
        }
        if params.name.as_str() == "cloud_browserbase_close_session" {
            let sid = params
                .arguments
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            return Ok(crate::cloud_browserbase::close_session(sid).await);
        }
        let known = TOOL_NAMES.contains(&params.name.as_str())
            || HTTP_ONLY_TOOLS.contains(&params.name.as_str());
        let text = if known {
            NOT_WIRED.to_string()
        } else {
            format!("unknown tool: {}", params.name)
        };
        Ok(CallToolResult {
            content: vec![McpContent::Text { text }],
            is_error: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_advertises_twelve_spec_tools_plus_http_probes() {
        let s = BrowserService;
        let tools = s.list_tools();
        assert_eq!(
            tools.len(),
            33,
            "12 spec tools + list_tabs + chrome_version + 17 session-* + 2 cloud_browserbase_* (#642)"
        );
    }

    #[tokio::test]
    async fn tool_names_match_spec() {
        let s = BrowserService;
        let tools = s.list_tools();
        for (i, expected) in TOOL_NAMES.iter().enumerate() {
            assert_eq!(tools[i].name, *expected, "position {i}");
        }
        // HTTP probes live at the end, after the 12 spec tools.
        assert_eq!(tools[TOOL_NAMES.len()].name, "list_tabs");
        assert_eq!(tools[TOOL_NAMES.len() + 1].name, "chrome_version");
    }

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let s = BrowserService;
        let r = s
            .call_tool(CallToolParams {
                name: "missing".into(),
                arguments: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert!(r.is_error);
        match &r.content[0] {
            McpContent::Text { text } => assert!(text.contains("unknown tool")),
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn unknown_tool_returns_unknown_error() {
        // All 14 spec tools are now wired. The dispatcher's
        // fallback branch only fires for unknown names.
        let s = BrowserService;
        let r = s
            .call_tool(CallToolParams {
                name: "definitely-not-a-tool".into(),
                arguments: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert!(r.is_error);
        match &r.content[0] {
            McpContent::Text { text } => assert!(text.contains("unknown tool")),
            _ => panic!("expected text"),
        }
    }
}
