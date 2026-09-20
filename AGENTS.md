# Working with this HelixDB project

This project uses [HelixDB](https://docs.helix-db.com). `helix.toml` holds the project
config; `.helix/` holds instance state (gitignored). The full docs index for agents is at
<https://docs.helix-db.com/llms.txt>.

## Workflow

```bash
helix start prv                                  # start the local instance (Docker/Podman)
helix query prv --file examples/request.json     # send a query JSON request
helix query prv -e 'readBatch().varAs("users", g().nWithLabel("User")).returning(["users"])'
helix status                                  # instance state
helix logs prv                                   # container logs
helix stop prv                                   # stop (in-memory data is lost; use --disk to persist)
```

There is no `helix compile` or `helix check` — queries are validated by the running
instance. `helix query -e` evaluates a TypeScript DSL expression and needs Node 20+.

## If `helix` is not installed

macOS and Linux:

```bash
curl -sSL "https://install.helix-db.com" | bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/HelixDB/helix-db/main/crates/cli/install.ps1 | iex
```

## If the container runtime is unavailable

`helix start` needs a working Docker or Podman:

- macOS: `brew install --cask docker`, or `brew install colima docker && colima start`.
- Linux: `curl -fsSL https://get.docker.com | sh`, or `apt-get install -y podman` and set
  `container_runtime = "podman"` under `[project]` in `helix.toml`.
- Daemon installed but stopped: `open -a Docker` (macOS) or `sudo systemctl start docker` /
  `sudo dockerd &` (Linux). `helix start` also tries this automatically.
- Restricted sandboxes without root usually cannot run containers. Use a host where Docker
  works, or point queries at a reachable instance with `helix query --host <h> --port <p>`.

## Query syntax

- TypeScript DSL: <https://docs.helix-db.com/database/querying-guide/overview>
- Dynamic JSON request shape: <https://docs.helix-db.com/cli/command-reference/query>

## PRV Project Context

This project uses the Spec-and-Go methodology. Each agent turn:
1. Loads `spec.toml` + `specs/*.toml` as the authoritative contract
2. Inspects `docs/session.md` and `docs/handover.md` for context
3. Executes within current spec boundaries
4. Runs `prv-cli living update` at turn completion
5. Records handover for the next agent

The TOML spec is the memory; Markdown is a generated projection.
