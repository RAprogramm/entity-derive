// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

#[test]
fn compile_pass() {
    let t = trybuild::TestCases::new();
    t.pass("tests/cases/pass/*.rs");
}

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/cases/fail/*.rs");
}
