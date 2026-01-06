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
