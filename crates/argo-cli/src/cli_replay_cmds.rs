//! `argo replay` subcommand surface.
//!
//! Currently only carries `export`; future siblings (import,
//! re-verify) plug in alongside without disturbing the existing
//! `argo session replay` family.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ReplayCmd {
    /// Export a session as a self-contained tar/tar.gz bundle for
    /// offline replay or support hand-off. Bundle includes the audit
    /// chain in JSONL, a verify-chain summary, attached skill ids,
    /// per-action bundles, a manifest with a content checksum, and a
    /// short README. With `--sanitize`, JSON payloads are scrubbed of
    /// provider keys, JWTs, emails, and phone numbers before write.
    #[command(after_help = "Examples:\n  \
        argo replay export 01J9XYZ... --out /tmp/s.tar.gz\n  \
        argo replay export 01J9XYZ... --out /tmp/s.tar --uncompressed\n  \
        argo replay export 01J9XYZ... --out /tmp/s.tar.gz --sanitize")]
    Export {
        /// Session UUID to export.
        session_id: String,
        /// Output path (tar.gz by default; tar with `--uncompressed`).
        #[arg(long)]
        out: String,
        /// Emit a plain tar archive instead of gzip-compressed.
        #[arg(long)]
        uncompressed: bool,
        /// Redact provider keys, JWTs, emails, and phone numbers in
        /// JSON payloads before bundling. The manifest records
        /// `sanitized = true` so downstream consumers can distinguish.
        #[arg(long)]
        sanitize: bool,
    },
}
