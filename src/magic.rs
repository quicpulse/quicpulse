//! Dynamic "Magic" Values
//!
//! Provides runtime data generation with special template tags like:
//! - `{uuid}` or `{uuid4}` - Random UUID v4
//! - `{uuid7}` - Time-ordered UUID v7
//! - `{now}` - Current ISO 8601 timestamp
//! - `{now:FORMAT}` - Custom formatted timestamp
//! - `{timestamp}` - Unix timestamp (seconds)
//! - `{timestamp_ms}` - Unix timestamp (milliseconds)
//! - `{random_int}` - Random integer (0 to i64::MAX)
//! - `{random_int:MIN:MAX}` - Random integer in range
//! - `{random_float}` - Random float (0.0 to 1.0)
//! - `{random_string:LEN}` - Random alphanumeric string
//! - `{random_hex:LEN}` - Random hex string
//! - `{random_bytes:LEN}` - Random base64-encoded bytes
//! - `{env:VAR_NAME}` - Environment variable value

use chrono::{Utc, Local};
use once_cell::sync::Lazy;
use rand::Rng;
use regex::Regex;
use std::cell::Cell;
use std::collections::HashMap;
use uuid::Uuid;

// Thread-local sequence counter for {seq} magic value
// Using thread_local to avoid global state pollution between workflows
thread_local! {
    static SEQ_COUNTER: Cell<u64> = const { Cell::new(0) };
}

// Cached regex patterns to avoid recompilation in hot paths
static MAGIC_VALUE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{([a-z_][a-z0-9_]*)(?::([^}]*))?\}").unwrap()
});
static HAS_MAGIC_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{[a-z_][a-z0-9_]*(?::[^}]*)?\}").unwrap()
});

/// Reset the sequence counter (call between workflows for reproducible tests)
pub fn reset_seq_counter() {
    SEQ_COUNTER.with(|c| c.set(0));
}

/// Result of expanding magic values
#[derive(Debug)]
pub struct ExpansionResult {
    /// The expanded string
    pub value: String,
    /// Whether any magic values were found and expanded
    pub had_magic: bool,
    /// Values that were generated (for logging/debugging)
    pub generated: HashMap<String, String>,
}

const MAX_EXPANSION_DEPTH: usize = 10;

/// Expand all magic values in a string
pub fn expand_magic_values(input: &str) -> ExpansionResult {
    let mut result = input.to_string();
    let mut had_magic = false;
    let mut generated = HashMap::new();

    // Use cached regex pattern
    let re = &*MAGIC_VALUE_RE;

    let mut depth = 0;

    // Keep replacing until no more matches (handles nested cases)
    loop {
        depth += 1;
        if depth > MAX_EXPANSION_DEPTH {
            break;
        }

        let mut found_match = false;

        // Find all matches and replace them
        let replacements: Vec<_> = re.captures_iter(&result)
            .map(|cap| {
                let full_match = cap.get(0).unwrap().as_str().to_string();
                let name = cap.get(1).unwrap().as_str();
                let args = cap.get(2).map(|m| m.as_str());

                let replacement = generate_magic_value(name, args);
                (full_match, replacement)
            })
            .collect();

        for (pattern, replacement) in replacements {
            if let Some(value) = replacement {
                // Do replacement first (borrowing pattern), then move both into generated
                result = result.replacen(&pattern, &value, 1);
                generated.insert(pattern, value);
                had_magic = true;
                found_match = true;
            }
        }

        if !found_match {
            break;
        }
    }

    ExpansionResult {
        value: result,
        had_magic,
        generated,
    }
}

/// Generate a magic value based on name and optional args
fn generate_magic_value(name: &str, args: Option<&str>) -> Option<String> {
    match name {
        // UUIDs
        "uuid" | "uuid4" => Some(Uuid::new_v4().to_string()),
        "uuid7" => {
            // UUID v7 is time-ordered
            Some(Uuid::now_v7().to_string())
        }

        // Timestamps
        "now" => {
            if let Some(format) = args {
                // Custom format
                Some(Utc::now().format(format).to_string())
            } else {
                // ISO 8601
                Some(Utc::now().to_rfc3339())
            }
        }
        "now_local" => {
            if let Some(format) = args {
                Some(Local::now().format(format).to_string())
            } else {
                Some(Local::now().to_rfc3339())
            }
        }
        "timestamp" => Some(Utc::now().timestamp().to_string()),
        "timestamp_ms" => Some(Utc::now().timestamp_millis().to_string()),
        "date" => Some(Utc::now().format("%Y-%m-%d").to_string()),
        "time" => Some(Utc::now().format("%H:%M:%S").to_string()),

        // Random integers
        "random_int" | "random" | "rand" => {
            let mut rng = rand::rng();
            if let Some(range_str) = args {
                // Parse MIN:MAX format
                let parts: Vec<&str> = range_str.split(':').collect();
                match parts.len() {
                    1 => {
                        // Just max
                        if let Ok(max) = parts[0].parse::<i64>() {
                            Some(rng.random_range(0..=max).to_string())
                        } else {
                            None
                        }
                    }
                    2 => {
                        // min:max
                        if let (Ok(min), Ok(max)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
                            Some(rng.random_range(min..=max).to_string())
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            } else {
                Some(rng.random_range(0..=i32::MAX as i64).to_string())
            }
        }

        // Random float
        "random_float" | "randf" => {
            let mut rng = rand::rng();
            if let Some(range_str) = args {
                let parts: Vec<&str> = range_str.split(':').collect();
                match parts.len() {
                    1 => {
                        if let Ok(max) = parts[0].parse::<f64>() {
                            Some(format!("{:.6}", rng.random_range(0.0..=max)))
                        } else {
                            None
                        }
                    }
                    2 => {
                        if let (Ok(min), Ok(max)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                            Some(format!("{:.6}", rng.random_range(min..=max)))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            } else {
                Some(format!("{:.6}", rng.random::<f64>()))
            }
        }

        // Random string (alphanumeric)
        "random_string" | "rands" => {
            let len = args.and_then(|s| s.parse().ok()).unwrap_or(16);
            Some(generate_random_string(len))
        }

        // Random hex string
        "random_hex" | "hex" => {
            let len = args.and_then(|s| s.parse().ok()).unwrap_or(32);
            Some(generate_random_hex(len))
        }

        // Random bytes (base64 encoded)
        "random_bytes" | "bytes" => {
            let len = args.and_then(|s| s.parse().ok()).unwrap_or(16);
            Some(generate_random_bytes_base64(len))
        }

        // Environment variable
        "env" => {
            args.and_then(|var_name| std::env::var(var_name).ok())
        }

        // Boolean
        "random_bool" | "bool" => {
            let mut rng = rand::rng();
            Some(rng.random_bool(0.5).to_string())
        }

        // Pick from list
        "pick" => {
            args.and_then(|options| {
                let items: Vec<&str> = options.split(',').collect();
                if items.is_empty() {
                    None
                } else {
                    let mut rng = rand::rng();
                    let idx = rng.random_range(0..items.len());
                    Some(items[idx].trim().to_string())
                }
            })
        }

        // Sequence/counter (useful in loops)
        "seq" => {
            let start = args.and_then(|s| s.parse().ok()).unwrap_or(0);
            let current = SEQ_COUNTER.with(|c| {
                let val = c.get();
                c.set(val + 1);
                val
            });
            Some((start + current).to_string())
        }

        // Reset sequence counter (for testing/reproducibility)
        "seq_reset" => {
            reset_seq_counter();
            Some("0".to_string())
        }

        // Email (random)
        "email" => {
            let user = generate_random_string(8).to_lowercase();
            let domain = args.unwrap_or("example.com");
            Some(format!("{}@{}", user, domain))
        }

        // Lorem ipsum placeholder
        "lorem" => {
            let words = args.and_then(|s| s.parse().ok()).unwrap_or(10);
            Some(generate_lorem(words))
        }

        // Name placeholder
        "first_name" => Some(random_first_name()),
        "last_name" => Some(random_last_name()),
        "full_name" => Some(format!("{} {}", random_first_name(), random_last_name())),

        _ => None,
    }
}

/// Generate a random alphanumeric string
fn generate_random_string(len: usize) -> String {
    use rand::distr::Alphanumeric;
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

/// Generate a random hex string
fn generate_random_hex(len: usize) -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..len / 2 + 1).map(|_| rng.random()).collect();
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    hex[..len].to_string()
}

/// Generate random bytes as base64
fn generate_random_bytes_base64(len: usize) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..len).map(|_| rng.random()).collect();
    STANDARD.encode(&bytes)
}

/// Generate lorem ipsum text
fn generate_lorem(words: usize) -> String {
    const LOREM_WORDS: &[&str] = &[
        "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit",
        "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore", "et", "dolore",
        "magna", "aliqua", "enim", "ad", "minim", "veniam", "quis", "nostrud",
        "exercitation", "ullamco", "laboris", "nisi", "aliquip", "ex", "ea", "commodo",
        "consequat", "duis", "aute", "irure", "in", "reprehenderit", "voluptate",
        "velit", "esse", "cillum", "fugiat", "nulla", "pariatur", "excepteur", "sint",
        "occaecat", "cupidatat", "non", "proident", "sunt", "culpa", "qui", "officia",
        "deserunt", "mollit", "anim", "id", "est", "laborum",
    ];

    let mut rng = rand::rng();
    (0..words)
        .map(|_| LOREM_WORDS[rng.random_range(0..LOREM_WORDS.len())])
        .collect::<Vec<_>>()
        .join(" ")
}

/// Random first name
fn random_first_name() -> String {
    const NAMES: &[&str] = &[
        "James", "Mary", "John", "Patricia", "Robert", "Jennifer", "Michael", "Linda",
        "William", "Elizabeth", "David", "Barbara", "Richard", "Susan", "Joseph", "Jessica",
        "Thomas", "Sarah", "Charles", "Karen", "Emma", "Olivia", "Ava", "Isabella",
        "Sophia", "Mia", "Charlotte", "Amelia", "Harper", "Evelyn",
    ];
    let mut rng = rand::rng();
    NAMES[rng.random_range(0..NAMES.len())].to_string()
}

/// Random last name
fn random_last_name() -> String {
    const NAMES: &[&str] = &[
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis",
        "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson",
        "Thomas", "Taylor", "Moore", "Jackson", "Martin", "Lee", "Perez", "Thompson",
        "White", "Harris", "Sanchez", "Clark", "Ramirez", "Lewis", "Robinson",
    ];
    let mut rng = rand::rng();
    NAMES[rng.random_range(0..NAMES.len())].to_string()
}

/// Check if a string contains any magic value patterns
pub fn has_magic_values(s: &str) -> bool {
    HAS_MAGIC_RE.is_match(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_expansion() {
        let result = expand_magic_values("id={uuid}");
        assert!(result.had_magic);
        assert!(result.value.starts_with("id="));
        assert!(result.value.len() > 10);
        // UUID format check
        let uuid_part = &result.value[3..];
        assert!(uuid_part.contains('-'));
    }

    #[test]
    fn test_uuid7_expansion() {
        let result = expand_magic_values("{uuid7}");
        assert!(result.had_magic);
        assert!(result.value.contains('-'));
    }

    #[test]
    fn test_timestamp_expansion() {
        let result = expand_magic_values("{timestamp}");
        assert!(result.had_magic);
        let ts: i64 = result.value.parse().unwrap();
        assert!(ts > 1700000000); // After Nov 2023
    }

    #[test]
    fn test_now_expansion() {
        let result = expand_magic_values("{now}");
        assert!(result.had_magic);
        assert!(result.value.contains('T')); // ISO 8601 format
    }

    #[test]
    fn test_now_custom_format() {
        let result = expand_magic_values("{now:%Y-%m-%d}");
        assert!(result.had_magic);
        assert!(result.value.len() == 10); // YYYY-MM-DD
    }

    #[test]
    fn test_random_int() {
        let result = expand_magic_values("{random_int}");
        assert!(result.had_magic);
        let num: i64 = result.value.parse().unwrap();
        assert!(num >= 0);
    }

    #[test]
    fn test_random_int_range() {
        let result = expand_magic_values("{random_int:1:10}");
        assert!(result.had_magic);
        let num: i64 = result.value.parse().unwrap();
        assert!(num >= 1 && num <= 10);
    }

    #[test]
    fn test_random_string() {
        let result = expand_magic_values("{random_string:8}");
        assert!(result.had_magic);
        assert_eq!(result.value.len(), 8);
    }

    #[test]
    fn test_random_hex() {
        let result = expand_magic_values("{random_hex:16}");
        assert!(result.had_magic);
        assert_eq!(result.value.len(), 16);
        assert!(result.value.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_env_expansion() {
        std::env::set_var("TEST_MAGIC_VAR", "test_value");
        let result = expand_magic_values("{env:TEST_MAGIC_VAR}");
        assert!(result.had_magic);
        assert_eq!(result.value, "test_value");
    }

    #[test]
    fn test_pick() {
        let result = expand_magic_values("{pick:a,b,c}");
        assert!(result.had_magic);
        assert!(["a", "b", "c"].contains(&result.value.as_str()));
    }

    #[test]
    fn test_multiple_values() {
        let result = expand_magic_values("id={uuid}&ts={timestamp}");
        assert!(result.had_magic);
        assert!(result.value.contains('&'));
        assert!(!result.value.contains("{uuid}"));
        assert!(!result.value.contains("{timestamp}"));
    }

    #[test]
    fn test_no_magic() {
        let result = expand_magic_values("normal string");
        assert!(!result.had_magic);
        assert_eq!(result.value, "normal string");
    }

    #[test]
    fn test_has_magic_values() {
        assert!(has_magic_values("{uuid}"));
        assert!(has_magic_values("id={random_int:1:100}"));
        assert!(!has_magic_values("normal string"));
        assert!(!has_magic_values("{NotMagic}")); // uppercase not matched
    }

    #[test]
    fn test_email() {
        let result = expand_magic_values("{email}");
        assert!(result.had_magic);
        assert!(result.value.contains('@'));
        assert!(result.value.ends_with("example.com"));
    }

    #[test]
    fn test_full_name() {
        let result = expand_magic_values("{full_name}");
        assert!(result.had_magic);
        assert!(result.value.contains(' '));
    }

    #[test]
    fn test_lorem() {
        let result = expand_magic_values("{lorem:5}");
        assert!(result.had_magic);
        let words: Vec<&str> = result.value.split_whitespace().collect();
        assert_eq!(words.len(), 5);
    }
}
