use super::*;

#[test]
fn constructor_and_probe_are_available() {
    let cu = WindowsComputerUse::new();
    let report = cu.probe();
    assert_eq!(report.platform, std::env::consts::OS);
}

#[test]
fn default_is_constructible() {
    let _ = WindowsComputerUse;
}

#[test]
fn off_windows_methods_return_unsupported() {
    if cfg!(target_os = "windows") {
        return;
    }
    let cu = WindowsComputerUse::new();
    assert!(matches!(
        cu.screenshot(std::path::Path::new("C:/x.png")),
        Err(WindowsCuError::Unsupported { .. })
    ));
    assert!(matches!(
        cu.click(0, 0),
        Err(WindowsCuError::Unsupported { .. })
    ));
    assert!(matches!(
        cu.key("enter"),
        Err(WindowsCuError::Unsupported { .. })
    ));
    assert!(matches!(
        cu.type_text("hi"),
        Err(WindowsCuError::Unsupported { .. })
    ));
    assert!(matches!(
        cu.windows("Notepad"),
        Err(WindowsCuError::Unsupported { .. })
    ));
    assert!(matches!(cu.focus(1), Err(WindowsCuError::Unsupported { .. })));
}
