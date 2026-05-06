//! MCP server declarations for vertical manifests.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `[mcp_servers]` — MCP servers available to a vertical.
///
/// Each table entry names a server and declares how to start it.
/// For example:
///
/// ```toml
/// [mcp_servers]
/// gmail = { transport = "stdio", command = "npx", args = ["@modelcontextprotocol/server-gmail"] }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct McpServersSection {
    /// Server declarations keyed by the manifest table entry name.
    #[serde(flatten)]
    pub servers: BTreeMap<String, McpServerDeclaration>,
}

/// One MCP server transport declaration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct McpServerDeclaration {
    /// Transport kind. Phase 1 supports `stdio`.
    pub transport: String,
    /// Program to spawn for stdio transport.
    pub command: String,
    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,
}

#[cfg(test)]
mod tests {
    use crate::manifest::parse;

    #[test]
    fn parses_mcp_servers_section() {
        let raw = r#"
schema_version = "1"
[vertical]
id = "demo"
name = "Demo"
version = "0.1.0"

[mcp_servers]
gmail = { transport = "stdio", command = "npx", args = ["@modelcontextprotocol/server-gmail"] }
"#;
        let manifest = parse(raw).expect("manifest parses");
        let servers = manifest.mcp_servers.expect("mcp servers");
        let gmail = servers.servers.get("gmail").expect("gmail server");
        assert_eq!(gmail.transport, "stdio");
        assert_eq!(gmail.command, "npx");
        assert_eq!(gmail.args, vec!["@modelcontextprotocol/server-gmail"]);
    }
}
