//! Schema constructors for the session-* tool family.
//! Lifted out of `schemas.rs` so that file stays under the
//! 500-LOC budget once the stateful-session surface grew
//! to 12 tools.
use argo_mcp::McpToolSchema;

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

pub(crate) fn session_read_page_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_read_page".into(),
        description: "Like `read_page`, but runs against the \
            session's pinned tab via the persistent WS — no \
            /json + WS-handshake round-trip per call."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id"],
            "properties": { "session_id": { "type": "string" } }
        }),
    }
}

pub(crate) fn session_click_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_click".into(),
        description: "Like `click`, but runs against the \
            session's pinned tab via the persistent WS. Same \
            CSS-selector contract."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id", "selector"],
            "properties": {
                "session_id": { "type": "string" },
                "selector": { "type": "string" }
            }
        }),
    }
}

pub(crate) fn session_fill_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_fill".into(),
        description: "Like `fill`, but runs against the \
            session's pinned tab via the persistent WS. Sets \
            an input/textarea/select value and dispatches \
            input + change events."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id", "selector", "value"],
            "properties": {
                "session_id": { "type": "string" },
                "selector": { "type": "string" },
                "value": { "type": "string" }
            }
        }),
    }
}

pub(crate) fn session_submit_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_submit".into(),
        description: "Like `submit`, but runs against the \
            session's pinned tab via the persistent WS. \
            Submits the form nearest to the matched element."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id", "selector"],
            "properties": {
                "session_id": { "type": "string" },
                "selector": { "type": "string" }
            }
        }),
    }
}

pub(crate) fn session_extract_links_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_extract_links".into(),
        description: "Like `extract_links`, but runs against \
            the session's pinned tab via the persistent WS. \
            Returns a pretty-printed JSON array of \
            `{href, text}` objects for every `<a href>` on \
            the page."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id"],
            "properties": { "session_id": { "type": "string" } }
        }),
    }
}

pub(crate) fn session_screenshot_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_screenshot".into(),
        description: "Like `screenshot`, but runs against the \
            session's pinned tab via the persistent WS. \
            Returns an MCP Image content block with a base64 \
            PNG data URI. `full_page=true` captures beyond \
            the viewport."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id"],
            "properties": {
                "session_id": { "type": "string" },
                "full_page": { "type": "boolean", "default": false }
            }
        }),
    }
}

pub(crate) fn session_scroll_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_scroll".into(),
        description: "Like `scroll`, but runs against the \
            session's pinned tab. direction = up | down | \
            to-top | to-bottom; pixels (default 500) used by \
            up/down."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id"],
            "properties": {
                "session_id": { "type": "string" },
                "direction": {
                    "type": "string",
                    "enum": ["up", "down", "to-top", "to-bottom"],
                    "default": "down"
                },
                "pixels": { "type": "integer", "default": 500 }
            }
        }),
    }
}

pub(crate) fn session_wait_for_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_wait_for".into(),
        description: "Like `wait_for`, but polls against the \
            session's pinned tab. Polls every 200ms until the \
            CSS selector matches or timeout elapses."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id", "selector"],
            "properties": {
                "session_id": { "type": "string" },
                "selector": { "type": "string" },
                "timeout_ms": { "type": "integer", "default": 30000 }
            }
        }),
    }
}

pub(crate) fn session_pdf_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_pdf".into(),
        description: "Like `pdf`, but runs against the \
            session's pinned tab via the persistent WS. \
            Returns the PDF as an MCP Image content block \
            (mime application/pdf)."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id"],
            "properties": { "session_id": { "type": "string" } }
        }),
    }
}

pub(crate) fn session_cookies_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_cookies".into(),
        description: "Like `cookies` (op=get), but runs against \
            the session's pinned tab. Returns a pretty-printed \
            JSON array of cookies via Network.getCookies."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id"],
            "properties": { "session_id": { "type": "string" } }
        }),
    }
}

pub(crate) fn session_get_url_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_get_url".into(),
        description: "Return `window.location.href` for the \
            session's pinned tab. Useful after `session_navigate` \
            (or any in-page nav) to confirm where the tab \
            actually ended up — Page.navigate's response only \
            tells you what was *requested*, not what was \
            committed after redirects."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id"],
            "properties": { "session_id": { "type": "string" } }
        }),
    }
}

pub(crate) fn session_reap_schema() -> McpToolSchema {
    McpToolSchema {
        name: "session_reap".into(),
        description: "Drop every browser session whose \
            `last_used` timestamp is older than \
            `max_idle_secs` (default 600 = 10 minutes). \
            Pass `max_idle_secs=0` to reap all sessions \
            (useful for shutdown). Returns \
            `{reaped_ids, remaining, max_idle_secs}`."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "max_idle_secs": { "type": "integer", "default": 600 }
            }
        }),
    }
}


/// `cloud_browserbase_open_session` schema (#642). Spawns a hosted
/// Chromium session via Browserbase's REST API and returns the
/// CDP WebSocket connect URL plus the Browserbase session id.
/// Requires the `browserbase` feature at build time + the
/// `BROWSERBASE_API_KEY` and `BROWSERBASE_PROJECT_ID` env vars at
/// runtime.
pub(crate) fn cloud_browserbase_open_schema() -> McpToolSchema {
    McpToolSchema {
        name: "cloud_browserbase_open_session".into(),
        description: "Spawn a hosted Chromium session on Browserbase and \
            return its CDP connect_url + session_id. Requires the \
            `browserbase` feature at build time and BROWSERBASE_API_KEY \
            + BROWSERBASE_PROJECT_ID env vars at runtime. Free tier \
            covers ~60 min/month — sign up at https://browserbase.com."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

/// `cloud_browserbase_close_session` schema (#642). Tears down a
/// hosted session by id. Idempotent at the Browserbase end.
pub(crate) fn cloud_browserbase_close_schema() -> McpToolSchema {
    McpToolSchema {
        name: "cloud_browserbase_close_session".into(),
        description: "Tear down a Browserbase hosted session by id. \
            Idempotent — safe to call multiple times. Requires the \
            `browserbase` feature."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["session_id"],
            "properties": { "session_id": { "type": "string" } }
        }),
    }
}
