//! Notion MCP connector.
mod client;
mod service;

pub use client::{NotionClient, NotionConfig};
pub use service::NotionMcpService;
