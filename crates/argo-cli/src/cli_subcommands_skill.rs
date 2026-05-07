use clap::{Args, Subcommand};
use std::path::PathBuf;

use super::cli_skill_conflicts_cmds::SkillConflictsCmd;
use super::cli_skill_review_cmds::SkillReviewCmd;

#[derive(Subcommand, Debug)]
pub enum SkillCmd {
    List {
        /// Output as JSON (one object per skill) for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`
        /// (renders a markdown table). `--json` wins if both are
        /// passed.
        #[arg(long, default_value = "text")]
        format: String,
        /// Sort order: `created-at` (default, descending),
        /// `name` (lexical, ascending), or `trust` (most-trusted
        /// first — Builtin/PublishedVerified ahead of
        /// LearnedUnreviewed).
        #[arg(long, default_value = "created-at")]
        sort_by: String,
        /// Truncate to the first N rows (after sorting).
        #[arg(long)]
        limit: Option<u32>,
        /// Include superseded versions (default keeps only the
        /// active skill per name). Mirrors `?include-superseded`.
        #[arg(long)]
        include_superseded: bool,
        /// Case-insensitive substring filter on `frontmatter.name`.
        /// Mirrors `?name-contains=…`.
        #[arg(long)]
        name_contains: Option<String>,
        /// Filter by lock status: `none` / `user-protected` /
        /// `content-locked`. Mirrors `?lock=…`.
        #[arg(long)]
        lock: Option<String>,
        /// Case-insensitive substring filter on the skill body.
        /// Mirrors `?body-contains=…`.
        #[arg(long)]
        body_contains: Option<String>,
        /// Filter to skills whose latest quality score is at
        /// least this value (`0..=100`). Skills with no score
        /// row are excluded from the filtered list.
        #[arg(long)]
        quality_min: Option<u8>,
    },
    /// Print just the count of active skills. Default: one int
    /// per line; `--json` emits `{count}`.
    Count {
        /// Include superseded versions in the count too.
        #[arg(long)]
        include_superseded: bool,
        #[arg(long)]
        json: bool,
    },
    View {
        name: String,
        /// Output as a single JSON object for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Verify a skill's body matches its frontmatter `provenance.hash`.
    /// Pass `--all` to verify every active skill in the workspace.
    Verify {
        #[arg(required_unless_present = "all")]
        name: Option<String>,
        /// Verify every active skill instead of one named skill.
        #[arg(long, conflicts_with = "name")]
        all: bool,
    },
    /// Create a new skill from CLI flags or stdin. Inserts a
    /// fresh row via `SkillStore::install`; rejects duplicate
    /// names (use `argo skill refine` to bump an existing
    /// skill instead).
    Create {
        name: String,
        /// One-line description for the frontmatter. Defaults
        /// to a stub (`<name> — created via argo skill create`).
        #[arg(long)]
        description: Option<String>,
        /// Skill body text. Mutually exclusive with
        /// `--from-file`; without either flag, body is read
        /// from stdin until EOF.
        #[arg(long, conflicts_with = "from_file")]
        body: Option<String>,
        /// Read the skill body from a path on disk.
        #[arg(long)]
        from_file: Option<String>,
        /// Version label for the new row. Defaults to "0.1".
        #[arg(long = "new-version", default_value = "0.1")]
        new_version: String,
    },
    /// Search the cross-tenant OCI skill registry.
    Search(super::cli_skill_search_cmds::SkillSearchCmd),
    /// Publish a local skill to the marketplace registry.
    Publish(SkillPublishCmd),
    /// Remove a mutable marketplace tag while preserving blobs.
    Unpublish(SkillUnpublishCmd),
    /// List marketplace tags for one skill.
    Tags(SkillTagsCmd),
    /// Show chronological skill invocation history.
    Runs {
        /// Skill name. Omit when using `--tenant` for cross-skill history.
        #[arg(
            value_name = "skill-name",
            required_unless_present = "tenant",
            conflicts_with = "tenant"
        )]
        name: Option<String>,
        /// Cross-skill tenant view.
        #[arg(long, conflicts_with = "name")]
        tenant: Option<String>,
        /// Filter to one worker id.
        #[arg(long)]
        worker: Option<String>,
        /// Wall-clock relative filter such as `24h`, `30m`, or `7d`.
        #[arg(long)]
        since: Option<String>,
        /// Maximum number of rows to return.
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Include failed and skipped invocations. Defaults to successes only.
        #[arg(long)]
        include_failed: bool,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
    Versions(super::cli_skill_ab_cmds::SkillVersionsCmd),
    AbTest(super::cli_skill_ab_cmds::SkillAbTestCmd),
    AbStatus(super::cli_skill_ab_cmds::SkillAbStatusCmd),
    AbPromote(super::cli_skill_ab_cmds::SkillAbPromoteCmd),
    /// Drive the learned-skill review gate (#681 lifecycle):
    /// list pending proposals + approve / reject them. Without
    /// this surface the lifecycle gate is theoretical — agents
    /// can propose skills via `propose_for_review`, but
    /// operators have no way to act on the queue.
    Review {
        #[command(subcommand)]
        cmd: SkillReviewCmd,
    },
    /// Inspect + resolve workspace ↔ library skill name
    /// collisions surfaced by the H-4b layered lookup
    /// (Stream `claude-skill-conflict-resolution`).
    Conflicts {
        #[command(subcommand)]
        cmd: SkillConflictsCmd,
    },
    /// Append a hint to the active skill's body and supersede it
    /// with a fresh version. Manual mutator — the score-driven
    /// version (run bench, accept only when the new score wins)
    /// lands once `forge optimize` learns to mutate.
    Refine {
        name: String,
        /// Text to append to the skill body, under a new
        /// `## Refinement` section.
        #[arg(long)]
        hint: String,
        /// Version label for the refined skill. Defaults to
        /// bumping the existing version's last numeric segment
        /// (e.g. `0.1` → `0.2`). `--version` would clash with
        /// clap's auto-version flag, hence the rename.
        #[arg(long = "new-version")]
        new_version: Option<String>,
    },
    Install {
        source: String,
    },
    /// Run cohort-based Dreams distillation over recent
    /// trajectories and emit a per-cohort summary. Reads
    /// `TrajectoryInput` JSON array from stdin (the producer is
    /// out of v1 scope — a nightly tick will land later).
    /// Brief: `claude-dreams-skill-distillation`.
    DistillNow {
        /// Worker id to charge against the per-day Dreams cap
        /// shared with `argo memory dream-now`. Defaults to
        /// `skill-distillation` so the operator can run
        /// off-band without picking a specific worker.
        #[arg(long, default_value = "skill-distillation")]
        worker: String,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
    /// Export one active skill (or an explicit `name:version`)
    /// into a portable `.skill.tar.gz` artifact.
    Export {
        /// Skill name, optionally suffixed as `name:version`.
        name: String,
        /// Output `.skill.tar.gz` path.
        #[arg(long)]
        out: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Export every active skill into a directory of portable
    /// `.skill.tar.gz` artifacts.
    ExportAll {
        /// Output directory.
        #[arg(long)]
        out: String,
        /// Filter to skills bundled by this vertical id.
        #[arg(long)]
        vertical: Option<String>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Import a portable `.skill.tar.gz` artifact into the local
    /// skill store.
    Import {
        /// Artifact path.
        path: String,
        /// Rename on import; required to avoid duplicate-name
        /// conflicts when the original name is already installed.
        #[arg(long)]
        name: Option<String>,
        /// Resolve the target skill store as if running from this
        /// workspace path.
        #[arg(long)]
        workspace: Option<String>,
        /// Validate and print what would be installed without
        /// writing the store.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    Lock {
        name: String,
    },
    Unlock {
        name: String,
    },
    /// Show the latest quality score (0..=100) + inputs for one
    /// skill (`claude-skill-quality-judge`).
    Quality {
        /// Skill name (active version is used).
        name: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// List every scored skill, worst first. `--threshold` keeps
    /// rows with score ≤ threshold; `--worst` keeps only the
    /// lowest-scoring N.
    QualityList {
        /// Optional ceiling — only rows with `score <=
        /// threshold` are emitted.
        #[arg(long)]
        threshold: Option<u8>,
        /// Truncate to the lowest-scoring N rows.
        #[arg(long)]
        worst: Option<u32>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// ASCII sparkline of the past `--days` daily quality scores
    /// for one skill (gaps render as `·`).
    QualityTrend {
        /// Skill name (active version is used).
        name: String,
        /// Days to render (default 30, max 365).
        #[arg(long)]
        days: Option<u32>,
        /// Emit JSON envelope (per-day buckets + sparkline).
        #[arg(long)]
        json: bool,
    },
    /// Force a recompute of the quality score from operator-
    /// provided inputs. Use `--all` to recompute every active
    /// skill (uses defaults for the inputs — primarily a
    /// scheduler stand-in until the background tick lands).
    QualityRecompute {
        /// Skill name to recompute. Required unless `--all`.
        #[arg(required_unless_present = "all")]
        name: Option<String>,
        /// Recompute every active skill.
        #[arg(long, conflicts_with = "name")]
        all: bool,
        /// Operator-provided success rate (0.0..=1.0). Defaults
        /// to 1.0 when omitted.
        #[arg(long)]
        success_rate: Option<f64>,
        /// Operator-provided mean latency in milliseconds.
        #[arg(long)]
        avg_latency_ms: Option<f64>,
        /// Operator-provided retry count.
        #[arg(long)]
        retry_count: Option<u32>,
        /// Operator-provided p99 latency in milliseconds.
        #[arg(long)]
        p99_latency_ms: Option<f64>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args, Debug)]
pub struct SkillPublishCmd {
    /// Local active skill name to publish.
    pub name: String,
    /// Registry reference. `mock://path` writes to a filesystem registry.
    #[arg(long, default_value = "mock://.argo/skill-registry")]
    pub registry: String,
    /// Marketplace tag. Defaults to the source skill version.
    #[arg(long)]
    pub tag: Option<String>,
    /// Read a SKILL.md file instead of the active local skill row.
    #[arg(long)]
    pub manifest: Option<PathBuf>,
    /// Print the bundle plan without writing files or calling a registry.
    #[arg(long)]
    pub dry_run: bool,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SkillUnpublishCmd {
    /// Marketplace reference in `<name>:<tag>` form.
    pub reference: String,
    /// Registry reference. `mock://path` reads a filesystem registry.
    #[arg(long, default_value = "mock://.argo/skill-registry")]
    pub registry: String,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SkillTagsCmd {
    /// Skill name whose marketplace tags should be listed.
    pub name: String,
    /// Registry reference. `mock://path` reads a filesystem registry.
    #[arg(long, default_value = "mock://.argo/skill-registry")]
    pub registry: String,
    /// Maximum number of tags to list.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}
