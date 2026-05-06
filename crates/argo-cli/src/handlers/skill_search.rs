//! `argo skill search` marketplace search handler.
use argo_skill::marketplace::{parse_trust_grade, search, MarketplaceSkill, SearchOptions};
use argo_skill::TrustGrade;
use argo_types::{ArgoError, ArgoResult};

pub fn run(
    query: &str,
    trust_min: Option<&str>,
    tenant: Option<&str>,
    limit: usize,
    json: bool,
) -> ArgoResult<()> {
    let trust_min = trust_min
        .map(parse_trust_grade)
        .transpose()
        .map_err(ArgoError::Provider)?;
    let options = SearchOptions {
        trust_min,
        tenant: tenant.map(str::to_string),
        limit,
    };
    let rows = search(query, &options);
    if json {
        let serialized = serde_json::to_string(&rows)
            .map_err(|e| ArgoError::Provider(format!("serialize skill search: {e}")))?;
        println!("{serialized}");
        return Ok(());
    }
    render_table(&rows);
    Ok(())
}

fn render_table(rows: &[MarketplaceSkill]) {
    if rows.is_empty() {
        println!("(no marketplace skills matched)");
        return;
    }
    println!(
        "{:<30} {:<12} {:<20} {:<10} {:<20} description",
        "skill", "tenant", "trust", "version", "last_published"
    );
    for row in rows {
        println!(
            "{:<30} {:<12} {:<20} {:<10} {:<20} {}",
            row.name,
            row.tenant,
            trust_label(row.trust_grade),
            row.version,
            row.last_published_at,
            row.description
        );
    }
}

fn trust_label(grade: TrustGrade) -> &'static str {
    match grade {
        TrustGrade::Builtin => "builtin",
        TrustGrade::PublishedVerified => "published_verified",
        TrustGrade::Tenant => "tenant",
        TrustGrade::PublishedCommunity => "published_community",
        TrustGrade::LearnedUnreviewed => "learned_unreviewed",
    }
}
