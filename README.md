<p align="center">
  <img src="assets/icon.png" width="128" alt="Flovo Dockit Logo" />
</p>

# Flovo Dockit

> **Your containers, under control.**  
> Docker operations, backups, recovery, and deployment safety in one CLI.

A tiny, opinionated CLI that orchestrates a repository's Docker Compose
projects — dependency-ordered startup, per-environment overlays,
variable/volume/network provisioning, backup/restore, and scaffolding, on top
of plain `docker compose`.

Dockit guides and orchestrates compose projects: `dockit up traefik`, `dockit pull bonob`, `dockit configure navidrome`.

## What it does

dockit sits on top of `docker compose` and never becomes a runtime dependency of
your services — it only assembles and spawns `docker` commands, and your
compose files stay runnable with plain `docker compose`.

- **Dependency-ordered lifecycle.** `up` starts a project *and its whole
  dependency chain*, dependencies first; `down`/`restart`/`pull`/`logs` act on
  one project; `all <verb>` fans out over everything running.
- **Per-environment compose overlays.** `docker-compose.yml` plus an optional
  `docker-compose.<env>.yml`, selected automatically.
- **Provisioning.** `configure` prompts for and persists required environment
  variables (with secrets), and creates the docker volumes and networks a
  project declares — for the project and all its dependencies at once.
- **Backup & restore** of a project's docker volumes to/from compressed
  archives.
- **Scaffolding.** `new <name>` stamps out a fresh app from templates.
- **Graph queries & completion.** `list`, `deps` (forward/reverse, transitive,
  JSON for CI), `ps`, and dynamic `bash`/`zsh`/`fish` completion.

Single self-contained binary. The only runtime dependency is the `docker`
binary on your `PATH` — no Node.js, no toolchain.

## The repository it expects

dockit is built for a repository laid out like this, where each service is a
directory with a `project.json` describing its name, dependencies, and the
variables/volumes/networks it needs:

```
<repoRoot>/
  apps/<name>/            # a service: docker-compose.yml + project.json
  libs/<name>/            # shared snippets and graph-only projects (e.g. networks)
  archive/<name>/         # retired services
  .env / .env.local       # dotenv layers
  .secrets/               # one file per secret
  .dockitignore          # optional: directories to stay out of entirely (.tugignore also supported)
```

```jsonc
// apps/bonob/project.json
{
  "name": "bonob",
  "implicitDependencies": ["networks", "traefik", "navidrome"],
  "targets": {
    "create-volumes":   { "options": { "volumes":   ["bonob-data"] } },
    "create-variables": { "options": { "variables": [ /* … */ ] } }
  }
}
```

Directory names (`apps/`, `libs/`, `archive/`) are convention — discovery scans
the whole repo for `project.json` files, skipping dot-directories and anything
an optional root `.dockitignore` (or `.tugignore`) names. The full contract is in
[`docs/reference.md`](docs/reference.md); the vocabulary is in
[`CONTEXT.md`](CONTEXT.md).

## Install

Prebuilt binaries are static `x86_64-unknown-linux-musl` and run on any x86_64
Linux. For other platforms, build from source.

**Install script** (latest release into `~/.local/bin`):

```sh
curl -fsSL https://raw.githubusercontent.com/raeffs/tug/main/install.sh | sh
```

Pin a version or change the target directory:

```sh
curl -fsSL https://raw.githubusercontent.com/raeffs/tug/main/install.sh \
  | DOCKIT_VERSION=v1.0.0 DOCKIT_INSTALL_DIR=/usr/local/bin sh
```

**Manual download** (verify the checksum yourself):

```sh
VERSION=v1.0.0
base=https://github.com/raeffs/tug/releases/download/$VERSION
curl -fsSLO $base/dockit-$VERSION-x86_64-unknown-linux-musl
curl -fsSLO $base/dockit-$VERSION-x86_64-unknown-linux-musl.sha256
sha256sum -c dockit-$VERSION-x86_64-unknown-linux-musl.sha256
install -m 0755 dockit-$VERSION-x86_64-unknown-linux-musl ~/.local/bin/dockit
```

**From source** (needs a Rust toolchain):

```sh
cargo install --git https://github.com/raeffs/tug   # or: cargo build --release
```

## Shell completions

`dockit completions <shell>` prints a completion script for `bash`, `zsh`, or
`fish`. Completion is dynamic — it completes subcommands, flags, and your
repository's discovered project names. Enable it for your shell:

```sh
# bash — add to ~/.bashrc:
source <(dockit completions bash)

# zsh — write to a directory on your $fpath, before compinit:
dockit completions zsh > ~/.zsh/completions/_dockit

# fish:
dockit completions fish > ~/.config/fish/completions/dockit.fish
```

`dockit` must be on your `$PATH` when completion runs, since the script calls back
out to it. See [`docs/reference.md`](docs/reference.md) for the details.

## Quickstart

```sh
# From anywhere inside your repository:
dockit list                      # every project and its direct dependencies
dockit configure bonob --use-defaults   # provision variables/volumes/networks for the chain
dockit up bonob                  # start bonob and its dependencies, in order
dockit ps                        # what's running
dockit logs bonob --follow       # tail its logs
dockit restart bonob             # bounce just bonob
dockit down bonob                # stop just bonob
```

Preview exactly what would run, without touching docker:

```sh
dockit up bonob --dry-run
```

## Commands

| Command | What it does |
|---|---|
| `up <project>` | Start the project **and its dependency chain**, in order. `--no-deps` starts the project alone. `--force`, `--no-detach`, `--wait [--wait-timeout <s>]`, `--dry-run`. |
| `down <project>` | Stop just the project (no cascade). |
| `restart <project>` | Bounce just the project: down then up. |
| `pull <project>` | Pull the project's images (no cascade). |
| `logs <project>` | Stream the project's logs. `--follow`. |
| `all down\|restart\|pull` | Run a verb over every **running** project, best-effort. |
| `list` | Every project with its direct dependencies. |
| `deps <project>` | Query the dependency graph. `--reverse`, `--transitive`, `--json`. |
| `ps` | The repository's running projects and their status. |
| `configure <project>` | Provision variables, volumes, and networks for the chain. `--use-defaults`. |
| `create-variables / create-volumes / create-networks <project>` | The individual provisioning steps. |
| `backup-volumes / restore-volumes <project>` | Archive/restore the project's own volumes. |
| `new <name>` | Scaffold a new app under `apps/`. |
| `completions <bash\|zsh\|fish>` | Print a shell completion script. |

Every command takes `--help`. Full behaviour — exact `docker` invocations,
environment resolution, provisioning rules, error handling — is in
[`docs/reference.md`](docs/reference.md).

## Using dockit in CI

`configure --use-defaults` and `up --wait` are the CI-facing commands; `deps
--reverse --transitive --json` lets a pipeline compute the affected-project set.
When a job only exercises the app itself, `up --wait --no-deps` skips booting
the dependency chain — `configure` still needs the full cascade to resolve
variables the dependencies define.
Install a pinned, checksum-verified binary in a GitHub Actions step:

```yaml
- name: Install dockit
  run: |
    VERSION=v1.0.0
    base=https://github.com/raeffs/tug/releases/download/$VERSION
    curl -fsSLO $base/dockit-$VERSION-x86_64-unknown-linux-musl
    curl -fsSLO $base/dockit-$VERSION-x86_64-unknown-linux-musl.sha256
    sha256sum -c dockit-$VERSION-x86_64-unknown-linux-musl.sha256
    install -m 0755 dockit-$VERSION-x86_64-unknown-linux-musl /usr/local/bin/dockit
```

## Development

dockit is Rust, with a deliberately minimal dependency set (`clap`, `serde`,
`serde_json`, `getrandom`, `anyhow`) — everything else is hand-rolled. The
toolchain is pinned in `rust-toolchain.toml`.

```sh
cargo build
cargo test          # unit + integration tests; no docker required
cargo fmt --check
cargo clippy        # lints: clippy `all` + `pedantic`
```

The tests need no docker: every external effect goes through a process-spawn
**seam** (a `docker` invocation described as data), so tests drive the CLI and
assert exact argv sequences against fixture repositories, and `--dry-run` is
just a printing implementation of the same seam. See
[`CONTEXT.md`](CONTEXT.md) for the vocabulary.

## Credits & Acknowledgments

Flovo Dockit is built upon the foundation of [**tug**](https://github.com/raeffs/tug), originally designed and developed by [**Raphael Fleischlin**](https://github.com/raeffs). We extend our deep gratitude to Raphael for the elegant design philosophy, process-spawn seam architecture, and rock-solid Compose orchestration fundamentals. Flovo Dockit extends and evolves this foundation into a unified suite for Docker Compose operations, incremental backups, and deployment safety.

## License

MIT — see [`LICENSE`](LICENSE).
