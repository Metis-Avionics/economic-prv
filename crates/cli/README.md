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
# Validate specs against codebase
prv-cli spec validate

# Update living docs from spec.toml + specs/*.toml
prv-cli living update

# Generate session handover notes
prv-cli session handover

# Show workspace status
prv-cli status
```

## Pipeline Demo

```bash
# Run the full economic simulation pipeline with faux data
cargo run --example run_pipeline -p prv-cli
```

This wires `DataLoader` → `Ekf` → `Simulator` → `PolicyEngine` → `Evaluator` end-to-end using `examples/faux_data.csv`.

## Dependencies

- All workspace crates (`prv-core`, `prv-filter`, `prv-monte-carlo`, `prv-policy`, `prv-geometry`, `prv-data`, `prv-evaluation`)
- `clap` — argument parsing
- `toml_edit` — TOML manipulation
- `thesix` — data access and cache abstraction
- `themql-core` / `themql-runtime` — message query and dispatch

## License

MIT © Metis Avionics
