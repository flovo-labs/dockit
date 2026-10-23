# Lifecycle Commands

Dockit manages the operational lifecycle of your Docker Compose projects.

---

## 1. `dockit up <project>`

Starts a project and its entire transitive dependency cascade in topological order (dependencies first):

```bash
dockit up <project> [options]
```

### Options

| Flag | Description |
|---|---|
| `--no-deps` | Start only the named project, skipping the dependency cascade. *(Note: this cuts Dockit's graph, not compose's `depends_on`).* |
| `--force` | Recreate containers even if configuration is unchanged (`compose --force-recreate`). |
| `--no-detach` | Stay attached to container output instead of passing `--detach`. |
| `--wait` | Block until every container reports healthy before proceeding to the next dependent. |
| `--wait-timeout <secs>` | Maximum seconds to wait for container health (default: 120s). Requires `--wait`. |
| `--dry-run` | Print the exact `docker compose` commands without executing them. |

```bash
# Start bonob and wait up to 60s for health checks on each dependency
dockit up bonob --wait --wait-timeout 60
```

---

## 2. `dockit down <project>` & `dockit restart <project>`

- **`dockit down <project>`**: Stops and tears down containers for the named project (does not cascade to dependencies).
- **`dockit restart <project>`**: Bounces the service (a `down` followed by an `up`, no cascade either way).

```bash
dockit down navidrome
dockit restart navidrome
```

---

## 3. `dockit pull <project>` & `dockit logs <project>`

- **`dockit pull <project>`**: Pulls updated images for a project.
- **`dockit logs <project> [--follow]`**: Streams compose logs for a project.

```bash
dockit pull traefik
dockit logs traefik --follow
```

---

## 4. `dockit all <verb>`

Runs a lifecycle action (`down`, `restart`, `pull`) across **every currently running project**:

```bash
dockit all down
dockit all restart
dockit all pull
```

- **Order Aware**: `all down` executes in reverse topological order (dependents first); `all restart` and `all pull` execute dependencies first.
- **Best-Effort Execution**: Continues through failures, collects errors, and prints a summary.

---

## 5. Inspection Commands

- **`dockit list`**: Lists all discovered projects and their direct dependencies.
- **`dockit ps`**: Reports all currently running compose projects.
- **`dockit deps <project>`**: Inspects dependency relationships:
  - `--reverse`: Show dependents (what depends on this project).
  - `--transitive`: Show full dependency tree.
  - `--json`: Machine-readable JSON output for CI/CD pipelines.

---

## 6. Project Scaffolding (`dockit new <name>`)

Stamps out a fresh app structure under `apps/<name>` from built-in templates:

```bash
dockit new my-app
```

Automatically generates:
- `docker-compose.yml`
- `docker-compose.local.yml`
- `project.json`
- `.env`
with normalized casings (`my-app`, `MY_APP`, `MyApp`).
