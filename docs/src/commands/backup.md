# Backup Operations

Dockit provides commands to safeguard application state, configuration, and data volumes.

---

## 1. Volume Backup (`dockit backup-volumes`)

Archives each of a project's declared volumes to compressed bzip2 archives:

```bash
dockit backup-volumes <project> [--target-path <dir>]
```

### Options

| Flag | Description | Default |
|---|---|---|
| `--target-path <dir>` | Directory where archive files are written | `.backups` at repo root |

### How It Works

1. **Own Volumes Only**: Unlike lifecycle commands, backup operations deliberately **do not cascade** into dependencies. You back up the specific project named.
2. **Atomic Write Guarantee**: Each archive is written to `<volume>.tar.bz2.partial` first. Only upon complete, error-free completion of the backup child is it renamed to `<volume>.tar.bz2`. A failed backup never clobbers a previously good backup.
3. **No Shell Invocations**: Archives are streamed through child process stdio connected directly to file descriptors at the process seam.

```bash
# Example: Backup navidrome volumes to default .backups/
dockit backup-volumes navidrome

# Example: Backup to an external mount or backup volume
dockit backup-volumes navidrome --target-path /mnt/nas/dockit-backups
```

---

## 2. Next-Gen Incremental Backups (Release 2)

As part of the [Dockit Release Roadmap](../roadmap.md), Dockit is evolving from full volume archives to deduplicated incremental snapshots:

### The `dockit backup` Command
```bash
dockit backup <project>
```

- **Incremental Content-Addressed Snapshots**: Integrated with [Restic](https://restic.readthedocs.io/), storing only changed blocks across runs and reusing identical data.
- **Database-Native Exports**: Dumps live PostgreSQL, MySQL, and SQLite databases in consistent states rather than copying raw live data directories.
- **Off-Site Replication**: Direct support for S3, Cloudflare R2, Backblaze B2, and MinIO targets.
- **Safe Retention Policies**: Enforcing 7 daily, 4 weekly, and 3 monthly recovery points, with automated guards ensuring old snapshots are never pruned until newer backups pass integrity verification.
