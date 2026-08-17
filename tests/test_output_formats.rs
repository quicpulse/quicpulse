//! Unit tests for OutputFlags, PrintOptions, and pretty printing settings

use quicpulse::output::options::{OutputFlags, PrettyOption};

#[test]
fn test_output_flags_parsing() {
    let flags_all = OutputFlags::from_print_str("HhBbm");
    assert!(flags_all.contains(OutputFlags::REQUEST_HEADERS));
    assert!(flags_all.contains(OutputFlags::REQUEST_BODY));
    assert!(flags_all.contains(OutputFlags::RESPONSE_HEADERS));
    assert!(flags_all.contains(OutputFlags::RESPONSE_BODY));
    assert!(flags_all.contains(OutputFlags::METADATA));

    let flags_headers = OutputFlags::from_print_str("Hh");
    assert_eq!(flags_headers, OutputFlags::HEADERS);

    let flags_bodies = OutputFlags::from_print_str("Bb");
    assert_eq!(flags_bodies, OutputFlags::BODIES);

    let flags_offline = OutputFlags::from_print_str("HB");
    assert_eq!(flags_offline, OutputFlags::OFFLINE);
}

#[test]
fn test_output_flags_to_print_str() {
    let flags = OutputFlags::REQUEST_HEADERS | OutputFlags::RESPONSE_BODY;
    let s = flags.to_print_str();
    assert!(s.contains('H'));
    assert!(s.contains('b'));
    assert!(!s.contains('h'));
    assert!(!s.contains('B'));
}

#[test]
fn test_pretty_option_helpers() {
    assert!(PrettyOption::All.uses_colors());
    assert!(PrettyOption::All.uses_formatting());

    assert!(PrettyOption::Colors.uses_colors());
    assert!(!PrettyOption::Colors.uses_formatting());

    assert!(!PrettyOption::Format.uses_colors());
    assert!(PrettyOption::Format.uses_formatting());

    assert!(!PrettyOption::None.uses_colors());
    assert!(!PrettyOption::None.uses_formatting());
}
