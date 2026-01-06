// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Streaming types for real-time entity updates.
//!
//! Provides error types and traits for async streams via Postgres
//! LISTEN/NOTIFY.

use std::fmt;

/// Error type for stream operations.
#[derive(Debug)]
pub enum StreamError<D> {
    /// Database/listener error.
    Database(D),
    /// JSON deserialization error.
    Deserialize(String)
}

impl<D> StreamError<D> {
    /// Check if this is a database error.
    pub const fn is_database(&self) -> bool {
        matches!(self, Self::Database(_))
    }

    /// Check if this is a deserialization error.
    pub const fn is_deserialize(&self) -> bool {
        matches!(self, Self::Deserialize(_))
    }
}

impl<D: fmt::Display> fmt::Display for StreamError<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(e) => write!(f, "database error: {}", e),
            Self::Deserialize(e) => write!(f, "deserialize error: {}", e)
        }
    }
}

impl<D: std::error::Error + 'static> std::error::Error for StreamError<D> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(e) => Some(e),
            Self::Deserialize(_) => None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[test]
    fn stream_error_is_database() {
        let err: StreamError<TestError> = StreamError::Database(TestError("db"));
        assert!(err.is_database());
        assert!(!err.is_deserialize());
    }

    #[test]
    fn stream_error_is_deserialize() {
        let err: StreamError<TestError> = StreamError::Deserialize("json".into());
        assert!(err.is_deserialize());
        assert!(!err.is_database());
    }

    #[test]
    fn stream_error_display() {
        let db: StreamError<TestError> = StreamError::Database(TestError("conn"));
        assert_eq!(format!("{}", db), "database error: conn");

        let de: StreamError<TestError> = StreamError::Deserialize("invalid".into());
        assert_eq!(format!("{}", de), "deserialize error: invalid");
    }

    #[test]
    fn stream_error_source() {
        use std::error::Error;

        let db: StreamError<TestError> = StreamError::Database(TestError("source"));
        assert!(db.source().is_some());

        let de: StreamError<TestError> = StreamError::Deserialize("no source".into());
        assert!(de.source().is_none());
    }
}
