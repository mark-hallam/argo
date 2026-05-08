use super::*;

#[test]
fn load_all_returns_exactly_five_templates() {
    let templates = load_all().expect("templates load");

    assert_eq!(templates.len(), 5);
    assert!(templates.iter().all(|t| !t.prompt_skeleton.is_empty()));
}

#[test]
fn sales_sdr_for_b2b_saas_matches_sales_template() {
    let template = match_template("sales SDR for B2B SaaS").expect("template");

    assert_eq!(template.meta.id, "sales_sdr");
    assert_eq!(template.archetype(), Some(Archetype::SalesSdr));
}

#[test]
fn ar_clerk_reconciling_invoices_matches_finance_ar() {
    let template = match_template("AR clerk reconciling invoices").expect("template");

    assert_eq!(template.meta.id, "finance_ar");
    assert_eq!(template.archetype(), Some(Archetype::BookkeepingAssistant));
}

#[test]
fn unknown_role_returns_none() {
    assert!(match_template("poetry coach for a theater workshop").is_none());
}

#[test]
fn merge_with_overrides_preserves_prompt_skeleton_without_hint() {
    let template = match_template("sales SDR for SaaS").expect("template");
    let merged = merge_with_overrides(&template, "sales SDR for SaaS").expect("merge");

    assert_eq!(merged.prompt_skeleton, template.prompt_skeleton);
    assert!(!merged.actions.is_empty());
    assert!(!merged.memory_layers.is_empty());
    assert!(!merged.eval_seeds.is_empty());
}
