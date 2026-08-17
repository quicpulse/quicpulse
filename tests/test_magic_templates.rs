//! Unit tests for Dynamic Magic Value expansions ({uuid}, {now}, {random_*}, {seq})

use quicpulse::magic::{expand_magic_values, has_magic_values, reset_seq_counter};

#[test]
fn test_has_magic_values_detection() {
    assert!(has_magic_values("id={uuid}"));
    assert!(has_magic_values("created={now}"));
    assert!(has_magic_values("code={random_string:10}"));
    assert!(has_magic_values("count={random_int:1:100}"));
    assert!(has_magic_values("n={seq}"));
    assert!(!has_magic_values("plain_string_without_magic"));
}

#[test]
fn test_expand_uuid_and_timestamp() {
    let res = expand_magic_values("id={uuid}&ts={timestamp}&ts_ms={timestamp_ms}");
    assert!(res.had_magic);
    assert!(!res.value.contains("{uuid}"));
    assert!(!res.value.contains("{timestamp}"));
    assert!(!res.value.contains("{timestamp_ms}"));
    assert!(res.generated.contains_key("{uuid}"));
}

#[test]
fn test_expand_random_numbers_and_strings() {
    let res =
        expand_magic_values("num={random_int:50:60}&str={random_string:12}&hex={random_hex:8}");
    assert!(res.had_magic);
    assert!(!res.value.contains("{random_int"));
    assert!(!res.value.contains("{random_string"));
    assert!(!res.value.contains("{random_hex"));
}

#[test]
fn test_expand_sequence_counter() {
    reset_seq_counter();
    let res1 = expand_magic_values("item_{seq}");
    let res2 = expand_magic_values("item_{seq}");
    assert_eq!(res1.value, "item_0");
    assert_eq!(res2.value, "item_1");

    reset_seq_counter();
    let res3 = expand_magic_values("item_{seq}");
    assert_eq!(res3.value, "item_0");
}
