//! Unit tests for Rune scripting engine, module loading, and script type detection

use quicpulse::scripting::context::ScriptContext;
use quicpulse::scripting::detection::ScriptType;
use quicpulse::scripting::runtime::ScriptEngine;

#[test]
fn test_script_type_detection() {
    assert_eq!(ScriptType::from_str("javascript"), ScriptType::JavaScript);
    assert_eq!(ScriptType::from_str("js"), ScriptType::JavaScript);
    assert_eq!(ScriptType::from_str("ecmascript"), ScriptType::JavaScript);
    assert_eq!(ScriptType::from_str("rune"), ScriptType::Rune);
    assert_eq!(ScriptType::from_str("rn"), ScriptType::Rune);
    assert_eq!(ScriptType::from_str("unknown"), ScriptType::Rune);

    assert_eq!(
        ScriptType::from_extension("script.js"),
        ScriptType::JavaScript
    );
    assert_eq!(
        ScriptType::from_extension("module.mjs"),
        ScriptType::JavaScript
    );
    assert_eq!(
        ScriptType::from_extension("common.cjs"),
        ScriptType::JavaScript
    );
    assert_eq!(ScriptType::from_extension("test.rn"), ScriptType::Rune);
    assert_eq!(ScriptType::from_extension("test.rune"), ScriptType::Rune);
    assert_eq!(ScriptType::from_extension("test.txt"), ScriptType::Rune);

    assert_eq!(ScriptType::Rune.name(), "Rune");
    assert_eq!(ScriptType::JavaScript.name(), "JavaScript");
}

#[tokio::test]
async fn test_rune_script_engine_basic_execution() {
    let engine = ScriptEngine::new().unwrap();
    let mut ctx = ScriptContext::new();

    let script = r#"
    pub fn main() {
        let x = 10;
        let y = 20;
        x + y == 30
    }
    "#;

    let res = engine.execute(script, &mut ctx).await.unwrap();
    assert!(res.as_bool().unwrap());
}

#[tokio::test]
async fn test_rune_script_crypto_modules() {
    let engine = ScriptEngine::new().unwrap();
    let mut ctx = ScriptContext::new();

    let script = r#"
    pub fn main() {
        let sha = crypto::sha256_hex("hello");
        let md5_val = crypto::md5_hex("world");
        let b64 = encoding::base64_encode("secret");
        !sha.is_empty() && !md5_val.is_empty() && !b64.is_empty()
    }
    "#;

    let res = engine.execute(script, &mut ctx).await.unwrap();
    assert!(res.as_bool().unwrap());
}
