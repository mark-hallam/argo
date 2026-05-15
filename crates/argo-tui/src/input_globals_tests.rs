use super::*;

#[test]
fn register_global_key_adds_lookup_entry() {
    let mut registry = Vec::new();
    register_global_key(
        &mut registry,
        KeyCode::Char('?'),
        GlobalKeyAction::KeybindingsHelpOverlay,
    )
    .unwrap();

    assert_eq!(
        lookup_global_key(&registry, KeyCode::Char('?')),
        Some(GlobalKeyAction::KeybindingsHelpOverlay)
    );
}

#[test]
fn register_global_key_rejects_duplicate_key() {
    let mut registry = Vec::new();
    register_global_key(
        &mut registry,
        KeyCode::Char('?'),
        GlobalKeyAction::KeybindingsHelpOverlay,
    )
    .unwrap();

    let err = register_global_key(
        &mut registry,
        KeyCode::Char('?'),
        GlobalKeyAction::DashboardOverlay,
    )
    .unwrap_err();

    assert_eq!(err.key, KeyCode::Char('?'));
    assert_eq!(
        lookup_global_key(&registry, KeyCode::Char('?')),
        Some(GlobalKeyAction::KeybindingsHelpOverlay)
    );
}

#[test]
fn default_registry_preserves_existing_global_openers() {
    let registry = default_global_keys();

    assert_eq!(
        lookup_global_key(&registry, KeyCode::Char('?')),
        Some(GlobalKeyAction::KeybindingsHelpOverlay)
    );
    assert_eq!(
        lookup_global_key(&registry, KeyCode::Char('$')),
        Some(GlobalKeyAction::StoreBackedOverlay)
    );
    assert_eq!(
        lookup_global_key(&registry, KeyCode::Char('V')),
        Some(GlobalKeyAction::ReplayOverlay)
    );
    assert_eq!(lookup_global_key(&registry, KeyCode::Char('x')), None);
}
