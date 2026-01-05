//! Assert module for JavaScript
//!
//! Provides assertion functions for testing.

use rquickjs::{Ctx, Object, Function};
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let assert_obj = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create assert object: {}", e)))?;

    assert_obj.set("eq", Function::new(ctx.clone(), assert_eq_fn)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("ne", Function::new(ctx.clone(), assert_ne_fn)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("is_true", Function::new(ctx.clone(), assert_true)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("is_false", Function::new(ctx.clone(), assert_false)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("gt", Function::new(ctx.clone(), assert_gt)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("gte", Function::new(ctx.clone(), assert_gte)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("lt", Function::new(ctx.clone(), assert_lt)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("lte", Function::new(ctx.clone(), assert_lte)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("ok", Function::new(ctx.clone(), assert_ok)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("fail", Function::new(ctx.clone(), assert_fail)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    // Status assertions
    assert_obj.set("status_ok", Function::new(ctx.clone(), assert_status_ok)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("status_success", Function::new(ctx.clone(), assert_status_success)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("status_redirect", Function::new(ctx.clone(), assert_status_redirect)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("status_client_error", Function::new(ctx.clone(), assert_status_client_error)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    assert_obj.set("status_server_error", Function::new(ctx.clone(), assert_status_server_error)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("assert", assert_obj)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set assert global: {}", e)))?;

    Ok(())
}

fn assert_eq_fn(a: rquickjs::Value<'_>, b: rquickjs::Value<'_>) -> bool {
    // Convert both values to their string representation for comparison
    let a_str = value_to_string(&a);
    let b_str = value_to_string(&b);

    if a_str != b_str {
        eprintln!("[ASSERT] Expected '{}' to equal '{}'", a_str, b_str);
        false
    } else {
        true
    }
}

fn value_to_string(value: &rquickjs::Value<'_>) -> String {
    if let Some(s) = value.as_string() {
        s.to_string().unwrap_or_default()
    } else if let Some(n) = value.as_int() {
        n.to_string()
    } else if let Some(n) = value.as_float() {
        n.to_string()
    } else if let Some(b) = value.as_bool() {
        b.to_string()
    } else if value.is_null() {
        "null".to_string()
    } else if value.is_undefined() {
        "undefined".to_string()
    } else {
        format!("{:?}", value)
    }
}

fn assert_ne_fn(a: String, b: String) -> bool {
    if a == b {
        eprintln!("[ASSERT] Expected '{}' to not equal '{}'", a, b);
        false
    } else {
        true
    }
}

fn assert_true(value: bool) -> bool {
    if !value {
        eprintln!("[ASSERT] Expected true but got false");
        false
    } else {
        true
    }
}

fn assert_false(value: bool) -> bool {
    if value {
        eprintln!("[ASSERT] Expected false but got true");
        false
    } else {
        true
    }
}

fn assert_gt(a: f64, b: f64) -> bool {
    if a <= b {
        eprintln!("[ASSERT] Expected {} > {}", a, b);
        false
    } else {
        true
    }
}

fn assert_gte(a: f64, b: f64) -> bool {
    if a < b {
        eprintln!("[ASSERT] Expected {} >= {}", a, b);
        false
    } else {
        true
    }
}

fn assert_lt(a: f64, b: f64) -> bool {
    if a >= b {
        eprintln!("[ASSERT] Expected {} < {}", a, b);
        false
    } else {
        true
    }
}

fn assert_lte(a: f64, b: f64) -> bool {
    if a > b {
        eprintln!("[ASSERT] Expected {} <= {}", a, b);
        false
    } else {
        true
    }
}

fn assert_ok(value: bool) -> bool {
    if !value {
        eprintln!("[ASSERT] Assertion failed");
        false
    } else {
        true
    }
}

fn assert_fail(message: String) -> bool {
    eprintln!("[ASSERT FAIL] {}", message);
    false
}

fn assert_status_ok(status: i32) -> bool {
    if status != 200 {
        eprintln!("[ASSERT] Expected status 200, got {}", status);
        false
    } else {
        true
    }
}

fn assert_status_success(status: i32) -> bool {
    if !(200..300).contains(&status) {
        eprintln!("[ASSERT] Expected success status (2xx), got {}", status);
        false
    } else {
        true
    }
}

fn assert_status_redirect(status: i32) -> bool {
    if !(300..400).contains(&status) {
        eprintln!("[ASSERT] Expected redirect status (3xx), got {}", status);
        false
    } else {
        true
    }
}

fn assert_status_client_error(status: i32) -> bool {
    if !(400..500).contains(&status) {
        eprintln!("[ASSERT] Expected client error status (4xx), got {}", status);
        false
    } else {
        true
    }
}

fn assert_status_server_error(status: i32) -> bool {
    if !(500..600).contains(&status) {
        eprintln!("[ASSERT] Expected server error status (5xx), got {}", status);
        false
    } else {
        true
    }
}
