use clap::Subcommand;

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
        /// Sort order: `created-at` (default, descending) or
        /// `name` (lexical, ascending).
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
    Lock {
        name: String,
    },
    Unlock {
        name: String,
    },
}
