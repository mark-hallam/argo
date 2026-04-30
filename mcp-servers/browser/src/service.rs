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

fn list_tabs_schema() -> McpToolSchema {
    McpToolSchema {
        name: "list_tabs".into(),
        description: "Probe Chrome's `/json` endpoint and return the open tabs. \
            Requires Chrome to be running with `--remote-debugging-port=<port>` \
            (default 9222). Pure HTTP — does not need the chromiumoxide driver."
            .into(),
        input_schema: chrome_port_schema(),
    }
}

fn chrome_version_schema() -> McpToolSchema {
    McpToolSchema {
        name: "chrome_version".into(),
        description: "Probe Chrome's `/json/version` endpoint and return the \
            User-Agent + protocol version + browser-level WebSocket URL. \
            Pure HTTP."
            .into(),
        input_schema: chrome_port_schema(),
    }
}

fn chrome_port_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "port": {
                "type": "integer",
                "description": "Chrome debug port. Defaults to 9222.",
                "default": 9222
            }
        }
    })
}

fn navigate_schema() -> McpToolSchema {
    McpToolSchema {
        name: "navigate".into(),
        description: "Open a URL in the first page-type tab via CDP \
            Page.navigate. Does not wait for the load to finish — the \
            caller is expected to follow up with `wait_for` (deferred) \
            or poll `read_page`."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string" },
                "port": { "type": "integer", "default": 9222 }
            },
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
        description: "Capture a screenshot of the first open page via CDP \
            Page.captureScreenshot. Returns an MCP Image content block \
            (base64 PNG, embedded as a `data:image/png;base64,...` URL). \
            Pass `full_page=true` for `captureBeyondViewport`."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "full_page": { "type": "boolean", "default": false },
                "port": {
                    "type": "integer",
                    "description": "Chrome debug port. Defaults to 9222.",
                    "default": 9222
                }
            }
        }),
    }
}

fn read_page_schema() -> McpToolSchema {
    McpToolSchema {
        name: "read_page".into(),
        description: "Return `document.body.innerText` of the first open page \
            via CDP Runtime.evaluate. Convenience wrapper around `eval`."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "port": {
                    "type": "integer",
                    "description": "Chrome debug port. Defaults to 9222.",
                    "default": 9222
                }
            }
        }),
    }
}

fn extract_links_schema() -> McpToolSchema {
    McpToolSchema {
        name: "extract_links".into(),
        description: "List every `<a href>` on the first open page as JSON \
            (`[{href, text}, …]`). Single-shot CDP Runtime.evaluate."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "port": {
                    "type": "integer",
                    "description": "Chrome debug port. Defaults to 9222.",
                    "default": 9222
                }
            }
        }),
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
        description: "Scroll the viewport via CDP Runtime.evaluate. Returns \
            the new {x, y} scroll position. `up` / `down` take `pixels` \
            (default 500); `to-top` / `to-bottom` ignore it."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "direction": { "type": "string", "enum": ["up", "down", "to-top", "to-bottom"] },
                "pixels":    { "type": "integer", "default": 500 },
                "port":      { "type": "integer", "default": 9222 }
            },
            "required": ["direction"]
        }),
    }
}

fn eval_schema() -> McpToolSchema {
    McpToolSchema {
        name: "eval".into(),
        description: "Run JavaScript in the first open page via CDP \
            Runtime.evaluate. Returns the value (or description for thrown \
            errors). Requires Chrome with `--remote-debugging-port=<port>` \
            (default 9222) and at least one `page`-type tab."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "JS source. Caller is responsible for escaping."
                },
                "port": {
                    "type": "integer",
                    "description": "Chrome debug port. Defaults to 9222.",
                    "default": 9222
                }
            },
            "required": ["expression"]
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
    async fn list_advertises_twelve_spec_tools_plus_http_probes() {
        let s = BrowserService;
        let tools = s.list_tools();
        assert_eq!(
            tools.len(),
            14,
            "12 spec tools + list_tabs + chrome_version"
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
    async fn unwired_spec_tool_returns_not_wired_error() {
        // `cookies` still needs a stateful CDP session — that's
        // the chromiumoxide rework. Pick it as the canary for the
        // not-wired stub path. (Was previously `navigate`, which
        // is now a real CDP tool.)
        let s = BrowserService;
        let r = s
            .call_tool(CallToolParams {
                name: "cookies".into(),
                arguments: serde_json::json!({"op": "get"}),
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
