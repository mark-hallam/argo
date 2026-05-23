//! Coverage pin: every in-core tool declares a per-call capability
//! (or is the documented session_search exemption) and has at least
//! one test file exercising it.
//!
//! Companion to the name/count pin at
//! `crates/argo-tool/src/tools/mod.rs::tests::builtin_tools_returns_unique_names`.
//! See the full audit at
//! `docs/skycot/evals/scheduled/2026-05-23/eval-in-core-tool-inventory-vs-mission.md`.
//!
//! When a new in-core tool is added: extend both `EXAMPLE_ARGS` and
//! `TOOL_TEST_FILES` below. When a tool migrates to MCP and disappears
//! from `builtin_tools(...)`: drop the matching entries.

use std::path::{Path, PathBuf};

use argo_tool::builtin_tools;

/// Tools that intentionally return `CapabilitySet::empty()` from
/// `required_capabilities` and rely on the registry's
/// `Capability::ToolDispatch { tool_name_glob }` as the sole gate.
/// Documented in the tool's module header — keep this list in sync
/// with those docstrings.
const DOCUMENTED_EMPTY: &[&str] = &["session_search"];

/// Minimal-valid args per tool. Picked so `required_capabilities`
/// returns a non-empty `CapabilitySet` for every tool except the
/// `DOCUMENTED_EMPTY` entries above.
///
/// `memory` uses `action="write"` so the `MemoryWrite` capability
/// branch fires; the `read`/`list` actions intentionally produce an
/// empty set because no extra capability beyond the registry glob
/// is needed for a read.
fn example_args(name: &str) -> serde_json::Value {
    match name {
        "clarify" => serde_json::json!({"question": "x"}),
        "code_execute" => serde_json::json!({}),
        "delegate_task" => serde_json::json!({}),
        "file_patch" => {
            serde_json::json!({"path": "a.txt", "old_str": "b", "new_str": "c"})
        }
        "file_read" => serde_json::json!({"path": "a.txt"}),
        "file_write" => serde_json::json!({"path": "a.txt", "content": "x"}),
        "memory" => serde_json::json!({"action": "write", "entry": "MEMORY.md", "content": "x"}),
        "process" => serde_json::json!({}),
        "send_message" => {
            serde_json::json!({"channel": "slack", "recipient": "C1", "body": "x"})
        }
        "session_search" => serde_json::json!({"query": "x"}),
        "terminal" => serde_json::json!({"command": "x"}),
        "todo" => serde_json::json!({"action": "list"}),
        "vision_analyze" => {
            serde_json::json!({"image": "https://example.com/x.png", "question": "x"})
        }
        "web_extract" => serde_json::json!({"url": "https://example.com"}),
        "web_search" => serde_json::json!({"query": "x"}),
        other => panic!(
            "in_core_tool_inventory_pin_test: missing example args for tool '{other}'. \
             A new tool was added to builtin_tools — extend EXAMPLE_ARGS in this file."
        ),
    }
}

/// Per-tool list of files known to contain `#[test]` /
/// `#[tokio::test]` blocks exercising the tool's surface. The pin
/// fails if any listed file is missing or contains no test
/// attribute — that's the signal the tool's test coverage has
/// regressed.
const TOOL_TEST_FILES: &[(&str, &[&str])] = &[
    (
        "clarify",
        &[
            "src/tools/clarify_tests.rs",
            "tests/todo_clarify_integration.rs",
        ],
    ),
    (
        "code_execute",
        &["src/tools/code_execute.rs", "tests/code_execute_integration.rs"],
    ),
    (
        "delegate_task",
        &[
            "src/tools/delegate_task_tests.rs",
            "tests/delegate_task_real_smoke.rs",
        ],
    ),
    ("file_patch", &["src/tools/file_patch.rs"]),
    ("file_read", &["src/tools/file_read.rs"]),
    ("file_write", &["src/tools/file_write.rs"]),
    (
        "memory",
        &[
            "src/tools/memory_legacy_tests.rs",
            "src/tools/memory_namespace_tests.rs",
            "tests/memory_namespace_smoke.rs",
            "tests/memory_query_scored_smoke.rs",
        ],
    ),
    (
        "process",
        &["src/tools/process.rs", "tests/process_registry_integration.rs"],
    ),
    (
        "send_message",
        &[
            "src/tools/send_message_tests.rs",
            "tests/send_message_integration.rs",
        ],
    ),
    (
        "session_search",
        &[
            "src/tools/session_search_tests.rs",
            "tests/session_search_integration.rs",
        ],
    ),
    (
        "terminal",
        &["src/tools/terminal_tests.rs", "tests/terminal_integration.rs"],
    ),
    (
        "todo",
        &["src/tools/todo_tests.rs", "tests/todo_clarify_integration.rs"],
    ),
    (
        "vision_analyze",
        &["src/tools/vision_tests.rs", "tests/vision_dispatch.rs"],
    ),
    ("web_extract", &["src/tools/web/mod.rs"]),
    ("web_search", &["src/tools/web/mod.rs"]),
];

#[test]
fn tool_declares_capability_or_documented_empty() {
    let tools = builtin_tools(PathBuf::from("/tmp"));
    assert_eq!(
        tools.len(),
        15,
        "in-core tool count drift; companion pin at \
         crates/argo-tool/src/tools/mod.rs::tests::builtin_tools_returns_unique_names",
    );

    for tool in &tools {
        let name = tool.name();
        let args = example_args(name);
        let caps = tool.required_capabilities(&args).unwrap_or_else(|e| {
            panic!(
                "required_capabilities({name}) errored on the pinned example args: {e:?}. \
                 Update example_args() in this test file."
            )
        });

        if DOCUMENTED_EMPTY.contains(&name) {
            assert!(
                caps.0.is_empty(),
                "{name} is listed in DOCUMENTED_EMPTY but returned a non-empty \
                 CapabilitySet — either drop it from DOCUMENTED_EMPTY or remove the \
                 capability declaration to match the module docstring.",
            );
        } else {
            assert!(
                !caps.0.is_empty(),
                "{name} returned an empty CapabilitySet on the pinned example args. \
                 Either declare a per-call capability in required_capabilities(), \
                 or add the tool to DOCUMENTED_EMPTY with a module-docstring note \
                 explaining why the registry's ToolDispatch glob is a sufficient gate.",
            );
        }
    }
}

#[test]
fn tool_has_at_least_one_test_file() {
    let tools = builtin_tools(PathBuf::from("/tmp"));
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    let covered: Vec<&str> = TOOL_TEST_FILES.iter().map(|(n, _)| *n).collect();
    for name in &names {
        assert!(
            covered.contains(name),
            "tool '{name}' is returned by builtin_tools() but absent from \
             TOOL_TEST_FILES — add coverage for it.",
        );
    }
    for (tool_name, _) in TOOL_TEST_FILES {
        assert!(
            names.contains(tool_name),
            "TOOL_TEST_FILES references '{tool_name}' but builtin_tools() doesn't \
             return it. Either the tool was removed (drop the entry) or its name \
             changed (update the entry).",
        );
    }

    for (tool_name, paths) in TOOL_TEST_FILES {
        let mut found = false;
        for rel in *paths {
            let full = crate_root.join(rel);
            let body = std::fs::read_to_string(&full).unwrap_or_else(|e| {
                panic!(
                    "expected test file '{rel}' to exist for tool '{tool_name}' but \
                     read failed: {e}. Update TOOL_TEST_FILES if the file moved."
                )
            });
            if body.contains("#[test]") || body.contains("#[tokio::test]") {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "tool '{tool_name}': none of the listed files contain a `#[test]` or \
             `#[tokio::test]` attribute. Either add a test or update TOOL_TEST_FILES \
             to point at a file that actually exercises the tool.",
        );
    }
}
