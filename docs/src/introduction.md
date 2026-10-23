# Introduction to Flovo Dockit

<p align="center">
  <img src="assets/icon.png" width="128" alt="Flovo Dockit Logo" />
</p>

<p align="center">
  <strong>Your containers, under control.</strong><br>
  <em>Docker operations, backups, recovery, and deployment safety in one CLI.</em>
</p>

---

## What is Dockit?

**Flovo Dockit** (formerly *tug*) is a tiny, opinionated CLI tool designed to orchestrate and safeguard Docker Compose applications across multi-service repositories.

Unlike heavy orchestrators that require daemon agents, background workers, or complex web services, Dockit operates as a single static binary that interacts directly with plain `docker compose`.

### Core Design Principles

1. **Plain Compose Stays Runnable**: Dockit never locks you into a proprietary orchestration runtime. Your `docker-compose.yml` and per-environment overlays stay 100% standard and directly runnable with `docker compose`.
2. **Process-Spawn Seam**: Every external Docker invocation is treated as pure immutable data (`Invocation`) dispatched through a testable execution seam (`Runner`). This guarantees reproducible `--dry-run` previews and parallel-safe testing without live Docker daemons.
3. **Safety First**: Dockit enforces dependency order for starting services, guarantees transaction-like volume archiving, and validates environment configs before touching production containers.
4. **Single Static Binary**: Built in Rust with minimal dependencies, compiling into a static Linux binary with no runtime dependencies beyond `docker` on your `$PATH`.

---

## Feature Overview

| Capability | Description |
|---|---|
| **Dependency-Ordered Lifecycle** | `up` walks transitive dependency cascades topologically; `down`/`restart` operate safely; `all <verb>` batches across running services. |
| **Per-Environment Overlays** | Automatically detects environment (`ci`, `$ENVIRONMENT_NAME`, or `local`) and merges `docker-compose.<env>.yml` onto base compose files. |
| **Unified Provisioning** | `configure` cascades through dependencies, collects required variables and secrets, and provisions declared volumes and networks. |
| **Backup & Recovery** | Volume snapshotting with atomic temporary files, and a planned transition to deduplicated incremental backups via Restic and database-native dumps. |
| **Scaffolding** | `new <name>` initializes well-formed services adhering to repository conventions with consistent casing normalization. |
| **Shell Completions** | Dynamic, repository-aware TAB completions for `bash`, `zsh`, and `fish`. |
