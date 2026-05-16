//! Unit tests for `argo completions <shell>`. We render each shell's
//! script into an in-memory buffer (via the `write_to` test seam) and
//! assert the expected sentinel string — the shell-specific marker
//! every loader expects at the top of the file.

use clap_complete::Shell;

use super::completions::write_to;

fn render(shell: Shell) -> String {
    let mut buf = Vec::new();
    write_to(shell, &mut buf);
    String::from_utf8(buf).expect("clap_complete emits utf-8")
}

#[test]
fn bash_script_contains_argo_function() {
    let out = render(Shell::Bash);
    assert!(!out.is_empty(), "bash completion script was empty");
    assert!(
        out.contains("_argo"),
        "bash completion script missing `_argo` function: {out}",
    );
}

#[test]
fn zsh_script_has_compdef_header() {
    let out = render(Shell::Zsh);
    assert!(
        out.contains("#compdef argo"),
        "zsh completion script missing `#compdef argo`: {out}",
    );
}

#[test]
fn fish_script_uses_complete_for_argo() {
    let out = render(Shell::Fish);
    assert!(
        out.contains("complete -c argo"),
        "fish completion script missing `complete -c argo`: {out}",
    );
}

#[test]
fn powershell_script_is_nonempty() {
    let out = render(Shell::PowerShell);
    assert!(!out.is_empty(), "powershell completion script was empty");
    assert!(
        out.contains("argo"),
        "powershell completion script missing `argo`: {out}",
    );
}

#[test]
fn elvish_script_is_nonempty() {
    let out = render(Shell::Elvish);
    assert!(!out.is_empty(), "elvish completion script was empty");
    assert!(
        out.contains("argo"),
        "elvish completion script missing `argo`: {out}",
    );
}
