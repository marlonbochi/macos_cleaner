//! Conversions between domain types and SQLite column values.

use std::path::{Path, PathBuf};

use cleaner_core::domain::size::ByteSize;
use cleaner_core::error::RepoError;

/// SQLite integers are signed; `u64` values are stored bit-for-bit so they round-trip exactly.
pub(crate) const fn u64_to_sql(value: u64) -> i64 {
    value.cast_signed()
}

pub(crate) const fn u64_from_sql(value: i64) -> u64 {
    value.cast_unsigned()
}

pub(crate) const fn size_to_sql(size: ByteSize) -> i64 {
    u64_to_sql(size.bytes())
}

pub(crate) const fn size_from_sql(value: i64) -> ByteSize {
    ByteSize::new(u64_from_sql(value))
}

/// APFS only allows UTF-8 names, so a non-UTF-8 path is a bug worth surfacing.
pub(crate) fn path_to_sql(path: &Path) -> Result<&str, RepoError> {
    path.to_str()
        .ok_or_else(|| RepoError::Storage(format!("non UTF-8 path: {}", path.display())))
}

pub(crate) fn path_from_sql(value: String) -> PathBuf {
    PathBuf::from(value)
}

/// Escapes `%`, `_` and `\` for use in a `LIKE ... ESCAPE '\'` pattern.
pub(crate) fn escape_like(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

pub(crate) fn invalid_column(column: &str, value: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(0, format!("{column}={value}"), rusqlite::types::Type::Text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_round_trips_including_values_above_i64_max() {
        for v in [0, 1, u64::from(u32::MAX), u64::MAX / 2 + 1, u64::MAX] {
            assert_eq!(u64_from_sql(u64_to_sql(v)), v);
        }
    }

    #[test]
    fn like_escaping_covers_wildcards() {
        assert_eq!(escape_like("a_b%c\\d"), "a\\_b\\%c\\\\d");
    }
}
