use quicpulse::cli::Args;
use quicpulse::devexp::codegen::{generate_code, Language};

#[test]
fn test_language_from_str() {
    assert_eq!(Language::from_str("python"), Some(Language::Python));
    assert_eq!(Language::from_str("py"), Some(Language::Python));
    assert_eq!(Language::from_str("node"), Some(Language::Node));
    assert_eq!(Language::from_str("nodejs"), Some(Language::Node));
    assert_eq!(Language::from_str("js"), Some(Language::Node));
    assert_eq!(Language::from_str("javascript"), Some(Language::Node));
    assert_eq!(Language::from_str("go"), Some(Language::Go));
    assert_eq!(Language::from_str("golang"), Some(Language::Go));
    assert_eq!(Language::from_str("java"), Some(Language::Java));
    assert_eq!(Language::from_str("php"), Some(Language::Php));
    assert_eq!(Language::from_str("rust"), Some(Language::Rust));
    assert_eq!(Language::from_str("rs"), Some(Language::Rust));
    assert_eq!(Language::from_str("ruby"), Some(Language::Ruby));
    assert_eq!(Language::from_str("rb"), Some(Language::Ruby));
    assert_eq!(Language::from_str("csharp"), Some(Language::Csharp));
    assert_eq!(Language::from_str("c#"), Some(Language::Csharp));
    assert_eq!(Language::from_str("cs"), Some(Language::Csharp));
    assert_eq!(Language::from_str("unknown_lang"), None);
}

#[test]
fn test_generate_code_all_supported_languages() {
    let args = Args {
        method: Some("POST".to_string()),
        url: Some("https://api.example.com/v1/users".to_string()),
        json: true,
        ..Default::default()
    };

    let processed = quicpulse::cli::process_args(&args).unwrap();

    let langs = [
        "python", "node", "go", "java", "php", "rust", "ruby", "csharp",
    ];
    for lang in langs {
        let code = generate_code(lang, &args, &processed);
        assert!(code.is_ok(), "Code generation for '{}' failed", lang);
        let snippet = code.unwrap();
        assert!(!snippet.is_empty());
        assert!(snippet.contains("https://api.example.com/v1/users"));
    }
}
