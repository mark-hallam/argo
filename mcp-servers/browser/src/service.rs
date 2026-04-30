//! Browser MCP service — 12 tools per §28.AA Hermes-parity:
//! navigate, click, fill, submit, screenshot, read_page,
//! extract_links, wait_for, scroll, eval, cookies, pdf.
//!
//! The schemas + dispatcher land today; chromiumoxide / local-
//! Playwright wiring + Browserbase / BrowserUse cloud backends
//! land per §12.2.1 in a follow-up.
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
        ]
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let known = TOOL_NAMES.contains(&params.name.as_str());
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

fn navigate_schema() -> McpToolSchema {
    McpToolSchema {
        name: "navigate".into(),
        description: "Open a URL in the browser context.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "url": { "type": "string" } },
            "required": ["url"]
        }),
    }
}

fn click_schema() -> McpToolSchema {
    McpToolSchema {
        name: "click".into(),
        description: "Click a CSS selector.".into(),
        input_schema: selector_schema(),
    }
}

fn fill_schema() -> McpToolSchema {
    McpToolSchema {
        name: "fill".into(),
        description: "Fill a form field.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string" },
                "value": { "type": "string" }
            },
            "required": ["selector", "value"]
        }),
    }
}

fn submit_schema() -> McpToolSchema {
    McpToolSchema {
        name: "submit".into(),
        description: "Submit a form (clicks the submit button or presses Enter).".into(),
        input_schema: selector_schema(),
    }
}

fn screenshot_schema() -> McpToolSchema {
    McpToolSchema {
        name: "screenshot".into(),
        description: "Capture a viewport screenshot. Returns base64 PNG.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "full_page": { "type": "boolean", "default": false }
            }
        }),
    }
}

fn read_page_schema() -> McpToolSchema {
    McpToolSchema {
        name: "read_page".into(),
        description: "Extract the page's visible text content (Hermes parity).".into(),
        input_schema: serde_json::json!({"type": "object"}),
    }
}

fn extract_links_schema() -> McpToolSchema {
    McpToolSchema {
        name: "extract_links".into(),
        description: "List all hyperlinks (href + visible text) on the page.".into(),
        input_schema: serde_json::json!({"type": "object"}),
    }
}

fn wait_for_schema() -> McpToolSchema {
    McpToolSchema {
        name: "wait_for".into(),
        description: "Wait for a selector to appear or for network-idle.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "selector":   { "type": "string" },
                "network_idle": { "type": "boolean", "default": false },
                "timeout_ms": { "type": "integer", "default": 30000 }
            }
        }),
    }
}

fn scroll_schema() -> McpToolSchema {
    McpToolSchema {
        name: "scroll".into(),
        description: "Scroll the viewport.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "direction": { "type": "string", "enum": ["up", "down", "to-top", "to-bottom"] },
                "pixels":    { "type": "integer", "default": 500 }
            },
            "required": ["direction"]
        }),
    }
}

fn eval_schema() -> McpToolSchema {
    McpToolSchema {
        name: "eval".into(),
        description: "Run JavaScript in the page context. Caller is responsible for escaping."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "script": { "type": "string" } },
            "required": ["script"]
        }),
    }
}

fn cookies_schema() -> McpToolSchema {
    McpToolSchema {
        name: "cookies".into(),
        description: "Get or set cookies for the current origin.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "op":    { "type": "string", "enum": ["get", "set", "clear"] },
                "name":  { "type": "string" },
                "value": { "type": "string" }
            },
            "required": ["op"]
        }),
    }
}

fn pdf_schema() -> McpToolSchema {
    McpToolSchema {
        name: "pdf".into(),
        description: "Render the page to a PDF. Returns base64.".into(),
        input_schema: serde_json::json!({"type": "object"}),
    }
}

fn selector_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": { "selector": { "type": "string" } },
        "required": ["selector"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_advertises_twelve_tools() {
        let s = BrowserService;
        let tools = s.list_tools();
        assert_eq!(tools.len(), 12);
    }

    #[tokio::test]
    async fn tool_names_match_spec() {
        let s = BrowserService;
        let tools = s.list_tools();
        for (i, expected) in TOOL_NAMES.iter().enumerate() {
            assert_eq!(tools[i].name, *expected, "position {i}");
        }
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
    async fn known_tool_returns_not_wired_error() {
        let s = BrowserService;
        let r = s
            .call_tool(CallToolParams {
                name: "navigate".into(),
                arguments: serde_json::json!({"url": "https://example.com"}),
            })
            .await
            .unwrap();
        assert!(r.is_error);
        match &r.content[0] {
            McpContent::Text { text } => assert!(text.contains("not yet wired")),
            _ => panic!("expected text"),
        }
    }
}
