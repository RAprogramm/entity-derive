// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Behavior tests for the `garde` validation backend.
//!
//! Run with `--features garde` (and without `validate`, which takes
//! precedence when both are enabled).

#![cfg(all(feature = "garde", not(feature = "validate")))]

use entity_derive::Entity;
use garde::Validate;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "users")]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[validate(length(min = 3, max = 8))]
    pub name: String,

    #[field(create, response)]
    #[validate(email)]
    pub email: String
}

#[test]
fn valid_create_dto_passes() {
    let dto = CreateUserRequest {
        name:  "alice".into(),
        email: "alice@example.com".into()
    };
    assert!(dto.validate().is_ok());
}

#[test]
fn short_name_fails_length_rule() {
    let dto = CreateUserRequest {
        name:  "al".into(),
        email: "alice@example.com".into()
    };
    assert!(dto.validate().is_err());
}

#[test]
fn bad_email_fails() {
    let dto = CreateUserRequest {
        name:  "alice".into(),
        email: "not-an-email".into()
    };
    assert!(dto.validate().is_err());
}

#[test]
fn update_dto_validates_inner_value() {
    let ok = UpdateUserRequest {
        name: Some("alice".into())
    };
    assert!(ok.validate().is_ok());

    let bad = UpdateUserRequest {
        name: Some("al".into())
    };
    assert!(bad.validate().is_err());

    let absent = UpdateUserRequest {
        name: None
    };
    assert!(absent.validate().is_ok());
}
