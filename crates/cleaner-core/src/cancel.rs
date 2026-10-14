//! A runtime-agnostic cancellation flag shared between the UI thread and workers.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Cheap to clone; all clones observe the same flag.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        // Release pairs with the Acquire in `is_cancelled`, so work done before
        // cancelling is visible to the thread that observes the flag.
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_the_same_flag() {
        let token = CancellationToken::new();
        let clone = token.clone();
        assert!(!clone.is_cancelled());
        token.cancel();
        assert!(clone.is_cancelled());
    }
}
