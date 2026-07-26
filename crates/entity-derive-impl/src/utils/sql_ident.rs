// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Compile-time validation of the identifiers that reach generated SQL.
//!
//! Table, schema and column names are interpolated into the generated
//! statements unquoted. A name that needs quoting — a reserved word, an
//! upper-case letter, a space — produces SQL that parses only at
//! runtime, and only once that particular statement is executed. This
//! module turns that into a compile error at the attribute that caused
//! it.
//!
//! ```rust,ignore
//! #[entity(table = "user")]   // rejected: `user` is reserved in SQL
//! #[entity(table = "Users")]  // rejected: unquoted SQL folds it to `users`
//! ```

/// SQL keywords that cannot appear unquoted where this macro puts
/// identifiers.
///
/// Postgres tolerates some of these as column names but not as table
/// names, and the set differs between dialects; the list is the
/// intersection that is unsafe everywhere the generator interpolates a
/// name.
const RESERVED: &[&str] = &[
    "all",
    "analyse",
    "analyze",
    "and",
    "any",
    "array",
    "as",
    "asc",
    "authorization",
    "between",
    "binary",
    "both",
    "case",
    "cast",
    "check",
    "collate",
    "column",
    "constraint",
    "create",
    "cross",
    "current_date",
    "current_role",
    "current_time",
    "current_timestamp",
    "current_user",
    "default",
    "deferrable",
    "desc",
    "distinct",
    "do",
    "else",
    "end",
    "except",
    "false",
    "for",
    "foreign",
    "freeze",
    "from",
    "full",
    "grant",
    "group",
    "having",
    "ilike",
    "in",
    "initially",
    "inner",
    "intersect",
    "into",
    "is",
    "isnull",
    "join",
    "leading",
    "left",
    "like",
    "limit",
    "localtime",
    "localtimestamp",
    "natural",
    "not",
    "notnull",
    "null",
    "offset",
    "on",
    "only",
    "or",
    "order",
    "outer",
    "overlaps",
    "placing",
    "primary",
    "references",
    "returning",
    "right",
    "select",
    "session_user",
    "similar",
    "some",
    "symmetric",
    "table",
    "then",
    "to",
    "trailing",
    "true",
    "union",
    "unique",
    "user",
    "using",
    "variadic",
    "verbose",
    "when",
    "where",
    "window",
    "with"
];

/// Longest identifier Postgres keeps without truncating it.
const MAX_LEN: usize = 63;

/// Check one identifier destined for generated SQL.
///
/// `kind` names the attribute in the error message (`table`, `schema`,
/// `column`), so the report points at what to change.
///
/// # Errors
///
/// Returns the message to hand to `compile_error!` when the name would
/// need quoting to survive in generated SQL.
pub fn validate(kind: &str, name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err(format!("{kind} name must not be empty"));
    }

    if name.len() > MAX_LEN {
        return Err(format!(
            "{kind} name `{name}` is {} bytes; Postgres truncates identifiers at {MAX_LEN}, \
             which would silently rename it",
            name.len()
        ));
    }

    if let Some(bad) = name
        .chars()
        .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_'))
    {
        return Err(format!(
            "{kind} name `{name}` contains `{bad}`; generated SQL interpolates it unquoted, so \
             it must be lower-case ASCII, digits or underscore"
        ));
    }

    if name.starts_with(|c: char| c.is_ascii_digit()) {
        return Err(format!(
            "{kind} name `{name}` starts with a digit; SQL identifiers must start with a letter \
             or underscore"
        ));
    }

    if RESERVED.contains(&name) {
        return Err(format!(
            "{kind} name `{name}` is a reserved SQL word and would need quoting; rename it (for \
             example `{name}s`)"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn plain_names_pass() {
        assert!(validate("table", "users").is_ok());
        assert!(validate("column", "created_at").is_ok());
        assert!(validate("column", "addr_line_2").is_ok());
        assert!(validate("schema", "_internal").is_ok());
    }

    #[test]
    fn reserved_words_are_rejected() {
        let err = validate("table", "user").unwrap_err();
        assert!(err.contains("reserved"), "{err}");
        assert!(
            err.contains("`users`"),
            "the hint should suggest a fix: {err}"
        );
        assert!(validate("table", "order").is_err());
        assert!(validate("column", "default").is_err());
    }

    #[test]
    fn case_and_punctuation_are_rejected() {
        assert!(validate("table", "Users").is_err());
        assert!(validate("table", "user profiles").is_err());
        assert!(validate("table", "users; DROP TABLE x").is_err());
        assert!(validate("column", "créé_à").is_err());
    }

    #[test]
    fn shape_rules_are_enforced() {
        assert!(validate("table", "").is_err());
        assert!(validate("table", "2fa_tokens").is_err());
        assert!(validate("table", &"a".repeat(64)).is_err());
        assert!(validate("table", &"a".repeat(63)).is_ok());
    }
}
