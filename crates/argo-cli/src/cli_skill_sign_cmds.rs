//! `argo skill {sign,verify-sig,keygen}` subcommand arguments.
use clap::Args;
use std::path::PathBuf;

/// `argo skill sign` — produce a cosign-compatible ed25519 signature
/// over a local skill's SKILL.md bytes and write the detached
/// signature artifact to the mock OCI registry.
#[derive(Args, Debug)]
pub struct SkillSignCmd {
    /// Active local skill name to sign. Omit when `--manifest` is set.
    #[arg(required_unless_present = "manifest")]
    pub name: Option<String>,
    /// PEM private key file produced by `argo skill keygen`.
    /// Required for v1; `--keyless` is deferred follow-up.
    #[arg(long)]
    pub key: Option<PathBuf>,
    /// Sigstore Fulcio keyless signing (deferred to follow-up).
    #[arg(long, conflicts_with = "key")]
    pub keyless: bool,
    /// Read SKILL.md bytes from a path instead of the active skill row.
    #[arg(long)]
    pub manifest: Option<PathBuf>,
    /// Override the published tag the signature is filed under
    /// (defaults to the skill's frontmatter `version`).
    #[arg(long)]
    pub tag: Option<String>,
    /// Mock OCI registry reference (must match `argo skill publish`).
    #[arg(long, default_value = "mock://.argo/skill-registry")]
    pub registry: String,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}

/// `argo skill verify-sig` — verify a published skill against its
/// detached signature artifact. The bare `verify` subcommand is
/// already taken by the body/provenance hash check (see
/// `cli_subcommands_skill.rs::SkillCmd::Verify`); this hyphenated form
/// avoids colliding with that surface.
#[derive(Args, Debug)]
pub struct SkillVerifySigCmd {
    /// Published reference as `<name>:<tag>` (e.g. `foo:0.1.0`).
    pub reference: String,
    /// PEM public key file. Required until keyless verify lands.
    #[arg(long)]
    pub key: Option<PathBuf>,
    /// Mock OCI registry reference.
    #[arg(long, default_value = "mock://.argo/skill-registry")]
    pub registry: String,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}

/// `argo skill keygen` — generate a fresh cosign-style ed25519 keypair
/// for testing and operator onboarding.
#[derive(Args, Debug)]
pub struct SkillKeygenCmd {
    /// Directory to write `cosign.key` + `cosign.pub` into.
    /// Defaults to the current working directory.
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}
