//! Runtime-tool allow/deny derivation for materialized teammates — the
//! `[tools]` section the capability fence enforces.
//!
//! ## The bug this closes
//!
//! `[tools].allowed` is the RUNTIME-TOOL allow-list
//! (`argo_capability::WorkerCapabilityProfile` dispatch fence) — real
//! in-core tool names like `file_read` / `web_search`, NOT the teammate's
//! business-action vocabulary. The previous `build_tools` copied the
//! `[actions]` keys (`scan_invoice`, `draft_reminder`, …) into `allowed`,
//! granting dispatch on non-existent "tools" while withholding every real
//! tool the agent calls — so a Genesis-minted teammate held only the 4
//! `MINIMAL_TEAMMATE_TOOLS` and was structurally unable to read a file or
//! search the web. The hand-authored verticals
//! (`verticals/bookkeeping-assistant/vertical.toml`) already use real tool
//! names; this module brings the Genesis path in line.
//!
//! ## Derivation rules (bias: under-grant, deny wins)
//!
//! 1. **Safe baseline** ([`SAFE_BASELINE_EXTRA_TOOLS`] +
//!    [`argo_capability::MINIMAL_TEAMMATE_TOOLS`]): read-only /
//!    introspective tools every teammate needs to function at all.
//! 2. **Role-conditional** additions from small, auditable keyword maps over
//!    the spec's OWN deterministic text (charter + responsibilities +
//!    capability intents + action names — no LLM call): research/lookup
//!    signals grant the read-only web pair; explicit write/save signals
//!    grant `file_write`. When unsure, NOT granted.
//! 3. **Always-denied** ([`ALWAYS_DENIED_RUNTIME_TOOLS`]): shell / exec /
//!    spawn / patch / transmit tools go into `[tools].denied`
//!    unconditionally and are stripped from `allowed` no matter what any
//!    signal suggested. Deny-wins holds end-to-end:
//!    `WorkerCapabilityProfile::permits_tool` gives `denied_tools`
//!    precedence over the allow-set, so even an `[actions]` key that
//!    collides with a denied tool name cannot resurrect it.
//! 4. The structural sensitive-verb deny-list
//!    ([`crate::safety_judge::SENSITIVE_VERBS`]) is preserved on top, so a
//!    hallucinated tool name carrying `send`/`pay`/`delete`/… is still
//!    refused (existing behaviour, unregressed).
//!
//! Runtime tool names are pinned to the REAL in-core registry — see
//! `builtin_tools_returns_unique_names` in `crates/argo-tool/src/tools/mod.rs`
//! (argo-genesis deliberately does not link argo-tool; the sibling test file
//! pins the exact names so a registry rename breaks the build here, not
//! silently at dispatch).

use crate::spec::CapabilitySpec;
use crate::{safety_judge, spec::humanize_action_name};
use argo_vertical::{ActionsSection, ToolsSection};
use std::collections::BTreeSet;

/// Read-only in-core tools granted to EVERY materialized teammate on top of
/// [`argo_capability::MINIMAL_TEAMMATE_TOOLS`] (clarify / memory /
/// skill_view / skills_list). `file_read` (read a known path) + `file_list`
/// (enumerate a directory): reading AND discovering files inside the worker's
/// sandbox is the baseline every file-working role needs — without `file_list`
/// an "ingest this folder / inbox" teammate has to guess filenames and stalls
/// (the 2026-06-20 stress-test P1). Both stay doubly fenced — they emit only an
/// `FsRead` capability, which is inert unless the worker also has a scoped
/// `fs_root` (`WorkerCapabilityProfile::with_fs_root`).
pub(crate) const SAFE_BASELINE_EXTRA_TOOLS: &[&str] = &["file_read", "file_list"];

/// The read-only web research pair, granted ONLY when the role text carries
/// a research/lookup signal ([`RESEARCH_SIGNAL_TOKENS`] /
/// [`RESEARCH_SIGNAL_PHRASES`]).
pub(crate) const WEB_RESEARCH_TOOLS: &[&str] = &["web_search", "web_extract"];

/// The single write-side file tool, granted ONLY on an explicit write/save
/// signal ([`FILE_WRITE_SIGNAL_TOKENS`]). Like `file_read`, it is inert
/// without a scoped `fs_root` (no `FsWrite` capability is emitted).
pub(crate) const FILE_WRITE_TOOL: &str = "file_write";

/// Runtime tools a Genesis-minted teammate may NEVER hold, regardless of
/// any signal: the shell/exec/spawn family (`terminal`, `code_execute`,
/// `process`, `delegate_task`) plus arbitrary file mutation (`file_patch`)
/// and external transmit (`send_message` — DEC-025 #6: a day-one teammate
/// drafts/proposes but physically cannot transmit). Listed in
/// `[tools].denied` so the fence refuses them even if a colliding
/// `[actions]` key lands them in the profile's allow-set (deny wins in
/// `WorkerCapabilityProfile::permits_tool`).
///
/// Re-exported at the crate root (`argo_genesis::ALWAYS_DENIED_RUNTIME_TOOLS`)
/// so the durable per-worker capability-grant route (tg3) refuses to grant any
/// of these with an explicit error, reusing THIS canonical list rather than
/// forking a second always-denied taxonomy (parallel-surface rule).
pub const ALWAYS_DENIED_RUNTIME_TOOLS: &[&str] = &[
    "code_execute",
    "delegate_task",
    "file_patch",
    "process",
    "send_message",
    "terminal",
];

/// Whole-token research/lookup signals (matched against the lowercased,
/// non-alphanumeric-split role text). Small + auditable by design; extend
/// deliberately, never broadly.
const RESEARCH_SIGNAL_TOKENS: &[&str] = &[
    "research",
    "researches",
    "researching",
    "search",
    "searches",
    "searching",
    "browse",
    "browses",
    "browsing",
    "web",
    "online",
    "internet",
    "lookup",
    "lookups",
];

/// Contiguous-phrase research signals (substring match on the lowercased
/// role text, for multi-word idioms token-matching cannot see).
const RESEARCH_SIGNAL_PHRASES: &[&str] = &["look up", "looks up", "looking up"];

/// Whole-token write/save signals. Deliberately strict: "draft" is NOT a
/// write signal (drafting is an in-conversation proposal, not a disk
/// write), so a draft-only role never gets `file_write`.
const FILE_WRITE_SIGNAL_TOKENS: &[&str] = &[
    "save", "saves", "saved", "saving", "write", "writes", "writing", "written",
];

/// Build the least-privilege `[tools]` section: a SAFE runtime-tool
/// allow-list (baseline + role-conditional, per the module rules) and a
/// deny-list of the always-denied runtime tools + the structural sensitive
/// verbs. `allowed` and `denied` are guaranteed disjoint (the manifest
/// validator rejects overlap), and every [`ALWAYS_DENIED_RUNTIME_TOOLS`]
/// entry is guaranteed present in `denied` and absent from `allowed`.
pub(crate) fn build_tools(spec: &CapabilitySpec, actions: &ActionsSection) -> ToolsSection {
    let text = role_signal_text(spec);
    let mut allowed: BTreeSet<String> = argo_capability::MINIMAL_TEAMMATE_TOOLS
        .iter()
        .chain(SAFE_BASELINE_EXTRA_TOOLS)
        .map(|t| (*t).to_string())
        .collect();
    if has_token_signal(&text, RESEARCH_SIGNAL_TOKENS)
        || RESEARCH_SIGNAL_PHRASES.iter().any(|p| text.contains(p))
    {
        allowed.extend(WEB_RESEARCH_TOOLS.iter().map(|t| (*t).to_string()));
    }
    if has_token_signal(&text, FILE_WRITE_SIGNAL_TOKENS) {
        allowed.insert(FILE_WRITE_TOOL.to_string());
    }
    // Deny-wins at the source: strip the always-denied set from `allowed`
    // even if a future signal map (or constant edit) ever suggested one.
    for tool in ALWAYS_DENIED_RUNTIME_TOOLS {
        allowed.remove(*tool);
    }

    let mut denied: BTreeSet<String> = ALWAYS_DENIED_RUNTIME_TOOLS
        .iter()
        .map(|t| (*t).to_string())
        .collect();
    // Existing structural fence, unregressed: every sensitive verb that is
    // neither an allowed runtime tool nor a declared action stays denied,
    // so a hallucinated `pay`/`delete`/… tool name is refused. (Declared
    // actions are excluded exactly as before — they are approval-gated by
    // the safety judge, not name-banned.)
    denied.extend(
        safety_judge::SENSITIVE_VERBS
            .iter()
            .map(|verb| (*verb).to_string())
            .filter(|verb| !allowed.contains(verb) && !actions.actions.contains_key(verb)),
    );

    ToolsSection {
        allowed: allowed.into_iter().collect(),
        denied: denied.into_iter().collect(),
    }
}

/// Concatenate the spec's deterministic role text — charter,
/// responsibilities, every capability's intent + humanized action name —
/// lowercased, for signal matching. No LLM output beyond what the spec
/// already carries is consulted.
fn role_signal_text(spec: &CapabilitySpec) -> String {
    let mut text = String::with_capacity(256);
    text.push_str(&spec.charter);
    for r in &spec.responsibilities {
        text.push(' ');
        text.push_str(r);
    }
    for cap in &spec.capabilities {
        text.push(' ');
        text.push_str(&cap.intent);
        text.push(' ');
        text.push_str(&humanize_action_name(&cap.action_name));
    }
    text.to_lowercase()
}

/// `true` when any whole token of `text` (split on non-alphanumerics)
/// equals one of `signals`. Whole-token matching keeps short signals from
/// false-positive substring hits (`web` ≠ `webinar`, `write` ≠ `rewrite`).
fn has_token_signal(text: &str, signals: &[&str]) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .any(|token| signals.contains(&token))
}

#[cfg(test)]
#[path = "materialize_tools_tests.rs"]
mod tests;
