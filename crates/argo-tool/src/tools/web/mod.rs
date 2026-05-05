//! `web_search` and `web_extract` — real in-core implementations
//! (#723). Worker v2's pre-flight gate (#722) was correctly
//! refusing to run autonomous research jobs because both tools
//! were schema-only stubs; this PR replaces the stubs with
//! actual HTTP-backed implementations so the worker can plan
//! → research → verify → draft inside one queued job.
//!
//! ## Backend choice
//!
//! `web_search` defaults to scraping DuckDuckGo's HTML endpoint
//! (`https://html.duckduckgo.com/html/`) — no API key required,
//! works out of the box. When `TAVILY_API_KEY` is set we prefer
//! Tavily (purpose-built for LLMs, JSON results); when
//! `BRAVE_API_KEY` is set we prefer Brave Search. Selection
//! happens per call so operators can rotate keys without a
//! restart.
//!
//! `web_extract` fetches the URL with reqwest then runs a small
//! readability-style extraction over the response HTML — strips
//! `<script>`/`<style>`/nav chrome, prefers `<article>` /
//! `<main>` / `[role=main]` content, falls back to all `<p>`
//! tags inside `<body>`. No JS rendering — that's the browser
//! MCP server's job (#642).
//!
//! Both tools serialize their output as JSON inside a single
//! text block so the agent loop's existing tool_result plumbing
//! handles them without changes.
//!
//! Sub-modules:
//!   * `search` — backend selection + per-backend fetch + DDG HTML parser
//!   * `extract` — fetch + readability-style text extraction

mod extract;
mod search;

use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_types::{ArgoResult, Capability, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::{Duration, Instant};

/// Default ceiling on results returned to the model — keeps the
/// tool_result block compact and stops a runaway query from
/// burning context.
const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 25;

/// User-Agent header. DuckDuckGo's HTML endpoint rejects empty
/// or obviously-bot UA strings, so we identify as Argo with a
/// pointer to the project URL — honest, scrape-friendly.
const UA: &str = concat!(
    "argo-tool/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/mark-hallam/Argo-Agent)"
);

const REQ_TIMEOUT: Duration = Duration::from_secs(20);

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(REQ_TIMEOUT)
        .timeout(REQ_TIMEOUT)
        .build()
        .map_err(|e| format!("reqwest builder: {e}"))
}

/// Web search tool. Backend selection happens per call (see
/// module-level docs); the schema the model sees is constant.
#[derive(Debug, Default)]
pub struct WebSearch;

#[derive(Debug, Deserialize)]
struct SearchArgs {
    query: String,
    #[serde(default)]
    limit: Option<u32>,
}

#[async_trait]
impl Tool for WebSearch {
    fn name(&self) -> &str {
        "web_search"
    }
    fn description(&self) -> &str {
        "Search the web for the given query. Returns up to `limit` \
         results (default 10) with title, snippet, and URL. Cite \
         the URLs in any factual claim built on these results."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 25 },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        Concurrency::Parallel
    }

    fn required_capabilities(&self, _args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        Ok(CapabilitySet(vec![Capability::NetEgress {
            host_glob: "**".into(),
            port: None,
        }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: SearchArgs =
            serde_json::from_value(args).map_err(|e| argo_types::ArgoError::Tool {
                tool: "web_search".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        let limit = parsed.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

        let backend = search::SearchBackend::detect();
        match backend.run(&parsed.query, limit).await {
            Ok(results) => {
                let payload = serde_json::json!({
                    "backend": backend.name(),
                    "query": parsed.query,
                    "result_count": results.len(),
                    "results": results,
                });
                Ok(success_text(
                    ctx.call_id,
                    serde_json::to_string(&payload).unwrap_or_default(),
                    started.elapsed().as_millis() as u64,
                ))
            }
            Err(msg) => Ok(error_text(
                ctx.call_id,
                format!("web_search ({}) failed: {msg}", backend.name()),
                started.elapsed().as_millis() as u64,
            )),
        }
    }
}

/// Web extract tool — fetch a URL and return its readable body.
#[derive(Debug, Default)]
pub struct WebExtract;

#[derive(Debug, Deserialize)]
struct ExtractArgs {
    url: String,
}

#[async_trait]
impl Tool for WebExtract {
    fn name(&self) -> &str {
        "web_extract"
    }
    fn description(&self) -> &str {
        "Fetch a URL and return its readable text content (article \
         body, no markup). Use after web_search to verify a claim \
         against the actual page."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "url": { "type": "string", "format": "uri" },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        Concurrency::Parallel
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let parsed: ExtractArgs =
            serde_json::from_value(args.clone()).map_err(|e| argo_types::ArgoError::Tool {
                tool: "web_extract".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        let host = parsed
            .url
            .split("://")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .unwrap_or("**")
            .to_string();
        Ok(CapabilitySet(vec![Capability::NetEgress {
            host_glob: host,
            port: None,
        }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: ExtractArgs =
            serde_json::from_value(args).map_err(|e| argo_types::ArgoError::Tool {
                tool: "web_extract".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        match extract::fetch_and_extract(&parsed.url).await {
            Ok((title, text)) => {
                let payload = serde_json::json!({
                    "url": parsed.url,
                    "title": title,
                    "text": text,
                    "char_count": text.chars().count(),
                    "truncated": text.chars().count() >= extract::EXTRACT_MAX_CHARS,
                });
                Ok(success_text(
                    ctx.call_id,
                    serde_json::to_string(&payload).unwrap_or_default(),
                    started.elapsed().as_millis() as u64,
                ))
            }
            Err(msg) => Ok(error_text(
                ctx.call_id,
                format!("web_extract failed: {msg}"),
                started.elapsed().as_millis() as u64,
            )),
        }
    }
}

/// Falls back to a selector that matches no element. The CSS
/// tag `argo-impossible-tag-name` is a valid CSS identifier so
/// `Selector::parse` always succeeds here in practice; we use
/// `unwrap_or_else` chains so a hypothetical scraper-version
/// change that broke this never panics in the request path —
/// it just degrades to "no match", which is what we want.
fn any_selector() -> scraper::Selector {
    scraper::Selector::parse("argo-impossible-tag-name").unwrap_or_else(|_| {
        scraper::Selector::parse("a").unwrap_or_else(|_| {
            scraper::Selector::parse("*").unwrap_or_else(|_| unreachable_selector())
        })
    })
}

#[cold]
fn unreachable_selector() -> scraper::Selector {
    unreachable!("scraper cannot parse the universal selector `*` — fundamentally broken")
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::Principal;

    fn ctx() -> ToolContext {
        ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            std::path::PathBuf::from("/tmp"),
        )
    }

    #[tokio::test]
    async fn web_search_rejects_invalid_args() {
        // missing required `query` field
        let r = WebSearch.dispatch(serde_json::json!({}), &ctx()).await;
        assert!(r.is_err(), "missing query must produce a structured error");
    }

    #[test]
    fn web_extract_required_caps_narrow_to_host() {
        let caps = WebExtract
            .required_capabilities(&serde_json::json!({"url": "https://example.com/path"}))
            .unwrap();
        assert!(caps.covers(&Capability::NetEgress {
            host_glob: "example.com".into(),
            port: None,
        }));
    }

    /// Live opt-in test for web_search. Hits real DuckDuckGo HTML
    /// and asserts a known query produces at least one result.
    /// Gated behind `ARGO_LIVE_WEB=1` so CI never depends on the
    /// live SERP.
    #[tokio::test]
    #[cfg(not(target_os = "windows"))]
    async fn web_search_live_against_duckduckgo_when_opted_in() {
        if !std::env::var("ARGO_LIVE_WEB")
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            eprintln!("SKIP: live web search requires ARGO_LIVE_WEB=1");
            return;
        }
        // Force DDG path even if a key happens to be in env.
        std::env::remove_var("TAVILY_API_KEY");
        std::env::remove_var("BRAVE_API_KEY");
        std::env::remove_var("BRAVE_SEARCH_API_KEY");
        let r = WebSearch
            .dispatch(
                serde_json::json!({"query": "rust programming language", "limit": 3}),
                &ctx(),
            )
            .await
            .expect("dispatch");
        assert!(
            !r.is_error,
            "live DDG search must succeed; got error: {r:?}"
        );
        let text = match &r.content[0] {
            argo_types::ContentBlock::Text { text } => text.clone(),
            _ => panic!("text"),
        };
        let payload: serde_json::Value = serde_json::from_str(&text).expect("json");
        let n = payload["result_count"].as_u64().unwrap_or(0);
        assert!(
            n > 0,
            "expected at least one result; got payload: {payload}"
        );
    }

    /// Live opt-in test for web_extract.
    #[tokio::test]
    #[cfg(not(target_os = "windows"))]
    async fn web_extract_live_pulls_readable_text_when_opted_in() {
        if !std::env::var("ARGO_LIVE_WEB")
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            eprintln!("SKIP: live web extract requires ARGO_LIVE_WEB=1");
            return;
        }
        let r = WebExtract
            .dispatch(serde_json::json!({"url": "https://example.com/"}), &ctx())
            .await
            .expect("dispatch");
        assert!(
            !r.is_error,
            "extract must succeed against example.com; got: {r:?}"
        );
        let text = match &r.content[0] {
            argo_types::ContentBlock::Text { text } => text.clone(),
            _ => panic!("text"),
        };
        let payload: serde_json::Value = serde_json::from_str(&text).expect("json");
        let body = payload["text"].as_str().unwrap_or("");
        assert!(
            body.contains("Example") || body.contains("example"),
            "extract should yield example.com body text; got {} chars",
            body.len()
        );
    }
}
