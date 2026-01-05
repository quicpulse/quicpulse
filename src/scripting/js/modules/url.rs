//! URL module for JavaScript
//!
//! Provides URL parsing and manipulation functions.

use rquickjs::{Ctx, Object, Function};
use url::Url;
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let url_obj = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create url object: {}", e)))?;

    url_obj.set("parse", Function::new(ctx.clone(), url_parse)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    url_obj.set("host", Function::new(ctx.clone(), url_host)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    url_obj.set("path", Function::new(ctx.clone(), url_path)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    url_obj.set("query", Function::new(ctx.clone(), url_query)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    url_obj.set("scheme", Function::new(ctx.clone(), url_scheme)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    url_obj.set("port", Function::new(ctx.clone(), url_port)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    url_obj.set("join", Function::new(ctx.clone(), url_join)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    url_obj.set("is_valid", Function::new(ctx.clone(), url_is_valid)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("url", url_obj)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set url global: {}", e)))?;

    Ok(())
}

fn url_parse(input: String) -> Option<String> {
    Url::parse(&input).ok().map(|u| {
        serde_json::json!({
            "scheme": u.scheme(),
            "host": u.host_str(),
            "port": u.port(),
            "path": u.path(),
            "query": u.query(),
            "fragment": u.fragment(),
        }).to_string()
    })
}

fn url_host(input: String) -> Option<String> {
    Url::parse(&input).ok().and_then(|u| u.host_str().map(String::from))
}

fn url_path(input: String) -> Option<String> {
    Url::parse(&input).ok().map(|u| u.path().to_string())
}

fn url_query(input: String) -> Option<String> {
    Url::parse(&input).ok().and_then(|u| u.query().map(String::from))
}

fn url_scheme(input: String) -> Option<String> {
    Url::parse(&input).ok().map(|u| u.scheme().to_string())
}

fn url_port(input: String) -> Option<i32> {
    Url::parse(&input).ok().and_then(|u| u.port().map(|p| p as i32))
}

fn url_join(base: String, path: String) -> Option<String> {
    Url::parse(&base).ok().and_then(|u| u.join(&path).ok().map(|u| u.to_string()))
}

fn url_is_valid(input: String) -> bool {
    Url::parse(&input).is_ok()
}
