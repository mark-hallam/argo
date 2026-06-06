//! Non-persistent sandbox quiz session — the "show me what you learned" gate
//! before Hire (Genesis Phase 2; memory [[genesis-is-ai-teammate-onboarding-studio]]).
//!
//! A real manager doesn't hand a new hire a stack of SOPs and turn them loose on
//! live tickets — they run a comprehension check first ("based on what you read,
//! how would you handle X?"), correct a wrong assumption, and re-test. This
//! module is the answering half of that loop: the to-be-hired teammate is
//! instantiated in a **sandbox** (no production tenant slot, no persistence
//! beyond the Genesis session) and answers the operator's free-form question by
//! **retrieving** over the documents already ingested via
//! [`crate::onboarding_documents`]. The gateway route
//! (`routes_genesis_quiz`) is the HTTP + audit shim around this pure core.
//!
//! ## No theater — retrieval, not fabrication
//!
//! The answer is assembled *only* from the uploaded document chunks. There is no
//! LLM call and no synthesis: the response is the highest-scoring chunk(s)
//! returned verbatim with their source label, so an investor watching the demo
//! sees the teammate quote the SOP it actually read. When no chunk shares a
//! meaningful term with the question — including the empty-memory case — the
//! teammate answers [`NO_KNOWLEDGE_ANSWER`] ("I don't have a SOP that covers
//! this") instead of inventing one. This is the same fail-loud honesty floor the
//! SOP moat and the comprehension gate enforce, surfaced at quiz time.
//!
//! ## No parallel retrieval path
//!
//! Scoring reuses the lowercased corpus exposed by
//! [`crate::OnboardingDocument::joined_text`] / its `chunks`, and the stopword +
//! salient-term shape mirrors [`crate::build_comprehension_questions`]'s scope
//! picker, so a quiz answer is graded against the SAME ingested text a hired
//! teammate would boot with — not a second copy.

use crate::OnboardingDocument;

/// The honest fallback when no uploaded document covers the question (including
/// the empty-memory case). Never fabricate — say so plainly.
pub const NO_KNOWLEDGE_ANSWER: &str =
    "I don't have a SOP that covers this. Upload a document with the answer, or \
     edit my memory, and ask me again.";

/// How many distinct source chunks a single answer may cite.
const MAX_CITATIONS: usize = 2;

/// Minimum length (chars) of a question term that counts toward retrieval. Short
/// tokens ("a", "is", "to") carry no topical signal.
const MIN_TERM_LEN: usize = 4;

/// One cited passage backing a quiz answer: which uploaded document it came from
/// and the chunk text quoted verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuizCitation {
    /// The uploaded document's source label (its filename).
    pub source_label: String,
    /// The chunk text quoted verbatim — the evidence the answer rests on.
    pub excerpt: String,
}

/// The sandbox teammate's answer to one operator quiz question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuizAnswer {
    /// `true` when at least one uploaded chunk matched the question; `false` is
    /// the honest "no SOP covers this" path (answer == [`NO_KNOWLEDGE_ANSWER`]).
    pub answered: bool,
    /// The answer text — quoted document content on a hit, the honest fallback
    /// otherwise.
    pub answer: String,
    /// The passages the answer cites (empty on the fallback path).
    pub citations: Vec<QuizCitation>,
}

impl QuizAnswer {
    /// The honest "no SOP covers this" answer (empty memory or no term overlap).
    #[must_use]
    fn no_knowledge() -> Self {
        Self {
            answered: false,
            answer: NO_KNOWLEDGE_ANSWER.to_string(),
            citations: Vec::new(),
        }
    }
}

/// Answer one free-form quiz question by retrieving over the teammate's uploaded
/// onboarding documents.
///
/// Each chunk is scored by how many DISTINCT meaningful question terms it
/// contains (case-insensitive). The top [`MAX_CITATIONS`] chunks with a non-zero
/// score become the citations, and their text is quoted verbatim as the answer.
/// When nothing scores — empty memory, or a question about something the
/// documents never mention — the teammate answers [`NO_KNOWLEDGE_ANSWER`] rather
/// than fabricating.
///
/// # Examples
///
/// ```
/// use argo_genesis::{answer_quiz_question, ingest_onboarding_document};
/// let doc = ingest_onboarding_document(
///     "Refunds above five hundred dollars require regional manager approval before the finance team can process them.",
///     "refund-policy.md",
/// ).unwrap();
/// let hit = answer_quiz_question("How do I handle a refund over $500?", &[doc.clone()]);
/// assert!(hit.answered);
/// assert!(hit.answer.to_lowercase().contains("regional manager"));
///
/// let miss = answer_quiz_question("What is our parental leave policy?", &[doc]);
/// assert!(!miss.answered);
/// ```
#[must_use]
pub fn answer_quiz_question(question: &str, documents: &[OnboardingDocument]) -> QuizAnswer {
    let terms = question_terms(question);
    if terms.is_empty() {
        return QuizAnswer::no_knowledge();
    }

    // (score, document index, chunk index) for every chunk that matches at least
    // one question term. Higher score = more distinct terms overlap.
    let mut scored: Vec<(usize, usize, usize)> = Vec::new();
    for (doc_idx, doc) in documents.iter().enumerate() {
        for (chunk_idx, chunk) in doc.chunks.iter().enumerate() {
            let lower = chunk.to_lowercase();
            let score = terms.iter().filter(|t| lower.contains(*t)).count();
            if score > 0 {
                scored.push((score, doc_idx, chunk_idx));
            }
        }
    }

    if scored.is_empty() {
        return QuizAnswer::no_knowledge();
    }

    // Best score first; ties keep document/chunk order for stable output.
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

    let citations: Vec<QuizCitation> = scored
        .iter()
        .take(MAX_CITATIONS)
        .map(|&(_, doc_idx, chunk_idx)| QuizCitation {
            source_label: documents[doc_idx].source_label.clone(),
            excerpt: documents[doc_idx].chunks[chunk_idx].clone(),
        })
        .collect();

    let answer = citations
        .iter()
        .map(|c| c.excerpt.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    QuizAnswer {
        answered: true,
        answer,
        citations,
    }
}

/// Suggest 3-5 starter quiz questions grounded in the uploaded content + role
/// text, so the operator has a one-click way to probe what the teammate learned.
///
/// The questions are templated around the most salient terms found in the
/// documents (and the role text as a fallback) so they reference material the
/// teammate can actually answer from — not a generic constant list. Returns an
/// empty vec when there is nothing ingested to ask about (the SPA then shows
/// "upload a document first").
///
/// # Examples
///
/// ```
/// use argo_genesis::{ingest_onboarding_document, suggested_quiz_questions};
/// let doc = ingest_onboarding_document(
///     "Refund requests above five hundred dollars require regional manager approval. \
///      Escalate any chargeback dispute to the finance team within 24 hours.",
///     "policy.md",
/// ).unwrap();
/// let qs = suggested_quiz_questions(&[doc], "support teammate");
/// assert!(!qs.is_empty());
/// assert!(qs.len() <= 5);
/// ```
#[must_use]
pub fn suggested_quiz_questions(documents: &[OnboardingDocument], role_text: &str) -> Vec<String> {
    let mut terms = salient_terms(documents, role_text);
    terms.truncate(5);
    if terms.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<String> = terms
        .into_iter()
        .map(|t| format!("Based on what you read, how would you handle a {t} request?"))
        .collect();
    // A second, broader prompt the operator can lean on first.
    out.insert(
        0,
        "Walk me through the most important thing you learned from the documents I uploaded."
            .to_string(),
    );
    out.truncate(5);
    out
}

/// Extract distinct, lowercased, meaningful terms from a question for scoring.
/// Filters short tokens and common English stopwords so retrieval keys off topic
/// words, not glue.
fn question_terms(question: &str) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for raw in question.split(|c: char| !c.is_ascii_alphanumeric()) {
        if raw.len() < MIN_TERM_LEN {
            continue;
        }
        let lower = raw.to_ascii_lowercase();
        if STOPWORDS.contains(&lower.as_str()) || seen.contains(&lower) {
            continue;
        }
        seen.push(lower);
    }
    seen
}

/// Rank the most salient topical terms across the ingested documents (longest,
/// frequent, non-stopword words win), falling back to the role text when nothing
/// is ingested. Used to seed grounded suggested questions.
fn salient_terms(documents: &[OnboardingDocument], role_text: &str) -> Vec<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    let corpus = if documents.is_empty() {
        role_text.to_string()
    } else {
        documents
            .iter()
            .map(OnboardingDocument::joined_text)
            .collect::<Vec<_>>()
            .join(" ")
    };
    for raw in corpus.split(|c: char| !c.is_ascii_alphabetic()) {
        if raw.len() < MIN_TERM_LEN + 1 {
            continue;
        }
        let lower = raw.to_ascii_lowercase();
        if STOPWORDS.contains(&lower.as_str()) {
            continue;
        }
        match counts.iter_mut().find(|(w, _)| *w == lower) {
            Some((_, n)) => *n += 1,
            None => counts.push((lower, 1)),
        }
    }
    // Most frequent first; ties to the longer (more specific) word.
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.len().cmp(&a.0.len())));
    counts.into_iter().map(|(w, _)| w).collect()
}

/// Common English stopwords stripped before scoring/ranking so retrieval keys
/// off topic words. Mirrors the spirit of the comprehension gate's stopword set.
const STOPWORDS: &[&str] = &[
    "that", "this", "with", "your", "into", "from", "have", "will", "they", "them", "what", "when",
    "then", "than", "been", "were", "being", "about", "would", "could", "should", "please",
    "their", "there", "which", "while", "every", "handle", "based", "read", "request", "above",
    "within", "also", "must", "make", "made", "does", "done", "over", "under", "after", "before",
];

#[cfg(test)]
#[path = "sandbox_session_tests.rs"]
mod tests;
