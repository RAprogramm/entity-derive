// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::ValueObject;
use std::str::FromStr;

#[derive(ValueObject, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[value_object(pg_type = "user_role")]
pub enum UserRole {
    Admin,
    Moderator,
    User,
}

fn main() {
    // Test Display
    assert_eq!(format!("{}", UserRole::Admin), "admin");
    assert_eq!(format!("{}", UserRole::Moderator), "moderator");
    assert_eq!(format!("{}", UserRole::User), "user");

    // Test FromStr case-insensitive
    assert!(UserRole::from_str("admin").is_ok());
    assert!(UserRole::from_str("ADMIN").is_ok());
    assert!(UserRole::from_str("Admin").is_ok());
    assert!(UserRole::from_str("AdMiN").is_ok());

    assert!(UserRole::from_str("moderator").is_ok());
    assert!(UserRole::from_str("MODERATOR").is_ok());

    assert!(UserRole::from_str("user").is_ok());
    assert!(UserRole::from_str("USER").is_ok());

    // Test FromStr error
    assert!(UserRole::from_str("unknown").is_err());

    // Test FromStr round-trip
    for variant in [UserRole::Admin, UserRole::Moderator, UserRole::User] {
        let display = format!("{}", variant);
        let parsed = UserRole::from_str(&display).unwrap();
        assert_eq!(parsed, variant);
    }

    // Test AsRef
    assert_eq!(UserRole::Admin.as_ref(), "admin");
    assert_eq!(UserRole::Moderator.as_ref(), "moderator");
    assert_eq!(UserRole::User.as_ref(), "user");

    // Test TryFrom
    let a: UserRole = UserRole::try_from("admin").unwrap();
    assert_eq!(a, UserRole::Admin);

    let b = UserRole::try_from("ADMIN");
    assert!(b.is_ok());
    assert_eq!(b.unwrap(), UserRole::Admin);

    let c = UserRole::try_from("unknown");
    assert!(c.is_err());
}
