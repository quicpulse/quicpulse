//! Interrupt/signal handling for graceful shutdown
//!
//! Provides global state for Ctrl+C handling across the application.

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag for Ctrl+C interrupt handling
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Check if the application was interrupted (Ctrl+C pressed)
#[inline]
pub fn was_interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Set the interrupted flag (called from signal handler)
#[inline]
pub fn set_interrupted() {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

/// Reset the interrupted flag
#[inline]
pub fn reset_interrupted() {
    INTERRUPTED.store(false, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_flag() {
        reset_interrupted();
        assert!(!was_interrupted());

        set_interrupted();
        assert!(was_interrupted());

        reset_interrupted();
        assert!(!was_interrupted());
    }
}
