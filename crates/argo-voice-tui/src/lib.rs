//! `argo-voice-tui` — local voice TUI for hands-free approval flow.
//!
//! Operator says `Argo approve maria` at the terminal; whisper.cpp
//! transcribes locally (privacy: voice never leaves the box), the
//! grammar layer maps the phrase to an [`Intent`], the fuzzy layer
//! resolves "maria" to the matching pending-approval worker, the
//! confirmation layer prompts "approve Maria for $25 charge — say
//! YES to confirm", and the runner dispatches the decision.
//!
//! ## Layering
//!
//! - [`grammar`] — pure-Rust phrase tokenizer (approve / deny /
//!   details / next / quit + optional worker name argument).
//! - [`intent`] — [`Intent`] enum + [`classify`] mapping recognized
//!   text → [`Intent`].
//! - [`fuzzy`] — Damerau-Levenshtein-style edit-distance worker
//!   name matching (≤2). Substring match before edit distance so
//!   "Maria" matches "Maria Hernandez".
//! - [`wake_word`] — gates listening on the prefix `argo` (case
//!   insensitive). Without the prefix, recognized text is
//!   discarded — battery + privacy.
//! - [`confirm`] — formats the safety confirmation prompt for
//!   approve / deny.
//! - [`recognizer`] — [`Recognizer`] trait + [`MockRecognizer`]
//!   (always-on, scriptable, used in tests).
//! - [`runner`] — wires everything together for one tick of the
//!   voice loop.
//!
//! Native audio capture (cpal) and STT (whisper-rs) sit behind the
//! optional `whisper-native` feature flag — mirrors the same
//! pattern as `mcp-servers/voice`. Without that feature, the CLI
//! handler falls back to the pre-existing TUI list view (per
//! brief acceptance "Mic-permission denial degrades to TUI list
//! cleanly").

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod confirm;
pub mod fuzzy;
pub mod grammar;
pub mod intent;
pub mod recognizer;
pub mod runner;
pub mod wake_word;

#[cfg(feature = "whisper-native")]
pub mod audio;
#[cfg(feature = "whisper-native")]
pub mod recognizer_whisper;

pub use confirm::{format_confirmation, parse_confirmation_response, ConfirmationResponse};
pub use fuzzy::{best_match, edit_distance};
pub use grammar::{tokens, Phrase};
pub use intent::{classify, Intent};
pub use recognizer::{MockRecognizer, Recognizer, RecognizerError};
pub use runner::{run_one_tick, PendingApproval, RunnerConfig, TickOutcome};
pub use wake_word::strip_wake_word;
