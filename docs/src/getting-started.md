# Getting Started

This guide walks you through the daily workflow with Flovo Dockit in a multi-service repository.

---

## 1. The Repository Structure

Dockit expects a Git repository where services reside in subdirectories containing a `project.json` file:

```text
my-repo/
├── .dockitignore           # Optional: directories discovery should skip
├── .env / .env.local       # Global environment variables
├── .secrets/               # Restrictively chmod'ed secret values
├── apps/
│   ├── traefik/            # Reverse proxy service
│   │   ├── docker-compose.yml
│   │   └── project.json
│   └── navidrome/          # Music server depending on traefik
│       ├── docker-compose.yml
│       ├── docker-compose.local.yml # Environment overlay
│       └── project.json
└── libs/
    └── networks/           # Graph-only project providing shared network
        └── project.json
```

### An Example `project.json`

```json
{
  "name": "navidrome",
  "implicitDependencies": ["networks", "traefik"],
  "targets": {
    "create-volumes": {
      "options": {
        "volumes": ["navidrome-music", "navidrome-data"]
      }
    },
    "create-variables": {
      "options": {
        "variables": [
          { "name": "MUSIC_DIR", "description": "Host path to music library" },
          { "name": "ADMIN_PASSWORD", "type": "password", "secret": true }
        ]
      }
    }
  }
}
```

---

## 2. Common Workflow Steps

### Step 1: Discover Projects
List every discovered project and its declared dependencies:

```bash
dockit list
```

Query the dependency chain for a specific service:

```bash
# View dependencies (what navidrome needs)
dockit deps navidrome

# View dependents (what depends on traefik)
dockit deps traefik --reverse
```

---

### Step 2: Provision Dependencies & Secrets
Before launching a service, run `configure` to ensure required variables, secrets, volumes, and networks exist for the project and all its upstream dependencies:

```bash
# Interactive prompt for missing variables and passwords
dockit configure navidrome

# Non-interactive mode using defaults and existing environment variables (ideal for CI)
dockit configure navidrome --use-defaults
```

---

### Step 3: Preview Invocations (`--dry-run`)
Dockit provides exact, reproducible execution previews without contacting Docker:

```bash
dockit up navidrome --dry-run
```

*Output:*
```text
docker compose --file libs/networks/docker-compose.yml up --yes --remove-orphans --detach
docker compose --file apps/traefik/docker-compose.yml up --yes --remove-orphans --detach
docker compose --file apps/navidrome/docker-compose.yml --file apps/navidrome/docker-compose.local.yml up --yes --remove-orphans --detach
```

---

### Step 4: Start Services
Start the service and its full dependency cascade in topological order:

```bash
dockit up navidrome
```

To block until each container's health checks report healthy:

```bash
dockit up navidrome --wait
```

To start only `navidrome` without its dependencies:

```bash
dockit up navidrome --no-deps
```

---

### Step 5: Monitor & Logs

Check the status of running projects across the repository:

```bash
dockit ps
```

Stream logs from a specific project:

```bash
dockit logs navidrome --follow
```

---

### Step 6: Backup and Restore

Create atomic archives of a project's declared volumes:

```bash
dockit backup-volumes navidrome
```

Restore volumes from existing backup archives:

```bash
dockit restore-volumes navidrome --force
```

---

### Step 7: Teardown

Stop a project:

```bash
dockit down navidrome
```

Or tear down all running projects across the entire repository in reverse-topological order:

```bash
dockit all down
```
