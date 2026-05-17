use argo_skill::{
    LayeredSkill, LifecycleState, Provenance, Skill, SkillFrontmatter, SkillLayer,
    SkillRelevanceSelector, SkillSelectionContext,
};
use argo_types::{CapabilitySet, Lock, SkillId};
use chrono::{Duration, Utc};
use uuid::Uuid;

fn skill(
    name: &str,
    source: &str,
    vertical_id: Option<&str>,
    body: &str,
    rating: u16,
) -> LayeredSkill {
    LayeredSkill {
        layer: SkillLayer::Library,
        skill: Skill {
            id: SkillId(Uuid::now_v7()),
            frontmatter: SkillFrontmatter {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                description: format!("rating: {rating}"),
                capabilities: CapabilitySet::default(),
                provenance: Provenance {
                    source: source.to_string(),
                    hash: "sha256-test".to_string(),
                },
                lock: Lock::None,
                evals: Vec::new(),
            },
            body: body.to_string(),
            hash: [0; 32],
            learned_in: None,
            superseded_by: None,
            created_at: Utc::now() + Duration::seconds(i64::from(rating)),
            vertical_id: vertical_id.map(str::to_string),
            lifecycle_state: LifecycleState::Active,
        },
    }
}

fn ctx<'a>(vertical_id: &'a str, tags: &'a [&'a str]) -> SkillSelectionContext<'a> {
    SkillSelectionContext {
        action_type: Some("draft_reply"),
        vertical_id: Some(vertical_id),
        task_tags: tags,
    }
}

#[test]
fn relevance_excludes_cross_vertical_marketplace_skill_without_marker() {
    let rows = vec![
        skill(
            "sales-reply",
            "published:verified:acme",
            Some("sales"),
            "action_types: [draft_reply]\ntags: [sales]",
            90,
        ),
        skill(
            "marketing-reply",
            "published:verified:acme",
            Some("marketing"),
            "action_types: [draft_reply]\ntags: [sales]",
            95,
        ),
    ];
    let selected = SkillRelevanceSelector::default().select(rows, ctx("sales", &["sales"]));
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].frontmatter.name, "sales-reply");
}

#[test]
fn relevance_excludes_untrusted_skill_when_policy_requires_verified() {
    let rows = vec![
        skill(
            "community-reply",
            "published:community:vendor",
            Some("support"),
            "action_types: [draft_reply]\ntags: [support]",
            95,
        ),
        skill(
            "verified-reply",
            "published:verified:vendor",
            Some("support"),
            "action_types: [draft_reply]\ntags: [support]",
            80,
        ),
    ];
    let selected = SkillRelevanceSelector::default().select(rows, ctx("support", &["support"]));
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].frontmatter.name, "verified-reply");
}

#[test]
fn relevance_cap_drops_lowest_rated_sixth_marketplace_skill() {
    let rows: Vec<_> = (1..=6)
        .map(|rating| {
            skill(
                &format!("reply-{rating}"),
                "published:verified:vendor",
                Some("support"),
                "action_types: [draft_reply]\ntags: [support]",
                rating,
            )
        })
        .collect();
    let selected = SkillRelevanceSelector::default().select(rows, ctx("support", &["support"]));
    assert_eq!(selected.len(), 5);
    assert!(selected.iter().all(|s| s.frontmatter.name != "reply-1"));
}
