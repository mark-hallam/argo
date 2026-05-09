use super::*;

#[test]
fn constructor_and_probe_are_available() {
    let cu = LinuxComputerUse::new();
    let report = cu.probe();
    assert_eq!(report.platform, std::env::consts::OS);
}

#[test]
fn default_is_constructible() {
    let _ = LinuxComputerUse;
}
