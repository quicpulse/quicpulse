//! Unit tests for QuickJS JavaScript runtime and all JavaScript modules

#[cfg(feature = "javascript")]
use quicpulse::scripting::context::ScriptContext;
#[cfg(feature = "javascript")]
use quicpulse::scripting::js::JsScriptEngine;

#[cfg(feature = "javascript")]
#[tokio::test]
async fn test_js_engine_crypto_module() {
    let engine = JsScriptEngine::new().unwrap();
    let mut ctx = ScriptContext::new();

    let res = engine
        .execute(
            r#"crypto.sha256_hex("test") === "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";"#,
            &mut ctx,
        )
        .await
        .unwrap();
    assert!(res.as_bool().unwrap());

    let res = engine
        .execute(
            r#"crypto.md5_hex("test") === "098f6bcd4621d373cade4e832627b4f6";"#,
            &mut ctx,
        )
        .await
        .unwrap();
    assert!(res.as_bool().unwrap());
}

#[cfg(feature = "javascript")]
#[tokio::test]
async fn test_js_encoding_module() {
    let engine = JsScriptEngine::new().unwrap();
    let mut ctx = ScriptContext::new();

    let res = engine
        .execute(
            r#"encoding.base64_encode("hello world") === "aGVsbG8gd29ybGQ=";"#,
            &mut ctx,
        )
        .await
        .unwrap();
    assert!(res.as_bool().unwrap());

    let res = engine
        .execute(r#"encoding.hex_encode("abc") === "616263";"#, &mut ctx)
        .await
        .unwrap();
    assert!(res.as_bool().unwrap());
}

#[cfg(feature = "javascript")]
#[tokio::test]
async fn test_js_faker_module() {
    let engine = JsScriptEngine::new().unwrap();
    let mut ctx = ScriptContext::new();

    let res = engine
        .execute(
            r#"typeof faker.name() === "string" && faker.email().includes("@");"#,
            &mut ctx,
        )
        .await
        .unwrap();
    assert!(res.as_bool().unwrap());
}
