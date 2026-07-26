<!--
SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
SPDX-License-Identifier: MIT
-->

# Getting help

Pick the channel that matches the question.

| Channel | Use it for |
|---|---|
| [Wiki](https://github.com/RAprogramm/entity-derive/wiki) | Attributes, filtering, relations, commands, hooks, web frameworks — the task-oriented guides, in five languages. |
| [docs.rs](https://docs.rs/entity-derive) | API reference for the generated items and the runtime traits. |
| [`examples/`](examples) | Ten runnable crates, one per feature area. |
| [Issues](https://github.com/RAprogramm/entity-derive/issues) | Reproducible defects and missing functionality. |
| [`SECURITY.md`](SECURITY.md) | **Anything that could be a vulnerability.** Never a public issue. |

## Before opening an issue

1. Search [closed issues](https://github.com/RAprogramm/entity-derive/issues?q=is%3Aissue+is%3Aclosed) — the answer is often there.
2. Reproduce against the latest published version (`cargo update`).
3. Reduce to a minimal entity definition, and say which features were
   enabled — most reports turn on a feature gate.
4. For a code-generation problem, include the expansion
   (`cargo expand`) or the generated SQL, not only the compiler error.

## Response time

The project is maintained on a best-effort basis; expect a first
response within a week. Security reports take priority through the
channel in `SECURITY.md`.

## Contributing a fix

[`CONTRIBUTING.md`](CONTRIBUTING.md) has the branch, commit and review
conventions, the local check commands and how to run the live Postgres
suite.
