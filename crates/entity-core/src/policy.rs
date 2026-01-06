// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Policy/authorization types for entity-derive.

use std::fmt;

/// Error type for policy-checked repository operations.
#[derive(Debug)]
pub enum PolicyError<R, P> {
    /// Authorization was denied by the policy.
    Policy(P),
    /// Repository operation failed.
    Repository(R)
}

impl<R, P> PolicyError<R, P> {
    /// Check if this is a policy (authorization) error.
    pub const fn is_policy(&self) -> bool {
        matches!(self, Self::Policy(_))
    }

    /// Check if this is a repository (database) error.
    pub const fn is_repository(&self) -> bool {
        matches!(self, Self::Repository(_))
    }
}

impl<R: fmt::Display, P: fmt::Display> fmt::Display for PolicyError<R, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(e) => write!(f, "authorization denied: {}", e),
            Self::Repository(e) => write!(f, "repository error: {}", e)
        }
    }
}

impl<R: std::error::Error + 'static, P: std::error::Error + 'static> std::error::Error
    for PolicyError<R, P>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Policy(e) => Some(e),
            Self::Repository(e) => Some(e)
        }
    }
}

/// Operation kind for policy checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolicyOperation {
    /// Create a new entity.
    Create,
    /// Read/find an entity.
    Read,
    /// Update an existing entity.
    Update,
    /// Delete an entity.
    Delete,
    /// List entities.
    List,
    /// Execute a command.
    Command
}

impl PolicyOperation {
    /// Check if this is a read-only operation.
    pub const fn is_read_only(&self) -> bool {
        matches!(self, Self::Read | Self::List)
    }

    /// Check if this is a mutation operation.
    pub const fn is_mutation(&self) -> bool {
        !self.is_read_only()
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

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
    fn policy_error_is_policy() {
        let err: PolicyError<TestError, TestError> = PolicyError::Policy(TestError("denied"));
        assert!(err.is_policy());
        assert!(!err.is_repository());
    }

    #[test]
    fn policy_error_is_repository() {
        let err: PolicyError<TestError, TestError> = PolicyError::Repository(TestError("db"));
        assert!(err.is_repository());
        assert!(!err.is_policy());
    }

    #[test]
    fn policy_error_display() {
        let policy: PolicyError<TestError, TestError> = PolicyError::Policy(TestError("denied"));
        assert_eq!(format!("{}", policy), "authorization denied: denied");

        let repo: PolicyError<TestError, TestError> = PolicyError::Repository(TestError("db"));
        assert_eq!(format!("{}", repo), "repository error: db");
    }

    #[test]
    fn policy_error_source() {
        let policy: PolicyError<TestError, TestError> = PolicyError::Policy(TestError("denied"));
        assert!(policy.source().is_some());

        let repo: PolicyError<TestError, TestError> = PolicyError::Repository(TestError("db"));
        assert!(repo.source().is_some());
    }

    #[test]
    fn policy_operation_is_read_only() {
        assert!(PolicyOperation::Read.is_read_only());
        assert!(PolicyOperation::List.is_read_only());
        assert!(!PolicyOperation::Create.is_read_only());
        assert!(!PolicyOperation::Update.is_read_only());
        assert!(!PolicyOperation::Delete.is_read_only());
        assert!(!PolicyOperation::Command.is_read_only());
    }

    #[test]
    fn policy_operation_is_mutation() {
        assert!(!PolicyOperation::Read.is_mutation());
        assert!(!PolicyOperation::List.is_mutation());
        assert!(PolicyOperation::Create.is_mutation());
        assert!(PolicyOperation::Update.is_mutation());
        assert!(PolicyOperation::Delete.is_mutation());
        assert!(PolicyOperation::Command.is_mutation());
    }
}
