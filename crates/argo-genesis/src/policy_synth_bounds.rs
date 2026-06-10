//! Server-side input bounds for the operator-supplied [`BoundariesSpec`].
//!
//! ## Why this exists (adversarial review of #2796, hardening note)
//!
//! [`crate::policy_synth`] iterates `escalation_topics` unbounded and
//! [`crate::policy_synth::normalize_topic`] allocates per character. In practice
//! the spec is bounded by the gateway body read and a single file write (no
//! OOM-grade amplification), but an uncapped, operator-supplied spec on the
//! policy-synthesis path is imprudent: a huge spec amplifies allocation in
//! `gated_class_names` / `synthesize_policy` / `boundaries_echo`.
//!
//! The fix is a cheap, explicit pre-flight: [`BoundariesSpec::validate_input_bounds`]
//! caps the escalation-topic count and the per-topic string length so the
//! gateway can reject (`400`) an adversarial spec before any synthesis runs.
//! It mirrors the existing [`crate::charter::CharterValidationError`] pattern so
//! callers can render a clear `400 Bad Request` message.

use crate::policy_synth::BoundariesSpec;

/// Max escalation topics accepted on one boundaries spec. `policy_synth`
/// allocates per topic, so the count is capped at the server input boundary.
/// 64 is far above any plausible legitimate configuration (the smart-default
/// archetypes set 1-2 topics) while still bounding adversarial amplification.
pub const MAX_ESCALATION_TOPICS: usize = 64;

/// Max characters in a single raw escalation topic (measured pre-normalization,
/// counting `char`s not bytes). 200 comfortably holds any honest topic phrase;
/// `normalize_topic` allocates per char, so this bounds the per-topic work.
pub const MAX_TOPIC_LEN: usize = 200;

/// Max access-scope entries accepted on one boundaries spec. There are only 4
/// [`crate::policy_synth::DataCategory`] variants × 2 access levels, so any
/// honest spec needs at most 8 entries — but `serde` does NOT dedup or bound the
/// `Vec<AccessScope>`, and `gated_class_names` / `boundaries_echo` iterate ALL
/// of them, so a flood of duplicate entries amplifies allocation on the exact
/// path this module guards. 16 leaves 2× headroom over any legitimate config
/// while bounding the adversarial case (security re-review of #2803).
pub const MAX_ACCESS_SCOPES: usize = 16;

/// Why a [`BoundariesSpec`] was rejected at the server input boundary.
///
/// Each variant identifies the offending bound so the gateway can produce a
/// clear `400 Bad Request` message (mirrors
/// [`crate::charter::CharterValidationError`]).
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BoundariesValidationError {
    /// More than [`MAX_ESCALATION_TOPICS`] escalation topics were supplied.
    #[error("escalation_topics count {count} exceeds max {max}", max = MAX_ESCALATION_TOPICS)]
    TooManyTopics {
        /// Observed topic count.
        count: usize,
    },
    /// A single escalation topic exceeded [`MAX_TOPIC_LEN`] characters.
    #[error("escalation_topics[{index}] length {len} chars exceeds max {max}", max = MAX_TOPIC_LEN)]
    TopicTooLong {
        /// Index of the offending topic (0-based).
        index: usize,
        /// Observed character length.
        len: usize,
    },
    /// More than [`MAX_ACCESS_SCOPES`] access-scope entries were supplied.
    #[error("access_scopes count {count} exceeds max {max}", max = MAX_ACCESS_SCOPES)]
    TooManyScopes {
        /// Observed access-scope count.
        count: usize,
    },
}

impl BoundariesSpec {
    /// Enforce server-side input bounds on the operator-supplied spec.
    ///
    /// Caps the escalation-topic count ([`MAX_ESCALATION_TOPICS`]), the
    /// per-topic character length ([`MAX_TOPIC_LEN`]), and the access-scope
    /// count ([`MAX_ACCESS_SCOPES`]) so an adversarial/huge spec cannot amplify
    /// allocation in policy synthesis. The gateway calls this before any
    /// synthesis runs and returns `400` on the first failing bound. Access
    /// scopes get a cap too: although there are only 4 fixed
    /// [`crate::policy_synth::DataCategory`] variants, `serde` neither dedups
    /// nor length-bounds the `Vec`, and `gated_class_names` / `boundaries_echo`
    /// iterate every entry — so a flood of duplicates is on the guarded path.
    ///
    /// # Errors
    /// Returns the first failing bound (topic count first, then per-topic
    /// length in index order).
    ///
    /// ```
    /// use argo_genesis::policy_synth::BoundariesSpec;
    ///
    /// let ok = BoundariesSpec {
    ///     escalation_topics: vec!["legal".into(), "compliance".into()],
    ///     ..Default::default()
    /// };
    /// assert!(ok.validate_input_bounds().is_ok());
    ///
    /// let too_many = BoundariesSpec {
    ///     escalation_topics: vec!["x".into(); 1000],
    ///     ..Default::default()
    /// };
    /// assert!(too_many.validate_input_bounds().is_err());
    /// ```
    pub fn validate_input_bounds(&self) -> Result<(), BoundariesValidationError> {
        if self.escalation_topics.len() > MAX_ESCALATION_TOPICS {
            return Err(BoundariesValidationError::TooManyTopics {
                count: self.escalation_topics.len(),
            });
        }
        if self.access_scopes.len() > MAX_ACCESS_SCOPES {
            return Err(BoundariesValidationError::TooManyScopes {
                count: self.access_scopes.len(),
            });
        }
        for (index, topic) in self.escalation_topics.iter().enumerate() {
            let len = topic.chars().count();
            if len > MAX_TOPIC_LEN {
                return Err(BoundariesValidationError::TopicTooLong { index, len });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_small_specs_pass() {
        assert!(BoundariesSpec::default().validate_input_bounds().is_ok());
        let spec = BoundariesSpec {
            escalation_topics: vec!["legal".into(), "compliance".into()],
            ..Default::default()
        };
        assert!(spec.validate_input_bounds().is_ok());
    }

    #[test]
    fn exactly_at_caps_pass() {
        let spec = BoundariesSpec {
            escalation_topics: vec!["a".repeat(MAX_TOPIC_LEN); MAX_ESCALATION_TOPICS],
            ..Default::default()
        };
        assert!(spec.validate_input_bounds().is_ok());
    }

    #[test]
    fn too_many_topics_rejected() {
        let spec = BoundariesSpec {
            escalation_topics: vec!["x".into(); MAX_ESCALATION_TOPICS + 1],
            ..Default::default()
        };
        assert_eq!(
            spec.validate_input_bounds(),
            Err(BoundariesValidationError::TooManyTopics {
                count: MAX_ESCALATION_TOPICS + 1,
            })
        );
    }

    #[test]
    fn over_long_topic_rejected_with_index() {
        let spec = BoundariesSpec {
            escalation_topics: vec!["legal".into(), "z".repeat(MAX_TOPIC_LEN + 5)],
            ..Default::default()
        };
        assert_eq!(
            spec.validate_input_bounds(),
            Err(BoundariesValidationError::TopicTooLong {
                index: 1,
                len: MAX_TOPIC_LEN + 5,
            })
        );
    }

    #[test]
    fn too_many_access_scopes_rejected() {
        use crate::policy_synth::{AccessLevel, AccessScope, DataCategory};
        // serde neither dedups nor length-bounds the Vec, so a flood of
        // duplicate scopes lands on the synthesis path. Over-cap → rejected.
        let scope = AccessScope {
            category: DataCategory::Financial,
            access: AccessLevel::ReadOnly,
        };
        let spec = BoundariesSpec {
            access_scopes: vec![scope; MAX_ACCESS_SCOPES + 1],
            ..Default::default()
        };
        assert_eq!(
            spec.validate_input_bounds(),
            Err(BoundariesValidationError::TooManyScopes {
                count: MAX_ACCESS_SCOPES + 1,
            })
        );
        // At-cap passes.
        let at_cap = BoundariesSpec {
            access_scopes: vec![
                AccessScope {
                    category: DataCategory::Financial,
                    access: AccessLevel::ReadOnly,
                };
                MAX_ACCESS_SCOPES
            ],
            ..Default::default()
        };
        assert!(at_cap.validate_input_bounds().is_ok());
    }

    #[test]
    fn topic_length_counts_chars_not_bytes() {
        // A multi-byte char must count as one toward the cap, so a string of
        // MAX_TOPIC_LEN multi-byte chars (well over MAX_TOPIC_LEN *bytes*) passes.
        let spec = BoundariesSpec {
            escalation_topics: vec!["é".repeat(MAX_TOPIC_LEN)],
            ..Default::default()
        };
        assert!(spec.validate_input_bounds().is_ok());
    }

    #[test]
    fn malformed_json_spec_handled_gracefully() {
        // serde #[serde(default)] on every field means a malformed / garbage
        // boundaries object does not panic: unknown keys are ignored and missing
        // fields default-empty, so the parsed spec is the (valid) empty spec.
        let parsed: BoundariesSpec =
            serde_json::from_str(r#"{"garbage": true, "escalation_topics": []}"#)
                .expect("garbage-but-typed boundaries parses to default-empty spec");
        assert!(parsed.is_empty());
        assert!(parsed.validate_input_bounds().is_ok());

        // A type-mismatched field is a hard parse error surfaced to the caller
        // (the gateway returns 400 `bad_request`), never a panic.
        let err =
            serde_json::from_str::<BoundariesSpec>(r#"{"escalation_topics": "not-an-array"}"#);
        assert!(err.is_err());
    }
}
