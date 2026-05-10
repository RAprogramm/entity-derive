// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::ValueObject;

#[derive(ValueObject, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[value_object(pg_type = "order_status")]
pub enum OrderStatus1 {
    Pending,
    Confirmed,
    Cancelled,
}

fn main() {
    let status = OrderStatus1::Pending;
    let _ = status.as_ref();
    let _ = format!("{}", status);
    let parsed: OrderStatus1 = "pending".parse().unwrap();
    assert_eq!(parsed, OrderStatus1::Pending);
    let converted = OrderStatus1::try_from("confirmed").unwrap();
    assert_eq!(converted, OrderStatus1::Confirmed);
}
