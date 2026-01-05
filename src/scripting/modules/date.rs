//! Date/Time module for Rune scripts
//!
//! Provides date parsing, formatting, and manipulation utilities.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use chrono::{DateTime, Duration, Local, NaiveDateTime, TimeZone, Utc};

/// Create the date module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("date")?;

    // Current time
    module.function("now", now_iso).build()?;
    module.function("now_utc", now_utc).build()?;
    module.function("now_local", now_local).build()?;
    module.function("timestamp", timestamp_secs).build()?;
    module.function("timestamp_ms", timestamp_ms).build()?;

    // Parsing
    module.function("parse", parse_date).build()?;
    module.function("parse_iso", parse_iso).build()?;
    module.function("parse_rfc2822", parse_rfc2822).build()?;
    module.function("from_timestamp", from_timestamp).build()?;
    module.function("from_timestamp_ms", from_timestamp_ms).build()?;

    // Formatting
    module.function("format", format_date).build()?;
    module.function("to_iso", to_iso).build()?;
    module.function("to_rfc2822", to_rfc2822).build()?;
    module.function("to_timestamp", to_timestamp).build()?;

    // Components
    module.function("year", get_year).build()?;
    module.function("month", get_month).build()?;
    module.function("day", get_day).build()?;
    module.function("hour", get_hour).build()?;
    module.function("minute", get_minute).build()?;
    module.function("second", get_second).build()?;
    module.function("weekday", get_weekday).build()?;
    module.function("day_of_year", get_day_of_year).build()?;

    // Arithmetic
    module.function("add_days", add_days).build()?;
    module.function("add_hours", add_hours).build()?;
    module.function("add_minutes", add_minutes).build()?;
    module.function("add_seconds", add_seconds).build()?;
    module.function("subtract_days", subtract_days).build()?;

    // Comparison
    module.function("diff_days", diff_days).build()?;
    module.function("diff_hours", diff_hours).build()?;
    module.function("diff_seconds", diff_seconds).build()?;
    module.function("is_before", is_before).build()?;
    module.function("is_after", is_after).build()?;

    // Utilities
    module.function("start_of_day", start_of_day).build()?;
    module.function("end_of_day", end_of_day).build()?;
    module.function("is_valid", is_valid_date).build()?;

    Ok(module)
}

/// Get current time as ISO 8601 string
fn now_iso() -> RuneString {
    let now = Utc::now();
    RuneString::try_from(now.to_rfc3339()).unwrap_or_default()
}

/// Get current UTC time as ISO 8601 string
fn now_utc() -> RuneString {
    let now = Utc::now();
    RuneString::try_from(now.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_default()
}

/// Get current local time as ISO 8601 string
fn now_local() -> RuneString {
    let now = Local::now();
    RuneString::try_from(now.to_rfc3339()).unwrap_or_default()
}

/// Get current timestamp in seconds
fn timestamp_secs() -> i64 {
    Utc::now().timestamp()
}

/// Get current timestamp in milliseconds
fn timestamp_ms() -> i64 {
    Utc::now().timestamp_millis()
}

/// Parse a date string with custom format
fn parse_date(date_str: &str, format: &str) -> RuneString {
    match NaiveDateTime::parse_from_str(date_str, format) {
        Ok(dt) => {
            let utc: DateTime<Utc> = DateTime::from_naive_utc_and_offset(dt, Utc);
            RuneString::try_from(utc.to_rfc3339()).unwrap_or_default()
        }
        Err(_) => RuneString::new(),
    }
}

/// Parse an ISO 8601 date string
fn parse_iso(date_str: &str) -> RuneString {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => RuneString::try_from(dt.to_rfc3339()).unwrap_or_default(),
        Err(_) => {
            // Try parsing without timezone
            match NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S") {
                Ok(dt) => {
                    let utc: DateTime<Utc> = DateTime::from_naive_utc_and_offset(dt, Utc);
                    RuneString::try_from(utc.to_rfc3339()).unwrap_or_default()
                }
                Err(_) => RuneString::new(),
            }
        }
    }
}

/// Parse an RFC 2822 date string
fn parse_rfc2822(date_str: &str) -> RuneString {
    match DateTime::parse_from_rfc2822(date_str) {
        Ok(dt) => RuneString::try_from(dt.to_rfc3339()).unwrap_or_default(),
        Err(_) => RuneString::new(),
    }
}

/// Create date from Unix timestamp (seconds)
fn from_timestamp(ts: i64) -> RuneString {
    match Utc.timestamp_opt(ts, 0).single() {
        Some(dt) => RuneString::try_from(dt.to_rfc3339()).unwrap_or_default(),
        None => RuneString::new(),
    }
}

/// Create date from Unix timestamp (milliseconds)
fn from_timestamp_ms(ts: i64) -> RuneString {
    match Utc.timestamp_millis_opt(ts).single() {
        Some(dt) => RuneString::try_from(dt.to_rfc3339()).unwrap_or_default(),
        None => RuneString::new(),
    }
}

/// Format a date string with custom format
fn format_date(date_str: &str, format: &str) -> RuneString {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => RuneString::try_from(dt.format(format).to_string()).unwrap_or_default(),
        Err(_) => RuneString::new(),
    }
}

/// Convert to ISO 8601 format
fn to_iso(date_str: &str) -> RuneString {
    parse_iso(date_str)
}

/// Convert to RFC 2822 format
fn to_rfc2822(date_str: &str) -> RuneString {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => RuneString::try_from(dt.to_rfc2822()).unwrap_or_default(),
        Err(_) => RuneString::new(),
    }
}

/// Convert to Unix timestamp (seconds)
fn to_timestamp(date_str: &str) -> i64 {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => dt.timestamp(),
        Err(_) => 0,
    }
}

/// Get year component
fn get_year(date_str: &str) -> i64 {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => dt.year() as i64,
        Err(_) => 0,
    }
}

/// Get month component (1-12)
fn get_month(date_str: &str) -> i64 {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => dt.month() as i64,
        Err(_) => 0,
    }
}

/// Get day component (1-31)
fn get_day(date_str: &str) -> i64 {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => dt.day() as i64,
        Err(_) => 0,
    }
}

/// Get hour component (0-23)
fn get_hour(date_str: &str) -> i64 {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => dt.hour() as i64,
        Err(_) => 0,
    }
}

/// Get minute component (0-59)
fn get_minute(date_str: &str) -> i64 {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => dt.minute() as i64,
        Err(_) => 0,
    }
}

/// Get second component (0-59)
fn get_second(date_str: &str) -> i64 {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => dt.second() as i64,
        Err(_) => 0,
    }
}

/// Get weekday (0=Sunday, 6=Saturday)
fn get_weekday(date_str: &str) -> i64 {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => dt.weekday().num_days_from_sunday() as i64,
        Err(_) => 0,
    }
}

/// Get day of year (1-366)
fn get_day_of_year(date_str: &str) -> i64 {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => dt.ordinal() as i64,
        Err(_) => 0,
    }
}

use chrono::Datelike;
use chrono::Timelike;

/// Add days to a date
fn add_days(date_str: &str, days: i64) -> RuneString {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => {
            let new_dt = dt + Duration::days(days);
            RuneString::try_from(new_dt.to_rfc3339()).unwrap_or_default()
        }
        Err(_) => RuneString::try_from(date_str.to_string()).unwrap_or_default(),
    }
}

/// Add hours to a date
fn add_hours(date_str: &str, hours: i64) -> RuneString {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => {
            let new_dt = dt + Duration::hours(hours);
            RuneString::try_from(new_dt.to_rfc3339()).unwrap_or_default()
        }
        Err(_) => RuneString::try_from(date_str.to_string()).unwrap_or_default(),
    }
}

/// Add minutes to a date
fn add_minutes(date_str: &str, minutes: i64) -> RuneString {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => {
            let new_dt = dt + Duration::minutes(minutes);
            RuneString::try_from(new_dt.to_rfc3339()).unwrap_or_default()
        }
        Err(_) => RuneString::try_from(date_str.to_string()).unwrap_or_default(),
    }
}

/// Add seconds to a date
fn add_seconds(date_str: &str, seconds: i64) -> RuneString {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => {
            let new_dt = dt + Duration::seconds(seconds);
            RuneString::try_from(new_dt.to_rfc3339()).unwrap_or_default()
        }
        Err(_) => RuneString::try_from(date_str.to_string()).unwrap_or_default(),
    }
}

/// Subtract days from a date
fn subtract_days(date_str: &str, days: i64) -> RuneString {
    add_days(date_str, -days)
}

/// Get difference in days between two dates
fn diff_days(date1: &str, date2: &str) -> i64 {
    match (DateTime::parse_from_rfc3339(date1), DateTime::parse_from_rfc3339(date2)) {
        (Ok(dt1), Ok(dt2)) => (dt2 - dt1).num_days(),
        _ => 0,
    }
}

/// Get difference in hours between two dates
fn diff_hours(date1: &str, date2: &str) -> i64 {
    match (DateTime::parse_from_rfc3339(date1), DateTime::parse_from_rfc3339(date2)) {
        (Ok(dt1), Ok(dt2)) => (dt2 - dt1).num_hours(),
        _ => 0,
    }
}

/// Get difference in seconds between two dates
fn diff_seconds(date1: &str, date2: &str) -> i64 {
    match (DateTime::parse_from_rfc3339(date1), DateTime::parse_from_rfc3339(date2)) {
        (Ok(dt1), Ok(dt2)) => (dt2 - dt1).num_seconds(),
        _ => 0,
    }
}

/// Check if date1 is before date2
fn is_before(date1: &str, date2: &str) -> bool {
    match (DateTime::parse_from_rfc3339(date1), DateTime::parse_from_rfc3339(date2)) {
        (Ok(dt1), Ok(dt2)) => dt1 < dt2,
        _ => false,
    }
}

/// Check if date1 is after date2
fn is_after(date1: &str, date2: &str) -> bool {
    match (DateTime::parse_from_rfc3339(date1), DateTime::parse_from_rfc3339(date2)) {
        (Ok(dt1), Ok(dt2)) => dt1 > dt2,
        _ => false,
    }
}

/// Get start of day (00:00:00)
fn start_of_day(date_str: &str) -> RuneString {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => {
            let start = dt.with_hour(0).and_then(|d| d.with_minute(0)).and_then(|d| d.with_second(0));
            match start {
                Some(s) => RuneString::try_from(s.to_rfc3339()).unwrap_or_default(),
                None => RuneString::try_from(date_str.to_string()).unwrap_or_default(),
            }
        }
        Err(_) => RuneString::try_from(date_str.to_string()).unwrap_or_default(),
    }
}

/// Get end of day (23:59:59)
fn end_of_day(date_str: &str) -> RuneString {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => {
            let end = dt.with_hour(23).and_then(|d| d.with_minute(59)).and_then(|d| d.with_second(59));
            match end {
                Some(e) => RuneString::try_from(e.to_rfc3339()).unwrap_or_default(),
                None => RuneString::try_from(date_str.to_string()).unwrap_or_default(),
            }
        }
        Err(_) => RuneString::try_from(date_str.to_string()).unwrap_or_default(),
    }
}

/// Check if a date string is valid
fn is_valid_date(date_str: &str) -> bool {
    DateTime::parse_from_rfc3339(date_str).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now() {
        let now = now_iso();
        assert!(!now.is_empty());
        assert!(now.contains("T"));
    }

    #[test]
    fn test_from_timestamp() {
        let date = from_timestamp(0);
        assert!(date.contains("1970"));
    }

    #[test]
    fn test_add_days() {
        let date = "2024-01-15T12:00:00+00:00";
        let result = add_days(date, 5);
        assert!(result.contains("2024-01-20"));
    }

    #[test]
    fn test_diff_days() {
        let date1 = "2024-01-01T00:00:00+00:00";
        let date2 = "2024-01-10T00:00:00+00:00";
        assert_eq!(diff_days(date1, date2), 9);
    }

    #[test]
    fn test_components() {
        let date = "2024-06-15T14:30:45+00:00";
        assert_eq!(get_year(date), 2024);
        assert_eq!(get_month(date), 6);
        assert_eq!(get_day(date), 15);
        assert_eq!(get_hour(date), 14);
        assert_eq!(get_minute(date), 30);
        assert_eq!(get_second(date), 45);
    }
}
