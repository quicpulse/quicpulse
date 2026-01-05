//! Regex module for Rune scripts
//!
//! Provides regular expression operations for pattern matching and text manipulation.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use regex::Regex;
use std::cell::RefCell;
use std::collections::HashMap;

/// Thread-local cache for compiled regexes to avoid recompilation in loops
/// Uses LRU-like eviction when cache is full
thread_local! {
    static REGEX_CACHE: RefCell<RegexCache> = RefCell::new(RegexCache::new(128));
}

struct RegexCache {
    cache: HashMap<String, Regex>,
    order: Vec<String>,
    capacity: usize,
}

impl RegexCache {
    fn new(capacity: usize) -> Self {
        Self {
            cache: HashMap::new(),
            order: Vec::new(),
            capacity,
        }
    }

    fn get_or_compile(&mut self, pattern: &str) -> Option<&Regex> {
        if !self.cache.contains_key(pattern) {
            // Compile the regex
            match Regex::new(pattern) {
                Ok(re) => {
                    // Evict oldest if at capacity
                    if self.cache.len() >= self.capacity {
                        if let Some(oldest) = self.order.first().cloned() {
                            self.cache.remove(&oldest);
                            self.order.remove(0);
                        }
                    }
                    self.cache.insert(pattern.to_string(), re);
                    self.order.push(pattern.to_string());
                }
                Err(_) => return None,
            }
        } else {
            // Move to end of order (most recently used)
            if let Some(pos) = self.order.iter().position(|p| p == pattern) {
                let key = self.order.remove(pos);
                self.order.push(key);
            }
        }
        self.cache.get(pattern)
    }
}

/// Get a cached regex or compile and cache it
fn with_cached_regex<F, T>(pattern: &str, default: T, f: F) -> T
where
    F: FnOnce(&Regex) -> T,
{
    REGEX_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        match cache.get_or_compile(pattern) {
            Some(re) => f(re),
            None => default,
        }
    })
}

/// Create the regex module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("regex")?;

    // Matching
    module.function("test", test_match).build()?;
    module.function("find", find_match).build()?;
    module.function("match_all", find_all_matches).build()?;

    // Capturing
    module.function("capture", capture_groups).build()?;
    module.function("capture_named", capture_named).build()?;

    // Replacement
    module.function("replace", replace_first).build()?;
    module.function("replace_all", replace_all).build()?;

    // Splitting
    module.function("split", split_by_pattern).build()?;

    // Validation
    module.function("is_valid", is_valid_pattern).build()?;

    // Utility
    module.function("escape", escape_pattern).build()?;
    module.function("count", count_matches).build()?;

    Ok(module)
}

/// Test if a pattern matches the input
fn test_match(input: &str, pattern: &str) -> bool {
    with_cached_regex(pattern, false, |re| re.is_match(input))
}

/// Find first match and return it
fn find_match(input: &str, pattern: &str) -> RuneString {
    with_cached_regex(pattern, RuneString::new(), |re| {
        if let Some(m) = re.find(input) {
            RuneString::try_from(m.as_str().to_string()).unwrap_or_default()
        } else {
            RuneString::new()
        }
    })
}

/// Find all matches and return as JSON array
fn find_all_matches(input: &str, pattern: &str) -> RuneString {
    with_cached_regex(
        pattern,
        RuneString::try_from("[]").unwrap_or_default(),
        |re| {
            let matches: Vec<&str> = re.find_iter(input).map(|m| m.as_str()).collect();
            let json = serde_json::to_string(&matches).unwrap_or("[]".to_string());
            RuneString::try_from(json).unwrap_or_default()
        },
    )
}

/// Capture groups and return as JSON array
fn capture_groups(input: &str, pattern: &str) -> RuneString {
    with_cached_regex(
        pattern,
        RuneString::try_from("[]").unwrap_or_default(),
        |re| {
            if let Some(caps) = re.captures(input) {
                let groups: Vec<String> = caps
                    .iter()
                    .skip(1) // Skip the full match
                    .map(|m| m.map(|m| m.as_str().to_string()).unwrap_or_default())
                    .collect();
                let json = serde_json::to_string(&groups).unwrap_or("[]".to_string());
                RuneString::try_from(json).unwrap_or_default()
            } else {
                RuneString::try_from("[]").unwrap_or_default()
            }
        },
    )
}

/// Capture named groups and return as JSON object
fn capture_named(input: &str, pattern: &str) -> RuneString {
    with_cached_regex(
        pattern,
        RuneString::try_from("{}").unwrap_or_default(),
        |re| {
            if let Some(caps) = re.captures(input) {
                let mut map = serde_json::Map::new();
                for name in re.capture_names().flatten() {
                    if let Some(m) = caps.name(name) {
                        map.insert(
                            name.to_string(),
                            serde_json::Value::String(m.as_str().to_string()),
                        );
                    }
                }
                let json = serde_json::to_string(&map).unwrap_or("{}".to_string());
                RuneString::try_from(json).unwrap_or_default()
            } else {
                RuneString::try_from("{}").unwrap_or_default()
            }
        },
    )
}

/// Replace first match
fn replace_first(input: &str, pattern: &str, replacement: &str) -> RuneString {
    with_cached_regex(
        pattern,
        RuneString::try_from(input.to_string()).unwrap_or_default(),
        |re| {
            let result = re.replace(input, replacement).to_string();
            RuneString::try_from(result).unwrap_or_default()
        },
    )
}

/// Replace all matches
fn replace_all(input: &str, pattern: &str, replacement: &str) -> RuneString {
    with_cached_regex(
        pattern,
        RuneString::try_from(input.to_string()).unwrap_or_default(),
        |re| {
            let result = re.replace_all(input, replacement).to_string();
            RuneString::try_from(result).unwrap_or_default()
        },
    )
}

/// Split string by pattern
fn split_by_pattern(input: &str, pattern: &str) -> RuneString {
    with_cached_regex(
        pattern,
        {
            let json = serde_json::to_string(&[input]).unwrap_or("[]".to_string());
            RuneString::try_from(json).unwrap_or_default()
        },
        |re| {
            let parts: Vec<&str> = re.split(input).collect();
            let json = serde_json::to_string(&parts).unwrap_or("[]".to_string());
            RuneString::try_from(json).unwrap_or_default()
        },
    )
}

/// Check if a regex pattern is valid
fn is_valid_pattern(pattern: &str) -> bool {
    Regex::new(pattern).is_ok()
}

/// Escape special regex characters in a string
fn escape_pattern(input: &str) -> RuneString {
    RuneString::try_from(regex::escape(input)).unwrap_or_default()
}

/// Count the number of matches
fn count_matches(input: &str, pattern: &str) -> i64 {
    with_cached_regex(pattern, 0, |re| re.find_iter(input).count() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_match() {
        assert!(test_match("hello world", r"world"));
        assert!(test_match("email@example.com", r"[\w]+@[\w]+\.[\w]+"));
        assert!(!test_match("hello", r"world"));
    }

    #[test]
    fn test_find_match() {
        let result = find_match("hello 123 world 456", r"\d+");
        assert_eq!(result.as_str(), "123");
    }

    #[test]
    fn test_find_all_matches() {
        let result = find_all_matches("a1b2c3", r"\d");
        assert!(result.contains("1"));
        assert!(result.contains("2"));
        assert!(result.contains("3"));
    }

    #[test]
    fn test_capture_groups() {
        let result = capture_groups("John Smith", r"(\w+) (\w+)");
        assert!(result.contains("John"));
        assert!(result.contains("Smith"));
    }

    #[test]
    fn test_capture_named() {
        let result = capture_named("John Smith", r"(?P<first>\w+) (?P<last>\w+)");
        assert!(result.contains("first"));
        assert!(result.contains("last"));
    }

    #[test]
    fn test_replace_all() {
        let result = replace_all("a1b2c3", r"\d", "X");
        assert_eq!(result.as_str(), "aXbXcX");
    }

    #[test]
    fn test_split() {
        let result = split_by_pattern("a,b;c", r"[,;]");
        assert!(result.contains("a"));
        assert!(result.contains("b"));
        assert!(result.contains("c"));
    }

    #[test]
    fn test_count_matches() {
        assert_eq!(count_matches("a1b2c3", r"\d"), 3);
    }

    #[test]
    fn test_escape() {
        let escaped = escape_pattern("hello.world");
        assert!(escaped.contains(r"\."));
    }
}
