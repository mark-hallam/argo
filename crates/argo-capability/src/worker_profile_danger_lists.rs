//! Deterministic irreversible-action lists for the worker capability
//! fence (extracted from `worker_profile.rs` to keep both files under the
//! 500-LOC ceiling; DEC-050 G1 cu1d rebase pushed the parent over).
//!
//! These two `pub const` lists are the hard, fixed floor under the
//! semantic judge — the action families where an autonomous mistake
//! cannot be undone. They are re-exported from [`crate::worker_profile`]
//! so the public path (`argo_capability::IRREVERSIBLE_ACTION_VERBS` etc.)
//! is unchanged. Matching logic lives in `worker_profile.rs`'s
//! `is_irreversible_action`.

/// Irreversible / business-ruining action verbs that ALWAYS require a
/// human in the loop, no matter what a generated profile grants — matched
/// as a **whole `_`-delimited token** (case-insensitive), or as a
/// contiguous phrase for the multi-token entries (`wire_funds`).
///
/// Deterministic by construction: this is a fixed list, NOT an
/// LLM-judged classification. It is the hard floor under the semantic
/// judge — the actions where an autonomous mistake cannot be undone.
///
/// Token-matching (not substring) is used for the short / ambiguous verbs
/// here so an unrelated word that merely *contains* one of them does not
/// false-positive (e.g. `rewire_config` ≠ `wire`, `display_board` ≠ `pay`,
/// `attach_file` ≠ `ach`). The high-signal stems that ARE safe to match
/// anywhere in a compound name live in [`IRREVERSIBLE_DANGER_STEMS`].
///
/// ## Synonym coverage (closes the token-exact / synonym-blind hole)
///
/// The original list was token-exact + synonym-blind: `payment` ≠ `pay`,
/// and `transfer` / `empty` / `erase` / `export` / `authenticate` went
/// uncaught — so money/destroy/exfil/credential actions surfaced
/// autonomous + low-risk. The families below are now covered (money,
/// destructive, egress, credential). A false-positive approval gate is
/// SAFE; a false-negative is the hole, so coverage biases toward flagging.
pub const IRREVERSIBLE_ACTION_VERBS: &[&str] = &[
    // money movement (whole-token; short/ambiguous, so not substring-matched)
    "wire",
    "wire_funds",
    "pay",
    "remit",
    "ach",
    "trade",
    "withdraw",
    "deposit",
    // destructive (whole-token)
    "drop",
    "drop_table",
    "wipe",
    "purge",
    "destroy",
    "erase",
    "empty",
    "clear",
    "flush",
    "remove",
    "terminate",
    "deprovision",
    "revoke",
    // account / subscription lifecycle (whole-token + contiguous phrase)
    "cancel_subscription",
    "delete_account",
    "close_account",
    // egress / credential (whole-token; the stems below cover compounds)
    "export",
    "exfiltrate",
    "upload",
    "forward",
    "leak",
    "extract",
    "download",
    "authenticate",
    "credential",
    "password",
    "login",
    "signin",
];

/// High-signal dangerous **stems** matched anywhere inside a compound
/// action name (substring, case-insensitive) — so `process_payment`,
/// `TRANSFER_funds`, `EXTRACT_customer_records`, `authenticate_portal`,
/// and `empty_archived_storage` all promote to always-human even though
/// the dangerous family appears as part of a larger token. Every stem
/// here was chosen because a substring hit is almost always genuinely
/// dangerous (low benign-collision risk); short / ambiguous verbs that
/// would false-positive as substrings (e.g. `pay` → `display`, `ach` →
/// `attach`) stay token-only in [`IRREVERSIBLE_ACTION_VERBS`].
pub const IRREVERSIBLE_DANGER_STEMS: &[&str] = &[
    // money movement
    "payment",
    "transfer",
    "disburse",
    "payout",
    "remit",
    "chargeback",
    "refund",
    "withdraw",
    // destructive
    "truncate",
    "destroy",
    "deprovision",
    "purge",
    "exfiltrat",
    // egress
    "exfiltrate",
    "extract",
    "download",
    // credential
    "authenticate",
    "credential",
    "password",
];
