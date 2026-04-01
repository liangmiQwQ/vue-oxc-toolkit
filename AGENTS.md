# AGENTS.md

This file is a quick operating guide for coding agents working in this repository.

## Project Summary

- Name: `vue-oxc-toolkit`
- Goal: Parse Vue SFCs and produce semantically-correct OXC-compatible AST and related lint metadata.
- Language: Rust
- Workspace layout:
  - `crates/vue_oxc_toolkit`: main library crate
  - `benchmark`: benchmark crate

## Ground Rules

- Keep changes focused and minimal.
- Do not change public behavior without tests.
- Prefer existing patterns in `crates/vue_oxc_toolkit/src/parser`.
- Update docs when behavior or APIs change.
- Avoid unrelated refactors in the same patch.

## Setup

Prerequisites:

- Rust toolchain (see `rust-toolchain.toml`)
- `just` task runner

Bootstrap environment:

```bash
cargo install just
just init
```

## Daily Commands

List tasks:

```bash
just
```

Format:

```bash
just fmt
```

Lint:

```bash
just lint
```

Test:

```bash
just test
```

Pre-CI local gate:

```bash
just ready
```

Build:

```bash
just build
```

Benchmark:

```bash
just bench
```

Coverage:

```bash
just coverage
```

## Testing Notes

- Parser behavior is heavily snapshot-driven.
- Snapshot fixtures live under:
  - `crates/vue_oxc_toolkit/fixtures`
  - `crates/vue_oxc_toolkit/src/parser/snapshots`
- When parser output intentionally changes, update snapshots accordingly.

## Contribution Expectations

- Ensure `just lint`, `just fmt`, `just test` passes before finishing.
- Add or update tests for bug fixes and features.
- Keep commit/PR titles in conventional commit style when possible.
