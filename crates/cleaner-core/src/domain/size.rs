//! `ByteSize`: a newtype around `u64` so sizes cannot be confused with counts or ids.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign};

use serde::{Deserialize, Serialize};

/// A number of bytes. Formatting uses base 1000, matching what Finder shows.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct ByteSize(u64);

impl ByteSize {
    pub const ZERO: Self = Self(0);

    pub const fn new(bytes: u64) -> Self {
        Self(bytes)
    }

    pub const fn from_mb(mb: u64) -> Self {
        Self(mb * 1_000_000)
    }

    pub const fn bytes(self) -> u64 {
        self.0
    }
}

impl From<u64> for ByteSize {
    fn from(bytes: u64) -> Self {
        Self(bytes)
    }
}

// Sizes never wrap: a saturating add is the honest answer for "how much space".
impl Add for ByteSize {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign for ByteSize {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for ByteSize {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, Add::add)
    }
}

impl fmt::Display for ByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
        if self.0 < 1000 {
            return write!(f, "{} B", self.0);
        }
        // Precision loss in the cast is irrelevant for a one-decimal display value.
        #[allow(clippy::cast_precision_loss)]
        let mut value = self.0 as f64;
        let mut unit = 0;
        while value >= 1000.0 && unit < UNITS.len() - 1 {
            value /= 1000.0;
            unit += 1;
        }
        write!(f, "{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes_below_one_kilobyte_without_decimals() {
        assert_eq!(ByteSize::new(0).to_string(), "0 B");
        assert_eq!(ByteSize::new(999).to_string(), "999 B");
    }

    #[test]
    fn formats_using_base_1000_with_one_decimal() {
        assert_eq!(ByteSize::new(1_000).to_string(), "1.0 KB");
        assert_eq!(ByteSize::new(1_500_000).to_string(), "1.5 MB");
        assert_eq!(ByteSize::new(5_000_000_000).to_string(), "5.0 GB");
        assert_eq!(ByteSize::new(2_340_000_000_000).to_string(), "2.3 TB");
    }

    #[test]
    fn addition_saturates_instead_of_overflowing() {
        assert_eq!(
            ByteSize::new(u64::MAX) + ByteSize::new(1),
            ByteSize::new(u64::MAX)
        );
    }

    #[test]
    fn sums_an_iterator() {
        let total: ByteSize = [1, 2, 3].into_iter().map(ByteSize::new).sum();
        assert_eq!(total, ByteSize::new(6));
    }

    #[test]
    fn from_mb_uses_decimal_megabytes() {
        assert_eq!(ByteSize::from_mb(100).bytes(), 100_000_000);
    }
}
