//! Assertion module for Rune scripts
//!
//! Provides assertion functions for testing HTTP responses
//! and validating data in scripts.

use rune::{ContextError, Module};

/// Create the assert module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("assert")?;

    // Basic assertions
    module.function("eq", assert_eq_fn).build()?;
    module.function("ne", assert_ne_fn).build()?;
    module.function("is_true", assert_true).build()?;
    module.function("is_false", assert_false).build()?;

    // Comparison assertions
    module.function("gt", assert_gt).build()?;
    module.function("gte", assert_gte).build()?;
    module.function("lt", assert_lt).build()?;
    module.function("lte", assert_lte).build()?;

    // HTTP-specific assertions
    module.function("status_success", assert_status_success).build()?;
    module.function("status_redirect", assert_status_redirect).build()?;
    module.function("status_client_error", assert_status_client_error).build()?;
    module.function("status_server_error", assert_status_server_error).build()?;

    // Soft assertions (return bool instead of panicking)
    module.function("check_eq", check_eq).build()?;
    module.function("check_ne", check_ne).build()?;
    module.function("check_gt", check_gt).build()?;
    module.function("check_gte", check_gte).build()?;
    module.function("check_lt", check_lt).build()?;
    module.function("check_lte", check_lte).build()?;

    Ok(module)
}

/// Assert two values are equal
fn assert_eq_fn(a: i64, b: i64) -> bool {
    if a != b {
        panic!("assertion failed: {} != {}", a, b);
    }
    true
}

/// Assert two values are not equal
fn assert_ne_fn(a: i64, b: i64) -> bool {
    if a == b {
        panic!("assertion failed: {} == {}", a, b);
    }
    true
}

/// Assert value is true
fn assert_true(value: bool) -> bool {
    if !value {
        panic!("assertion failed: expected true, got false");
    }
    true
}

/// Assert value is false
fn assert_false(value: bool) -> bool {
    if value {
        panic!("assertion failed: expected false, got true");
    }
    true
}

/// Assert a > b
fn assert_gt(a: i64, b: i64) -> bool {
    if a <= b {
        panic!("assertion failed: {} is not greater than {}", a, b);
    }
    true
}

/// Assert a >= b
fn assert_gte(a: i64, b: i64) -> bool {
    if a < b {
        panic!("assertion failed: {} is not >= {}", a, b);
    }
    true
}

/// Assert a < b
fn assert_lt(a: i64, b: i64) -> bool {
    if a >= b {
        panic!("assertion failed: {} is not less than {}", a, b);
    }
    true
}

/// Assert a <= b
fn assert_lte(a: i64, b: i64) -> bool {
    if a > b {
        panic!("assertion failed: {} is not <= {}", a, b);
    }
    true
}

/// Assert HTTP status is success (2xx)
fn assert_status_success(status: i64) -> bool {
    if status < 200 || status >= 300 {
        panic!("assertion failed: status {} is not a success status (2xx)", status);
    }
    true
}

/// Assert HTTP status is redirect (3xx)
fn assert_status_redirect(status: i64) -> bool {
    if status < 300 || status >= 400 {
        panic!("assertion failed: status {} is not a redirect status (3xx)", status);
    }
    true
}

/// Assert HTTP status is client error (4xx)
fn assert_status_client_error(status: i64) -> bool {
    if status < 400 || status >= 500 {
        panic!("assertion failed: status {} is not a client error status (4xx)", status);
    }
    true
}

/// Assert HTTP status is server error (5xx)
fn assert_status_server_error(status: i64) -> bool {
    if status < 500 || status >= 600 {
        panic!("assertion failed: status {} is not a server error status (5xx)", status);
    }
    true
}

/// Check equality (returns bool instead of panicking)
fn check_eq(a: i64, b: i64) -> bool {
    a == b
}

/// Check inequality (returns bool instead of panicking)
fn check_ne(a: i64, b: i64) -> bool {
    a != b
}

/// Check greater than
fn check_gt(a: i64, b: i64) -> bool {
    a > b
}

/// Check greater than or equal
fn check_gte(a: i64, b: i64) -> bool {
    a >= b
}

/// Check less than
fn check_lt(a: i64, b: i64) -> bool {
    a < b
}

/// Check less than or equal
fn check_lte(a: i64, b: i64) -> bool {
    a <= b
}
