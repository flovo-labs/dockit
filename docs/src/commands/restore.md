# Restore & Recovery

Dockit ensures recovery is safe, predictable, and protected against accidental data loss.

---

## 1. Volume Restore (`dockit restore-volumes`)

Restores a project's declared volumes from compressed archives created by `backup-volumes`:

```bash
dockit restore-volumes <project> [--source-path <dir>] [--force]
```

### Options

| Flag | Description | Default |
|---|---|---|
| `--source-path <dir>` | Directory where archive files are read from | `.backups` at repo root |
| `-f, --force` | Overwrite volume contents if data already exists | `false` |

### Safety Rules

1. **All Archives Must Exist**: Before any container is spawned or any volume touched, Dockit verifies that every archive file needed for the project is present and readable. A missing archive aborts immediately with a clear refusal, preventing half-restored states.
2. **Overwrite Guard**: Unless `--force` is explicitly provided, existing non-empty volumes will refuse restoration to avoid clobbering live data.

```bash
# Example: Restore navidrome from default .backups/
dockit restore-volumes navidrome --force

# Example: Restore from external archive path
dockit restore-volumes navidrome --source-path /mnt/nas/dockit-backups --force
```

---

## 2. Next-Gen Verified Recovery (Release 2)

Future releases introduce comprehensive recovery orchestration:

```bash
# Preview recovery without applying changes
dockit restore myapp --snapshot latest --dry-run

# Restore with explicit confirmation
dockit restore myapp --snapshot latest

# Test restore in isolated test sandbox
dockit restore myapp --snapshot latest --test-sandbox
```

- **Restoration Previews (`--dry-run`)**: Shows exactly which database instances will be dropped/reloaded and which volume paths will be updated.
- **Isolated Sandbox Verification**: Spins up ephemeral containers on an isolated Docker network to verify that restored databases boot cleanly and pass health checks before touching production instances.
