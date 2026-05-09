//! Clap surface for `argo cu` — computer-use trajectory inspection.
//! Lives in its own file so `cli_command_enum.rs` keeps a one-line
//! variant for `Cu`.
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum CuCmd {
    /// Reconstruct a recorded computer-use session as a navigable
    /// artifact (markdown trajectory; optional GIF). Mirrors `argo
    /// session replay --stream` shape but specialised for the CU
    /// per-action `browser.jsonl` + `step-NNN.png` bundle.
    Replay {
        /// CU session id — the `action_id` of the dispatch whose
        /// trajectory to render. Cross-tenant lookups return
        /// `NotFound`.
        session_id: String,
        /// Write markdown to PATH instead of stdout. Ignored when
        /// `--format json`.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Also encode the per-step screenshots as a GIF written
        /// to PATH. Default fps = 1 (CU steps are slow).
        #[arg(long)]
        gif: Option<PathBuf>,
        /// Frames per second for `--gif`. Bound `1..=30`.
        #[arg(long, default_value_t = 1)]
        gif_fps: u32,
        /// Restrict the rendered trajectory to steps `>= N`
        /// (1-indexed, inclusive).
        #[arg(long)]
        from: Option<u32>,
        /// Restrict the rendered trajectory to steps `<= N`
        /// (1-indexed, inclusive).
        #[arg(long)]
        to: Option<u32>,
        /// Embed screenshots as `data:image/png;base64,...` URLs
        /// rather than `file://` paths. Useful when the markdown
        /// will be shipped to a host that doesn't see the bundle
        /// directory.
        #[arg(long)]
        inline: bool,
        /// `markdown` (default) or `json`. JSON emits NDJSON,
        /// one-step-per-line, mirroring `session replay --stream`.
        #[arg(long, default_value = "markdown")]
        format: String,
    },
}
