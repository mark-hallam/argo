//! Marker parsing helpers for prompt-skill relevance.
use crate::{Skill, TrustGrade};
use std::collections::HashSet;

pub(crate) fn values(skill: &Skill, keys: &[&str]) -> HashSet<String> {
    let mut out = HashSet::new();
    for line in skill
        .frontmatter
        .description
        .lines()
        .chain(skill.body.lines())
    {
        let trimmed = line
            .trim()
            .trim_start_matches('-')
            .trim()
            .to_ascii_lowercase();
        for key in keys {
            if let Some(rest) = trimmed
                .strip_prefix(key)
                .and_then(|s| s.trim_start().strip_prefix([':', '=']))
            {
                out.extend(tokens(rest));
            }
        }
    }
    out
}

pub(crate) fn marker_number(skill: &Skill, keys: &[&str]) -> u16 {
    values(skill, keys)
        .into_iter()
        .filter_map(|value| value.parse().ok())
        .max()
        .unwrap_or(0)
}

pub(crate) fn norm(input: &str) -> String {
    input
        .chars()
        .filter_map(|c| match c {
            c if c.is_ascii_alphanumeric() => Some(c.to_ascii_lowercase()),
            '-' | '_' => Some('_'),
            _ => None,
        })
        .collect()
}

pub(crate) fn parse_trust_label(input: &str) -> TrustGrade {
    match norm(input).as_str() {
        "builtin" => TrustGrade::Builtin,
        "verified" | "published_verified" => TrustGrade::PublishedVerified,
        "tenant" => TrustGrade::Tenant,
        "community" | "published_community" => TrustGrade::PublishedCommunity,
        "learned" | "unreviewed" | "learned_unreviewed" => TrustGrade::LearnedUnreviewed,
        _ => TrustGrade::PublishedVerified,
    }
}

fn tokens(input: &str) -> impl Iterator<Item = String> + '_ {
    input
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .map(norm)
        .filter(|s| !s.is_empty())
}
