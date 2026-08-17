//! Integration tests for shell completion and manpage generation

mod common;
use common::http;

#[test]
fn test_generate_bash_completions() {
    let r = http(&["--generate-completions=bash"]);
    assert_eq!(r.exit_code, 0);
    assert!(
        r.stdout.contains("complete -F")
            || r.stdout.contains("_http")
            || r.stdout.contains("quicpulse")
            || r.stdout.contains("bash")
    );
}

#[test]
fn test_generate_zsh_completions() {
    let r = http(&["--generate-completions=zsh"]);
    assert_eq!(r.exit_code, 0);
    assert!(
        r.stdout.contains("#compdef") || r.stdout.contains("_http") || r.stdout.contains("zsh")
    );
}

#[test]
fn test_generate_fish_completions() {
    let r = http(&["--generate-completions=fish"]);
    assert_eq!(r.exit_code, 0);
    assert!(r.stdout.contains("complete -c") || r.stdout.contains("fish"));
}

#[test]
fn test_generate_powershell_completions() {
    let r = http(&["--generate-completions=powershell"]);
    assert_eq!(r.exit_code, 0);
    assert!(
        r.stdout.contains("Register-ArgumentCompleter")
            || r.stdout.contains("PowerShell")
            || r.stdout.contains("ScriptBlock")
    );
}

#[test]
fn test_generate_elvish_completions() {
    let r = http(&["--generate-completions=elvish"]);
    assert_eq!(r.exit_code, 0);
    assert!(r.stdout.contains("edit:completion:arg-completer") || r.stdout.contains("elvish"));
}

#[test]
fn test_generate_manpage() {
    let r = http(&["--generate-manpage"]);
    assert_eq!(r.exit_code, 0);
    assert!(r.stdout.contains(".TH") || r.stdout.contains(".SH") || r.stdout.contains("NAME"));
}
