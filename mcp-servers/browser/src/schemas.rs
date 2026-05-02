//! Schema constructors for browser MCP tools. Pure data
//! builders extracted from `service.rs` to keep that file under
//! the 500-LOC budget.
use argo_mcp::McpToolSchema;

pub(crate) fn list_tabs_schema() -> McpToolSchema {
    McpToolSchema {
        name: "list_tabs".into(),
        description: "Probe Chrome's `/json` endpoint and return the open tabs. \
            Requires Chrome to be running with `--remote-debugging-port=<port>` \
            (default 9222). Pure HTTP — does not need the chromiumoxide driver."
            .into(),
        input_schema: chrome_port_schema(),
    }
}

pub(crate) fn chrome_version_schema() -> McpToolSchema {
    McpToolSchema {
        name: "chrome_version".into(),
        description: "Probe Chrome's `/json/version` endpoint and return the \
            User-Agent + protocol version + browser-level WebSocket URL. \
            Pure HTTP."
            .into(),
        input_schema: chrome_port_schema(),
    }
}

pub(crate) fn chrome_port_schema() -> serde_json::Value {
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

pub(crate) fn navigate_schema() -> McpToolSchema {
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

pub(crate) fn click_schema() -> McpToolSchema {
    McpToolSchema {
        name: "click".into(),
        description: "Click an element matched by CSS selector via CDP \
            Runtime.evaluate(el.click())."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string" },
                "port":     { "type": "integer", "default": 9222 }
            },
            "required": ["selector"]
        }),
    }
}

pub(crate) fn fill_schema() -> McpToolSchema {
    McpToolSchema {
        name: "fill".into(),
        description: "Set `value` on a form field (input/textarea/select) \
            via CDP Runtime.evaluate, dispatching `input` + `change` events \
            so client-side JS sees the change."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string" },
                "value":    { "type": "string" },
                "port":     { "type": "integer", "default": 9222 }
            },
            "required": ["selector", "value"]
        }),
    }
}

pub(crate) fn submit_schema() -> McpToolSchema {
    McpToolSchema {
        name: "submit".into(),
        description: "Submit a form via CDP Runtime.evaluate. The selector \
            can match the `<form>` directly or any enclosed input — we walk \
            up to the form via `el.form`."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string" },
                "port":     { "type": "integer", "default": 9222 }
            },
            "required": ["selector"]
        }),
    }
}

pub(crate) fn screenshot_schema() -> McpToolSchema {
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

pub(crate) fn read_page_schema() -> McpToolSchema {
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

pub(crate) fn extract_links_schema() -> McpToolSchema {
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

pub(crate) fn wait_for_schema() -> McpToolSchema {
    McpToolSchema {
        name: "wait_for".into(),
        description: "Poll for a CSS selector to appear via CDP \
            Runtime.evaluate (200ms cadence) until truthy or timeout. \
            Network-idle waiting is not yet wired."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "selector":   { "type": "string" },
                "timeout_ms": { "type": "integer", "default": 30000 },
                "port":       { "type": "integer", "default": 9222 }
            },
            "required": ["selector"]
        }),
    }
}

pub(crate) fn scroll_schema() -> McpToolSchema {
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

pub(crate) fn eval_schema() -> McpToolSchema {
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

pub(crate) fn cookies_schema() -> McpToolSchema {
    McpToolSchema {
        name: "cookies".into(),
        description: "Get / set / clear cookies via CDP Network domain. \
            `op=get` returns the full cookie array; `op=set` requires \
            `name` (and accepts `value`, `domain`, `path`, `url`); \
            `op=clear` calls Network.clearBrowserCookies."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "op":     { "type": "string", "enum": ["get", "set", "clear"] },
                "name":   { "type": "string" },
                "value":  { "type": "string" },
                "domain": { "type": "string" },
                "path":   { "type": "string" },
                "url":    { "type": "string" },
                "port":   { "type": "integer", "default": 9222 }
            },
            "required": ["op"]
        }),
    }
}

pub(crate) fn pdf_schema() -> McpToolSchema {
    McpToolSchema {
        name: "pdf".into(),
        description: "Render the first open page to PDF via CDP \
            Page.printToPDF. Returns the base64 payload wrapped in a \
            `data:application/pdf;base64,...` URL."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "port": { "type": "integer", "default": 9222 }
            }
        }),
    }
}

pub(crate) fn session_open_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_open".into(),
        description: "Open a persistent CDP WebSocket pinned to one \
            tab. Returns `{session_id}` for use with subsequent \
            `session_eval` / `session_close` calls — no /json round-\
            trip per call."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "port": { "type": "integer", "default": 9222 } }
        }),
    }
}

pub(crate) fn session_close_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_close".into(),
        description: "Close a session opened with `session_open` and \
            drop its WebSocket."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id"],
            "properties": { "session_id": { "type": "string" } }
        }),
    }
}

pub(crate) fn session_eval_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_eval".into(),
        description: "Like `eval`, but runs against an existing \
            persistent session — saves the /json + WS-handshake \
            round-trip on every call."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id", "expression"],
            "properties": {
                "session_id": { "type": "string" },
                "expression": { "type": "string" }
            }
        }),
    }
}

pub(crate) fn session_navigate_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_navigate".into(),
        description: "Navigate the tab pinned by `session_open` to \
            a new URL via the persistent WS. Same shape as the \
            single-shot `navigate` tool but reuses the session's \
            connection — no /json round-trip."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id", "url"],
            "properties": {
                "session_id": { "type": "string" },
                "url": { "type": "string" }
            }
        }),
    }
}

pub(crate) fn session_list_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_list".into(),
        description: "List every active browser session id \
            (sessions opened with `session_open` and not yet \
            closed). Returns `{session_ids: [...], count: N}` \
            so a host can audit / reap stale sessions."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

