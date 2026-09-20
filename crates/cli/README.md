# prv-cli

Command-line interface for the `prv` Spec-and-Go workflow.

## Overview

`prv-cli` provides a CLI for interacting with the PRV project lifecycle:

- `prv-cli spec start` — initialize a new spec
- `prv-cli spec validate` — validate current spec against codebase
- `prv-cli spec plan` — generate implementation plan
- `prv-cli living update` — update living documentation from spec
- `prv-cli session handover` — write session handover notes
- `prv-cli status` — show workspace status

The CLI is built with `clap` and uses `toml_edit` for living documentation updates.

## Installation

```bash
cargo install --path crates/cli
```

## Usage

```bash
prv-cli spec validate
prv-cli living update
```

## Dependencies

- All workspace crates (`prv-core`, `prv-filter`, `prv-monte-carlo`, `prv-policy`, `prv-geometry`, `prv-data`, `prv-evaluation`)
- `clap` — argument parsing
- `toml_edit` — TOML manipulation
- `thesix` — data access and cache abstraction
- `themql-core` / `themql-runtime` — message query and dispatch

## License

MIT © Metis Avionics
