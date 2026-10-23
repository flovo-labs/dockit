# Provisioning Commands

Dockit coordinates the provisioning of variables, secrets, Docker volumes, and networks across your dependency graph.

---

## 1. `dockit configure <project>`

The main provisioning orchestrator. Runs through the three provisioning phases in order for the named project and all its transitive dependencies:

```bash
dockit configure <project> [--use-defaults]
```

Execution steps:
1. Prompts for and persists missing environment variables and secrets (`create-variables`).
2. Creates missing Docker volumes (`create-volumes`).
3. Creates missing Docker networks (`create-networks`).

### Non-Interactive CI Mode (`--use-defaults`)
When passed `--use-defaults`, Dockit never halts to prompt on stdin:
- Fallback to existing environment variables.
- Auto-generates random passwords for `password` and `generated` types.
- Accepts suggested host facts (`hostname`, `docker-group`).
- Leaves empty values for optional variables, or issues warnings for missing required variables.

```bash
dockit configure myapp --use-defaults
```

---

## 2. Individual Provisioning Commands

If you need granular control, each provisioning phase can be triggered separately:

### `dockit create-variables <project>`
Prompts for undeclared variables across the dependency cascade and writes non-secrets to `.env.local` and secrets to `.secrets/<name>`:

```bash
dockit create-variables myapp
```

### `dockit create-volumes <project>`
Creates declared Docker volumes for the cascade if they do not already exist:

```bash
dockit create-volumes myapp

# Re-create volumes even if they already exist
dockit create-volumes myapp --force
```

### `dockit create-networks <project>`
Creates declared Docker bridge networks for the cascade:

```bash
dockit create-networks myapp
```
