# Safety & Preflight Checks

Before managing production containers and stateful data, Dockit emphasizes preflight safety, configuration validation, and execution planning.

---

## 1. Dry Run Previews (`--dry-run`)

Dockit implements `--dry-run` natively across all lifecycle commands. Because all invocations are pure data structures passed through the process seam, `--dry-run` is guaranteed to match the exact command-line arguments, environment variables, and flags that a live run would execute.

```bash
# Preview service startup and dependency cascade
dockit up navidrome --dry-run

# Preview service restart
dockit restart navidrome --dry-run

# Preview tear down
dockit down navidrome --dry-run
```

---

## 2. Release 1 Safety Commands

In accordance with the [Dockit Release Roadmap](../roadmap.md), Release 1 introduces specialized safety commands:

### `dockit doctor`
Inspects host prerequisites, permissions, and dependencies before operations:

```bash
dockit doctor
```
- Verifies Docker daemon connectivity and Compose plugin version.
- Confirms current user permissions (`docker` group membership, read/write access to repo root).
- Verifies permissions on `.secrets/` directory (mode `0700` and file modes `0600`).
- Checks available disk headroom across Docker storage roots and backup targets.

---

### `dockit preflight <project>`
Validates configuration integrity and flags risks prior to deployment:

```bash
dockit preflight navidrome
```
- Validates Docker Compose YAML syntax and schema compliance.
- Checks for missing or unexpanded environment variables.
- Detects port conflicts with already-running containers or host listeners.
- Audits mounted file paths and bind mounts to prevent accidental volume overwrites.
- Flags insecure settings (e.g. `privileged: true`, dangerous host network sharing).

---

### `dockit plan <command>`
Outputs a detailed execution plan before applying changes:

```bash
dockit plan up navidrome --json
```
- Lists every container that will be started, recreated, or stopped.
- Formats dependency graph execution order clearly.
- Supports `--json` for automated verification in CI/CD pipelines.
