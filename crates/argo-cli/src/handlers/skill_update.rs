//! `argo skill update <name> --min-trust-grade <grade>` — mutate
//! metadata on an active skill row. Currently only the skill-side
//! trust gate is wired (Stream `mission-skill-min-trust-grade-column`,
//! M6 precursor 1/3); future flag pass-throughs extend this command
//! instead of adding fresh subcommands.
use argo_skill::{SkillStore, TrustGrade};
use argo_types::{ArgoError, ArgoResult};

/// Run `argo skill update <name>` with the optional flags.
///
/// `min_trust_grade` accepts the same snake_case strings the
/// `TrustGrade` enum serializes to (`builtin`,
/// `published_verified`, `tenant`, `published_community`,
/// `learned_unreviewed`) plus the literal `none` / `null` /
/// `clear` to drop the column back to NULL ("no restriction").
/// When the flag is omitted the command is a no-op so future flags
/// compose cleanly.
pub async fn run(
    skills: &SkillStore,
    name: &str,
    min_trust_grade: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    if let Some(raw) = min_trust_grade {
        let parsed = parse_min_trust_grade(raw)?;
        skills.set_min_trust_grade(name, parsed).await?;
    }
    let current = skills.get_min_trust_grade(name).await?;
    if json {
        let label = current.map(grade_label);
        let payload = serde_json::json!({"name": name, "min_trust_grade": label});
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        let label = current.map_or("none".to_string(), |g| grade_label(g).to_string());
        println!("skill '{name}' min_trust_grade = {label}");
    }
    Ok(())
}

fn parse_min_trust_grade(raw: &str) -> ArgoResult<Option<TrustGrade>> {
    let trimmed = raw.trim().to_ascii_lowercase();
    if matches!(trimmed.as_str(), "none" | "null" | "clear" | "") {
        return Ok(None);
    }
    let grade = match trimmed.as_str() {
        "builtin" => TrustGrade::Builtin,
        "published_verified" | "published-verified" => TrustGrade::PublishedVerified,
        "tenant" => TrustGrade::Tenant,
        "published_community" | "published-community" => TrustGrade::PublishedCommunity,
        "learned_unreviewed" | "learned-unreviewed" => TrustGrade::LearnedUnreviewed,
        other => {
            return Err(ArgoError::Provider(format!(
                "--min-trust-grade '{other}' is not a recognised grade; \
                 expected one of: builtin, published_verified, tenant, \
                 published_community, learned_unreviewed, none"
            )));
        }
    };
    Ok(Some(grade))
}

fn grade_label(grade: TrustGrade) -> &'static str {
    match grade {
        TrustGrade::Builtin => "builtin",
        TrustGrade::PublishedVerified => "published_verified",
        TrustGrade::Tenant => "tenant",
        TrustGrade::PublishedCommunity => "published_community",
        TrustGrade::LearnedUnreviewed => "learned_unreviewed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_recognises_none_aliases() {
        for raw in ["none", "NULL", "Clear", " none ", ""] {
            assert_eq!(parse_min_trust_grade(raw).unwrap(), None, "raw={raw:?}");
        }
    }

    #[test]
    fn parse_round_trips_grade_labels() {
        for (raw, expected) in [
            ("builtin", TrustGrade::Builtin),
            ("published_verified", TrustGrade::PublishedVerified),
            ("published-verified", TrustGrade::PublishedVerified),
            ("tenant", TrustGrade::Tenant),
            ("published_community", TrustGrade::PublishedCommunity),
            ("learned_unreviewed", TrustGrade::LearnedUnreviewed),
        ] {
            assert_eq!(
                parse_min_trust_grade(raw).unwrap(),
                Some(expected),
                "raw={raw:?}"
            );
        }
    }

    #[test]
    fn parse_rejects_unknown_grade() {
        let err = parse_min_trust_grade("legendary").expect_err("unknown grade must error");
        match err {
            ArgoError::Provider(msg) => {
                assert!(msg.contains("legendary"), "msg={msg:?}");
            }
            other => panic!("expected Provider error, got {other:?}"),
        }
    }
}
