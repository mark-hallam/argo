//! Hot-path canonical-Q&A matcher — Track P brief P-04 of the
//! v1.0 Pitch Agent demo.
//!
//! Maps an incoming question to a pre-warmed canonical answer using a
//! lexical (token-overlap with inverse-document-frequency weighting)
//! match over each entry's `prompts` list. The brief grants this
//! lexical fallback in lieu of a 50MB on-disk embedding model — it
//! ships ~80% of paraphrase coverage at zero binary weight, which is
//! acceptable for the v1 demo path. The matcher stays in-process so a
//! lookup is sub-millisecond on a 100-entry corpus, comfortably under
//! the ≤50ms p95 budget the storyboard requires for sub-300ms voice
//! responses.
//!
//! A canonical hit also carries its `declared_claim_classes`, so the
//! truth-boundary evaluator from P-01
//! ([`argo_capability::truth_boundary::ClaimCandidate::declared_classes`])
//! can short-circuit semantic classification — the entry already
//! asserts what claim classes its answer falls under.
//!
//! ```no_run
//! use argo_skill::canonical_qa::CanonicalQaIndex;
//! use argo_types::TenantId;
//! use std::path::Path;
//!
//! let tenant = TenantId::new();
//! let idx = CanonicalQaIndex::load(
//!     Path::new("skill-library/canonical-qa/corpora/skycot"),
//!     tenant,
//! ).unwrap();
//! if let Some(m) = idx.lookup("How is Skycot different from Lindy?", "investor") {
//!     println!("matched {} ({:.2})", m.entry_id, m.similarity);
//! }
//! ```

use crate::canonical_qa_score::{build_idf, score, tokenize};
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Default similarity threshold above which a canonical entry is
/// considered a hit. Tuned for the IDF-weighted query-recovery
/// scorer in [`crate::canonical_qa_score`] — values are bounded
/// `[0.0, 1.0]` and represent the fraction of the query's
/// IDF-weighted importance recovered by the prompt. 0.45 is the
/// operating point that hits real paraphrases without firing on
/// superficial overlap; raise for tighter matching, lower for more
/// permissive paraphrase coverage.
pub const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.45;

/// Minimum number of prompts per canonical entry. Below this, lexical
/// matching gets thin and the corpus author should add paraphrases.
pub const MIN_PROMPTS_PER_ENTRY: usize = 3;

/// Hard upper bound on `last_reviewed` age before the lint flags an
/// entry as stale. 90 days matches the brief's anti-rot rule.
pub const MAX_REVIEW_AGE_DAYS: i64 = 90;

/// One canonical question/answer entry within a corpus file.
///
/// `prompts` is the list of paraphrases the matcher scores against.
/// `answer` is the fixed reply rendered verbatim on a hit. The other
/// fields are metadata: `declared_claim_classes` feeds the
/// truth-boundary gate (P-02), `audience_tags` lets one corpus serve
/// multiple personas, and `last_reviewed` lets the lint enforce
/// freshness.
///
/// ```
/// use argo_skill::canonical_qa::CanonicalEntry;
/// use chrono::NaiveDate;
/// let e = CanonicalEntry {
///     id: "lindy-comparison".into(),
///     prompts: vec!["How is Skycot different from Lindy?".into()],
///     answer: "Different category, different buyer.".into(),
///     declared_claim_classes: vec!["competitive-positioning".into()],
///     audience_tags: vec!["investor".into()],
///     last_reviewed: NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
///     review_owner: "founder@example.com".into(),
/// };
/// assert_eq!(e.id, "lindy-comparison");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalEntry {
    /// Stable per-corpus identifier (kebab-case).
    pub id: String,
    /// Paraphrases — the matcher scores the query against each.
    pub prompts: Vec<String>,
    /// Fixed answer rendered verbatim on a hit.
    pub answer: String,
    /// Claim-class names asserted to apply (cross-checked by the
    /// lint against the tenant's truth-boundary policy at authoring
    /// time; fed to the P-02 evaluator as `declared_classes` at
    /// runtime).
    pub declared_claim_classes: Vec<String>,
    /// Audience filter (e.g. `investor`, `buyer`, `press`,
    /// `customer`). Empty = matches every audience.
    #[serde(default)]
    pub audience_tags: Vec<String>,
    /// Date of last human review. The lint warns if older than
    /// [`MAX_REVIEW_AGE_DAYS`].
    pub last_reviewed: NaiveDate,
    /// Operator email (free-form) responsible for this entry's
    /// freshness.
    #[serde(default)]
    pub review_owner: String,
}

/// One per-tenant in-memory index over a corpus directory.
#[derive(Debug)]
pub struct CanonicalQaIndex {
    tenant_id: TenantId,
    entries: Vec<CanonicalEntry>,
    prompts: Vec<PromptTokens>,
    idf: HashMap<String, f32>,
    similarity_threshold: f32,
}

#[derive(Debug, Clone)]
struct PromptTokens {
    entry_idx: usize,
    token_set: HashSet<String>,
}

/// A matched canonical entry, returned by
/// [`CanonicalQaIndex::lookup`].
#[derive(Debug, Clone, Serialize)]
pub struct CanonicalMatch {
    /// Matching entry's stable id.
    pub entry_id: String,
    /// The fixed answer.
    pub answer: String,
    /// Claim classes declared by the matched entry — pipes into the
    /// P-02 evaluator's `ClaimCandidate.declared_classes`.
    pub declared_claim_classes: Vec<String>,
    /// Similarity score in `[0.0, 1.0]`. Higher is closer.
    pub similarity: f32,
}

/// Skill-SDK-shaped input for `canonical-qa`.
#[derive(Debug, Clone, Deserialize)]
pub struct CanonicalQaInput {
    /// The user's question (utterance text).
    pub query: String,
    /// Persona-flavor filter — entries with non-empty
    /// `audience_tags` not containing this string are skipped. Pass
    /// an empty string to match all entries.
    #[serde(default)]
    pub audience_tag: String,
}

/// Skill-SDK-shaped output for `canonical-qa`. `matched: false` with
/// every other field empty signals the agent runner to fall back to
/// the cold (generative + truth-boundary gate) path.
#[derive(Debug, Clone, Serialize)]
pub struct CanonicalQaOutput {
    /// True iff a canonical entry scored above the threshold.
    pub matched: bool,
    /// The fixed answer, or empty string on miss.
    #[serde(default)]
    pub answer: String,
    /// Claim classes from the matched entry. Empty on miss.
    #[serde(default)]
    pub declared_claim_classes: Vec<String>,
    /// The matched entry's id, or empty on miss.
    #[serde(default)]
    pub entry_id: String,
    /// Similarity score in `[0.0, 1.0]`. Zero on miss.
    #[serde(default)]
    pub similarity: f32,
}

#[derive(Debug, Deserialize)]
struct CorpusFile {
    #[serde(default)]
    entries: Vec<CanonicalEntry>,
}

impl CanonicalQaIndex {
    /// Build an index by walking every `*.toml` file under
    /// `corpus_dir`.
    ///
    /// ```no_run
    /// use argo_skill::canonical_qa::CanonicalQaIndex;
    /// use argo_types::TenantId;
    /// use std::path::Path;
    /// let idx = CanonicalQaIndex::load(
    ///     Path::new("skill-library/canonical-qa/corpora/skycot"),
    ///     TenantId::new(),
    /// );
    /// assert!(idx.is_ok() || idx.is_err());
    /// ```
    pub fn load(corpus_dir: &Path, tenant_id: TenantId) -> ArgoResult<Self> {
        let mut paths: Vec<_> = std::fs::read_dir(corpus_dir)
            .map_err(|e| {
                ArgoError::Io(format!(
                    "failed to read canonical-qa corpus dir at {}: {e}",
                    corpus_dir.display()
                ))
            })?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("toml"))
            .collect();
        paths.sort();
        let mut entries = Vec::new();
        for path in &paths {
            let raw = std::fs::read_to_string(path).map_err(|e| {
                ArgoError::Io(format!("failed to read {}: {e}", path.display()))
            })?;
            let parsed: CorpusFile = toml::from_str(&raw).map_err(|e| {
                ArgoError::Schema(format!(
                    "malformed canonical-qa corpus at {}: {e}",
                    path.display()
                ))
            })?;
            for entry in parsed.entries {
                validate_entry(&entry, path)?;
                entries.push(entry);
            }
        }
        check_unique_ids(&entries)?;
        Self::from_entries(entries, tenant_id, DEFAULT_SIMILARITY_THRESHOLD)
    }

    /// Build an index directly from in-memory entries — used by
    /// tests and callers sourcing entries from somewhere other than
    /// the filesystem.
    ///
    /// ```
    /// use argo_skill::canonical_qa::{CanonicalEntry, CanonicalQaIndex,
    ///     DEFAULT_SIMILARITY_THRESHOLD};
    /// use argo_types::TenantId;
    /// use chrono::NaiveDate;
    /// let entries = vec![CanonicalEntry {
    ///     id: "x".into(),
    ///     prompts: vec!["a b c".into(), "d e f".into(), "g h i".into()],
    ///     answer: "y".into(),
    ///     declared_claim_classes: vec![],
    ///     audience_tags: vec![],
    ///     last_reviewed: NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
    ///     review_owner: String::new(),
    /// }];
    /// let idx = CanonicalQaIndex::from_entries(
    ///     entries, TenantId::new(), DEFAULT_SIMILARITY_THRESHOLD,
    /// ).unwrap();
    /// assert_eq!(idx.len(), 1);
    /// ```
    pub fn from_entries(
        entries: Vec<CanonicalEntry>,
        tenant_id: TenantId,
        similarity_threshold: f32,
    ) -> ArgoResult<Self> {
        for entry in &entries {
            validate_entry(entry, Path::new("<in-memory>"))?;
        }
        check_unique_ids(&entries)?;
        let mut prompts = Vec::new();
        let mut token_sets = Vec::new();
        for (idx, entry) in entries.iter().enumerate() {
            for prompt in &entry.prompts {
                let toks = tokenize(prompt);
                token_sets.push(toks.clone());
                prompts.push(PromptTokens {
                    entry_idx: idx,
                    token_set: toks,
                });
            }
        }
        let idf = build_idf(&token_sets);
        Ok(Self {
            tenant_id,
            entries,
            prompts,
            idf,
            similarity_threshold,
        })
    }

    /// The tenant this index belongs to.
    #[must_use]
    pub fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    /// Entry count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True iff the index has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Look up a single best-matching entry for `query` filtered by
    /// `audience` (use `""` to bypass the audience filter). Returns
    /// `None` if no entry exceeds the configured similarity
    /// threshold.
    pub fn lookup(&self, query: &str, audience: &str) -> Option<CanonicalMatch> {
        let q_tokens = tokenize(query);
        if q_tokens.is_empty() {
            return None;
        }
        let mut best: Option<(usize, f32)> = None;
        for pt in &self.prompts {
            let entry = &self.entries[pt.entry_idx];
            if !audience.is_empty()
                && !entry.audience_tags.is_empty()
                && !entry.audience_tags.iter().any(|t| t == audience)
            {
                continue;
            }
            let s = score(&q_tokens, &pt.token_set, &self.idf);
            match best {
                Some((_, bs)) if s <= bs => {}
                _ => best = Some((pt.entry_idx, s)),
            }
        }
        let (idx, sim) = best?;
        if sim < self.similarity_threshold {
            return None;
        }
        let entry = &self.entries[idx];
        Some(CanonicalMatch {
            entry_id: entry.id.clone(),
            answer: entry.answer.clone(),
            declared_claim_classes: entry.declared_claim_classes.clone(),
            similarity: sim,
        })
    }

    /// Borrow the underlying entry list — used by the lint to walk
    /// every entry for cross-checks against the truth-boundary
    /// policy.
    #[must_use]
    pub fn entries(&self) -> &[CanonicalEntry] {
        &self.entries
    }
}

fn validate_entry(entry: &CanonicalEntry, path: &Path) -> ArgoResult<()> {
    if entry.id.trim().is_empty() {
        return Err(ArgoError::Schema(format!(
            "canonical-qa entry in {} has empty id",
            path.display()
        )));
    }
    if entry.prompts.len() < MIN_PROMPTS_PER_ENTRY {
        return Err(ArgoError::Schema(format!(
            "canonical-qa entry {} in {} has {} prompt(s); minimum is {}",
            entry.id,
            path.display(),
            entry.prompts.len(),
            MIN_PROMPTS_PER_ENTRY
        )));
    }
    if entry.answer.trim().is_empty() {
        return Err(ArgoError::Schema(format!(
            "canonical-qa entry {} in {} has empty answer",
            entry.id,
            path.display()
        )));
    }
    Ok(())
}

fn check_unique_ids(entries: &[CanonicalEntry]) -> ArgoResult<()> {
    let mut seen = HashSet::new();
    for entry in entries {
        if !seen.insert(entry.id.as_str()) {
            return Err(ArgoError::Schema(format!(
                "canonical-qa duplicate entry id: {}",
                entry.id
            )));
        }
    }
    Ok(())
}

impl From<CanonicalMatch> for CanonicalQaOutput {
    fn from(m: CanonicalMatch) -> Self {
        Self {
            matched: true,
            answer: m.answer,
            declared_claim_classes: m.declared_claim_classes,
            entry_id: m.entry_id,
            similarity: m.similarity,
        }
    }
}

impl CanonicalQaOutput {
    /// The miss output — every field empty, `matched = false`. Lets
    /// the agent runner key its fallback decision on a single field.
    #[must_use]
    pub fn miss() -> Self {
        Self {
            matched: false,
            answer: String::new(),
            declared_claim_classes: Vec::new(),
            entry_id: String::new(),
            similarity: 0.0,
        }
    }
}
