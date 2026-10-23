# Configuration & Layout

Dockit establishes clear conventions for project manifests, environment overlays, variable prompting, and directory exclusion.

---

## 1. Project Manifest (`project.json`)

Every project directory is identified by a `project.json` at its root.

```json
{
  "name": "my-service",
  "implicitDependencies": ["database", "networks"],
  "targets": {
    "create-variables": {
      "options": {
        "variables": [
          { "name": "PORT", "default": "8080" },
          { "name": "DB_PASSWORD", "type": "password", "secret": true },
          { "name": "SECRET_KEY", "type": "generated" },
          { "name": "ENABLE_METRICS", "type": "boolean", "default": "false" },
          { "name": "DOCKER_GID", "type": "docker-group" },
          { "name": "HOST_NAME", "type": "hostname" }
        ]
      }
    },
    "create-volumes": {
      "options": {
        "volumes": [
          { "name": "service-data", "chown": "1000:1000" }
        ]
      }
    },
    "create-networks": {
      "options": {
        "networks": ["shared-proxy"]
      }
    }
  }
}
```

### The Six Variable Types

| Type | Prompt Behavior | Persistence |
|---|---|---|
| `text` (default) | Standard line input, optional default value expansion | Written to root `.env.local` |
| `password` | Masked terminal input without echoing to screen | Written to `.secrets/<name>` (mode `0600`) |
| `generated` | Auto-generates cryptographically secure 32-byte secret | Written to `.secrets/<name>` |
| `boolean` | `[y/N]` or `[Y/n]` prompt validating truthy/falsy values | Written to root `.env.local` as `true`/`false` |
| `docker-group` | Automatically detects the host's `/etc/group` `docker` GID | Written to root `.env.local` |
| `hostname` | Inspects system hostname and offers it as default | Written to root `.env.local` |

---

## 2. Environment Resolution & Overlays

Dockit resolves an **Environment Name** once per execution:
1. `ci` if the `$CI` environment variable is set.
2. Value of `$ENVIRONMENT_NAME` if defined.
3. Defaults to `local`.

### Compose File Layering
For any lifecycle command, Dockit checks for an overlay file corresponding to the active environment:
- Always loads: `docker-compose.yml`
- Appends if present: `docker-compose.<env>.yml` (e.g. `docker-compose.local.yml`, `docker-compose.ci.yml`)

Docker Compose naturally merges the overlay on top of the base file.

---

## 3. Environment Variable Precedence

When spawning `docker compose`, Dockit constructs a single unified environment according to strict precedence (highest wins):

1. **Host Environment**: Variables already set in the executing shell.
2. **Project Dotenv Local**: `<projectDir>/.env.local`
3. **Project Dotenv**: `<projectDir>/.env`
4. **Root Dotenv Local**: `<repoRoot>/.env.local`
5. **Root Dotenv**: `<repoRoot>/.env`

---

## 4. Directory Exclusions (`.dockitignore`)

To prevent Dockit from scanning deep vendor trees or retired services, place a `.dockitignore` file at your repository root:

```gitignore
# Ignore third-party vendored packages
node_modules
vendor

# Ignore specific path relative to repo root
/archive
apps/legacy-app

# Comments open with #
#archive
```

> **Compatibility Note**: If `.dockitignore` is not present, Dockit automatically falls back to `.tugignore` for seamless backwards compatibility.
