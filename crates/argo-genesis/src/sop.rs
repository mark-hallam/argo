//! SOP-upload comprehension gate (DEC-025 #4 + #5, Genesis P6 — the moat).
//!
//! P4 ([`crate::comprehension`]) proves a teammate understands its **own role
//! contract** — generic governance knowledge gradeable without any uploaded
//! document. P6 adds the investor-defining differentiator: the operator may
//! drop **one** Standard-Operating-Procedure / playbook document (text or PDF)
//! at onboarding, and the teammate must demonstrate it **comprehended that
//! specific document** — not generic role knowledge — before it can be hired.
//!
//! ## The pipeline (pure half lives here; grading + I/O in the gateway)
//!
//! ```text
//! SOP doc (text|PDF) → ingest_sop()  → SopKnowledge { tenant-scoped chunks }
//!                    → build_sop_quiz() → questions keyed on DOC-SPECIFIC terms
//!                    → (gateway) run_eval(ContractSubject{ knowledge_text }, quiz)
//!                    → SopComprehensionOutcome → 5th axis of GateVerdict::compose
//! ```
//!
//! The PDF→text step reuses the proven `pdf_extract` path already wired in
//! [`crate::job_description`] (so there is **no parallel ingestion path**); the
//! grading reuses the SAME canonical `argo_eval::run_eval` + deterministic
//! `ContractSubject` the eval-floor and P4 comprehension gates use (so there is
//! **no parallel scorer** and `argo-genesis` gains no `argo-eval` edge). This
//! module owns only the provider-free, I/O-free core.
//!
//! ## Why this is a real moat, not theater — the four adversarial properties
//!
//! 1. **No hire without genuine comprehension.** The quiz is graded by feeding
//!    each question through the eval runner with the subject being the
//!    teammate's **ingested SOP knowledge text**. A teammate that genuinely
//!    absorbed the SOP answers every question with text containing the
//!    doc-specific terms (pass); a teammate that did NOT ingest it has empty /
//!    role-only knowledge and so the doc-specific keywords are absent (fail).
//!    The floor is **all-must-pass** (every question's keywords must appear),
//!    and an empty quiz over a present SOP is itself a fail — so a SOP that
//!    yields no gradeable question fails closed.
//! 2. **Quiz grounded in the doc.** Every question's expected keywords are
//!    **salient terms extracted from the ingested document** (rare, distinctive,
//!    multi-occurrence tokens), not constants and not role-derived. A
//!    question's answer only appears in the doc, so a teammate cannot guess it
//!    from its role. [`build_sop_quiz`] never emits a question whose keyword
//!    also appears in the supplied role text (it filters role-overlapping
//!    terms), so the quiz discriminates "absorbed THIS doc" from "knows the
//!    generic role".
//! 3. **Tenant-scoped ingestion.** [`SopKnowledge`] is serialized to
//!    `knowledge/sop.md` + `knowledge/sop-quiz.toml` UNDER the teammate's
//!    `.genesis/<tenant_slug>/<vertical_id>/` tree (the gateway write path),
//!    which is filesystem-isolated per tenant exactly like P5's tenant-match —
//!    so an uploaded SOP and its derived quiz are visible only to the owning
//!    tenant; a different tenant resolves a different directory and never reads
//!    it.
//! 4. **Discrimination preserved.** The SAME quiz that clears a teammate whose
//!    knowledge contains the ingested SOP scores 0 against a teammate whose
//!    knowledge is empty or role-only — proven by same-crate tests.
//!
//! ## Optionality (DEC-025 #5)
//!
//! A SOP is **optional** for v1 (text role + *optionally* one SOP doc). When no
//! SOP is uploaded the axis is *not applicable* and passes vacuously
//! ([`SopComprehensionOutcome::not_applicable`]) — the teammate is still gated
//! by the eval-floor + P4 comprehension + safety + contract axes. When a SOP IS
//! uploaded, the axis is fail-closed: the teammate must pass the doc quiz.

use serde::{Deserialize, Serialize};

/// Minimum length (chars, after trim) of usable extracted SOP text. Shorter
/// than this is treated as an empty / image-only / junk upload and rejected by
/// [`ingest_sop`] so a degenerate doc cannot silently produce an un-gradeable
/// (and therefore fail-closed) quiz that the operator can't diagnose.
pub const MIN_SOP_CHARS: usize = 80;

/// Target characters per knowledge chunk. SOP knowledge is split into
/// paragraph-ish chunks so retrieval / display is bounded; grading concatenates
/// them, so the exact boundary does not affect the gate — it only bounds memory
/// growth and keeps each stored chunk human-readable.
const CHUNK_TARGET_CHARS: usize = 600;

/// Number of doc-grounded questions the quiz asks. Small + closed-form,
/// mirroring the P4 comprehension gate's five-question shape; every one must
/// pass.
const SOP_QUIZ_LEN: usize = 4;

/// Minimum salient-term length. Terms shorter than this are too generic to
/// distinguish "read THIS doc" from "knows English", so they are never quiz
/// keywords.
const MIN_TERM_LEN: usize = 5;

/// Tenant-scoped knowledge ingested from one uploaded SOP/playbook document.
///
/// Persisted (by the gateway) under the teammate's `.genesis/<tenant>/<id>/`
/// tree, so it inherits P5's filesystem tenant isolation. The `chunks` are the
/// human-readable, provenance-tagged units stored as the teammate's knowledge;
/// `joined_text()` is the lowercased concatenation the deterministic eval
/// subject answers the quiz with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SopKnowledge {
    /// A stable label for the source document (operator-supplied filename, or a
    /// generic fallback). Surfaced on the review screen + audit row.
    pub source_label: String,
    /// The ingested, chunked document body. Every chunk is *untrusted data*
    /// (provenance: an uploaded document, NOT the operator's charter) — the
    /// prompt-injection trust boundary (design P6 (a)) tags it as such; this
    /// type carries the content, the assembler tags the provenance.
    pub chunks: Vec<String>,
}

impl SopKnowledge {
    /// The lowercased concatenation of every chunk — the text the deterministic
    /// eval subject "answers" each quiz question with. A teammate that ingested
    /// the SOP returns this (so the doc-specific keywords match); an un-ingested
    /// teammate returns empty knowledge (so they do not).
    #[must_use]
    pub fn joined_text(&self) -> String {
        self.chunks.join("\n").to_lowercase()
    }

    /// `true` when no usable knowledge was ingested (no chunks / all empty).
    /// An empty knowledge body cannot answer a doc-grounded quiz, so it is the
    /// honest fail-closed signal.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.iter().all(|c| c.trim().is_empty())
    }
}

/// One doc-grounded quiz question. Shape mirrors
/// [`crate::comprehension::ComprehensionQuestion`] so the gateway grades both
/// through the IDENTICAL `argo_eval::run_eval` + `SubstringContains` path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SopQuestion {
    /// Stable question id (becomes the `EvalTask` name).
    pub id: String,
    /// The closed-form question text.
    pub question: String,
    /// Lowercased salient terms extracted FROM the document; ALL must appear in
    /// the teammate's ingested knowledge (AND semantics) for the question to
    /// pass. These are doc-specific — absent from generic role text — so the
    /// answer cannot be guessed from the role.
    pub expected_keywords: Vec<String>,
}

/// Outcome of running the SOP-comprehension quiz, mirroring
/// [`crate::comprehension::ComprehensionOutcome`] in shape so
/// [`crate::GateVerdict::compose`] treats every deterministic check identically.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SopComprehensionOutcome {
    /// Whether a SOP was uploaded at all. When `false`, the axis is *not
    /// applicable* (DEC-025 #5: SOP is optional) and passes vacuously.
    pub applicable: bool,
    /// Quiz questions whose every doc-specific keyword appeared in the
    /// teammate's ingested knowledge.
    pub passed: usize,
    /// Total quiz questions asked.
    pub total: usize,
    /// `passed / total` (0.0 when `total == 0`).
    pub pass_rate: f64,
    /// `true` iff the axis clears: either no SOP was uploaded (not applicable),
    /// OR a SOP was uploaded AND EVERY quiz question passed AND at least one ran.
    /// A present-but-unanswerable SOP (zero questions, or any miss) is a fail —
    /// the moat is fail-closed when a doc is supplied.
    pub passed_floor: bool,
}

impl SopComprehensionOutcome {
    /// The not-applicable outcome: no SOP was uploaded. Passes vacuously so the
    /// optional-SOP path is unaffected (the other four axes still gate).
    #[must_use]
    pub const fn not_applicable() -> Self {
        Self {
            applicable: false,
            passed: 0,
            total: 0,
            pass_rate: 0.0,
            passed_floor: true,
        }
    }

    /// Build the applicable outcome from a raw pass/total count, deriving the
    /// rate + all-must-pass floor verdict. A SOP that produced zero gradeable
    /// questions (`total == 0`) fails closed — a present document that can't be
    /// quizzed must not wave a teammate through.
    #[must_use]
    pub fn from_counts(passed: usize, total: usize) -> Self {
        let pass_rate = if total == 0 {
            0.0
        } else {
            passed as f64 / total as f64
        };
        let passed_floor = total > 0 && passed == total;
        Self {
            applicable: true,
            passed,
            total,
            pass_rate,
            passed_floor,
        }
    }
}

/// Ingest one uploaded SOP/playbook document body into tenant-scoped knowledge.
///
/// `raw_text` is the already-extracted document text (the gateway runs the
/// proven `pdf_extract` path from [`crate::job_description`] for PDFs, and uses
/// the body verbatim for plain text — there is no parallel extractor). This
/// function is provider- and I/O-free: it normalises whitespace, rejects an
/// empty/too-short doc, and chunks the body into bounded, human-readable units.
///
/// # Errors
///
/// Returns [`crate::GenesisError::Validation`] when the extracted text is
/// shorter than [`MIN_SOP_CHARS`] after trimming (empty upload, image-only PDF
/// the gateway's extractor returned blank for, or junk) — a doc that cannot be
/// meaningfully quizzed must fail loudly at ingest, not silently produce an
/// un-gradeable quiz.
///
/// # Examples
///
/// ```
/// # fn demo() -> Result<(), argo_genesis::GenesisError> {
/// let sop = argo_genesis::ingest_sop(
///     "Refund Policy\n\nApprove refunds under fifty dollars automatically. \
///      Escalate any refund above five hundred dollars to the regional \
///      finance manager. Never issue a chargeback without a ticket reference.",
///     "refund-policy.txt",
/// )?;
/// assert!(!sop.is_empty());
/// # Ok(()) }
/// ```
pub fn ingest_sop(raw_text: &str, source_label: &str) -> Result<SopKnowledge, crate::GenesisError> {
    let normalised = normalise_whitespace(raw_text);
    if normalised.chars().count() < MIN_SOP_CHARS {
        return Err(crate::GenesisError::Validation {
            stage: "sop.ingest",
            detail: format!(
                "uploaded SOP has only {} usable characters (minimum {MIN_SOP_CHARS}); \
                 if this is a PDF it may be image-only (OCR unsupported in v1) or empty",
                normalised.chars().count()
            ),
        });
    }
    let label = {
        let trimmed = source_label.trim();
        if trimmed.is_empty() {
            "uploaded-sop".to_string()
        } else {
            trimmed.to_string()
        }
    };
    Ok(SopKnowledge {
        source_label: label,
        chunks: chunk_text(&normalised),
    })
}

/// Extract text from an uploaded SOP PDF (the bytes of a `.pdf`).
///
/// Reuses the EXACT proven extractor [`crate::job_description`] uses
/// (`pdf_extract::extract_text_from_mem`) so there is **no parallel PDF path**
/// in the workspace. The caller (gateway) base64-decodes the upload, then hands
/// the bytes here; the returned text is fed straight to [`ingest_sop`].
///
/// # Errors
///
/// Returns [`crate::GenesisError::Validation`] when `pdf-extract` cannot decode
/// the bytes (malformed PDF). An image-only PDF decodes to whitespace, which
/// [`ingest_sop`] then rejects with its own image-only hint — so the two-step
/// (extract → ingest) gives the operator a precise diagnosis either way.
///
/// # Examples
///
/// ```no_run
/// # fn demo() -> Result<(), argo_genesis::GenesisError> {
/// let pdf_bytes: Vec<u8> = std::fs::read("/tmp/sop.pdf").unwrap_or_default();
/// let text = argo_genesis::extract_sop_pdf_text(&pdf_bytes)?;
/// let sop = argo_genesis::ingest_sop(&text, "sop.pdf")?;
/// # let _ = sop; Ok(()) }
/// ```
pub fn extract_sop_pdf_text(bytes: &[u8]) -> Result<String, crate::GenesisError> {
    pdf_extract::extract_text_from_mem(bytes).map_err(|e| crate::GenesisError::Validation {
        stage: "sop.pdf.extract",
        detail: format!("pdf extraction failed: {e}"),
    })
}

/// Build the doc-grounded comprehension quiz from ingested SOP knowledge,
/// excluding any term that also appears in `role_text` so the quiz proves
/// "absorbed THIS document", not "knows the generic role".
///
/// Extracts the most salient terms from the document — distinctive, non-stopword
/// tokens, ranked by a (frequency × length) salience so domain nouns like a
/// dollar threshold word, a named approver role, or a policy term surface — and
/// builds up to [`SOP_QUIZ_LEN`] closed-form questions, each requiring one
/// doc-specific term. `role_text` is the teammate's charter + action vocabulary
/// (lowercased); any salient term already present there is dropped so the quiz
/// keyword is genuinely doc-specific (a term in BOTH the SOP and the role can't
/// distinguish ingestion from role knowledge).
///
/// Returns an **empty** quiz when the document yields no role-distinct salient
/// term. An empty quiz over a present SOP is a fail-closed signal
/// ([`SopComprehensionOutcome::from_counts`] with `total == 0` → `passed_floor =
/// false`), so a doc that is entirely generic-role boilerplate cannot wave a
/// teammate through.
///
/// # Examples
///
/// ```
/// # fn demo() -> Result<(), argo_genesis::GenesisError> {
/// let sop = argo_genesis::ingest_sop(
///     "Escalate any refund above five hundred dollars to the regional finance \
///      manager named Priya. Never issue a chargeback without a ticket reference \
///      called an RMA. Apply the restocking surcharge on returned merchandise.",
///     "policy.txt",
/// )?;
/// let quiz = argo_genesis::build_sop_quiz(&sop, "be my support agent");
/// assert!(!quiz.is_empty(), "a content-rich SOP yields doc-grounded questions");
/// // Every keyword is doc-specific (it appears in the ingested knowledge).
/// let knowledge = sop.joined_text();
/// for q in &quiz {
///     for kw in &q.expected_keywords {
///         assert!(knowledge.contains(kw.as_str()));
///     }
/// }
/// # Ok(()) }
/// ```
#[must_use]
pub fn build_sop_quiz(knowledge: &SopKnowledge, role_text: &str) -> Vec<SopQuestion> {
    let role_lower = role_text.to_lowercase();
    let terms = salient_terms(&knowledge.joined_text(), &role_lower);
    terms
        .into_iter()
        .take(SOP_QUIZ_LEN)
        .enumerate()
        .map(|(idx, term)| SopQuestion {
            id: format!("sop_comprehension_{idx}"),
            question: format!(
                "From the procedure document you were given, what does it say about \
                 \"{term}\"? Answer using the document's own wording."
            ),
            expected_keywords: vec![term],
        })
        .collect()
}

/// Collapse runs of whitespace (incl. the form-feeds + stray newlines PDF
/// extraction emits) into single spaces, preserving paragraph breaks as `\n\n`.
fn normalise_whitespace(raw: &str) -> String {
    let mut paragraphs: Vec<String> = Vec::new();
    for block in raw.split("\n\n") {
        let collapsed = block.split_whitespace().collect::<Vec<_>>().join(" ");
        if !collapsed.is_empty() {
            paragraphs.push(collapsed);
        }
    }
    paragraphs.join("\n\n")
}

/// Split normalised text into bounded, human-readable chunks (~[`CHUNK_TARGET_CHARS`]
/// each), never splitting mid-paragraph unless a single paragraph already
/// exceeds the target.
fn chunk_text(normalised: &str) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for para in normalised.split("\n\n") {
        if current.is_empty() {
            current.push_str(para);
        } else if current.chars().count() + para.chars().count() + 2 <= CHUNK_TARGET_CHARS {
            current.push_str("\n\n");
            current.push_str(para);
        } else {
            chunks.push(std::mem::take(&mut current));
            current.push_str(para);
        }
    }
    if !current.trim().is_empty() {
        chunks.push(current);
    }
    if chunks.is_empty() {
        chunks.push(normalised.to_string());
    }
    chunks
}

/// Extract the document's most salient terms, ranked by salience and excluding
/// any term present in `role_lower` (so quiz keywords are doc-specific).
///
/// Salience = `occurrences × term_len`: a distinctive multi-occurrence domain
/// word (a named role, a policy noun, a threshold word) outranks an incidental
/// long word. Deterministic: stable sort by salience desc, then the term itself,
/// so the same doc always yields the same quiz (the gate is repeatable).
fn salient_terms(doc_lower: &str, role_lower: &str) -> Vec<String> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for tok in tokenize(doc_lower) {
        if tok.chars().count() >= MIN_TERM_LEN && !is_stopword(&tok) {
            *counts.entry(tok).or_insert(0) += 1;
        }
    }
    // Drop terms that also appear in the role text — they can't distinguish
    // "read the doc" from "knows the role".
    let role_tokens: BTreeSet<String> = tokenize(role_lower).into_iter().collect();
    let mut ranked: Vec<(String, usize)> = counts
        .into_iter()
        .filter(|(term, _)| !role_tokens.contains(term))
        .collect();
    ranked.sort_by(|a, b| {
        let sa = a.1 * a.0.chars().count();
        let sb = b.1 * b.0.chars().count();
        sb.cmp(&sa).then_with(|| a.0.cmp(&b.0))
    });
    ranked.into_iter().map(|(term, _)| term).collect()
}

/// Split into lowercased contiguous-alphabetic tokens. Matches the eval
/// subject's tokenization posture (the behaviour/knowledge contract is only
/// lowercased and keeps boundaries), so a token is a substring of the knowledge
/// text iff its source word is. Numbers are excluded — a bare digit run is a
/// weak comprehension keyword (a teammate parroting "500" proves little); spelt
/// thresholds ("five hundred dollars") survive as alphabetic tokens.
fn tokenize(raw: &str) -> Vec<String> {
    raw.split(|c: char| !c.is_ascii_alphabetic())
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

/// Common English words that carry no domain signal — never quiz keywords.
fn is_stopword(word: &str) -> bool {
    const STOPWORDS: &[&str] = &[
        "about", "above", "after", "again", "against", "always", "another", "because", "before",
        "being", "below", "between", "could", "doing", "during", "every", "first", "found", "from",
        "given", "having", "into", "never", "other", "should", "since", "their", "there", "these",
        "thing", "those", "through", "under", "until", "using", "where", "which", "while", "would",
        "your", "yours", "without", "within", "value", "values", "document", "procedure", "policy",
        "step", "steps", "process", "must", "shall", "will", "when", "then", "than", "with", "that",
        "this",
    ];
    STOPWORDS.contains(&word)
}

#[cfg(test)]
#[path = "sop_tests.rs"]
mod tests;
