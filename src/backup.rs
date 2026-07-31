//! The backup commands: `backup-volumes` and
//! `restore-volumes`. Archives move through the child's stdio, wired
//! natively to files at the seam — no shell, no redirection.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::env;
use crate::process::{Invocation, Runner};
use crate::provision::named_project;
use crate::repo::Repo;

/// `backup-volumes <project>`: archive each of the project's
/// **own** volumes — no cascade — to `<target>/<name>.tar.bz2`. Each archive
/// is written to a temporary file and renamed on success, so a failed backup
/// never clobbers a previous one.
///
/// Returns the CLI exit code: `0` on success, the first failing child's
/// exit code otherwise.
///
/// # Errors
///
/// Fails on an unknown project name, unreadable dotenv files, an uncreatable
/// target directory, or when a child cannot be spawned at all.
pub fn backup_volumes(
    repo: &Repo,
    process_env: &BTreeMap<String, String>,
    name: &str,
    target_path: Option<&Path>,
    runner: &mut dyn Runner,
) -> Result<i32> {
    let project = named_project(repo, name)?;
    let merged = env::merged_env(process_env, repo.root(), &project.dir)?;
    let root = repo.root().to_string_lossy().into_owned();
    let target = resolve(repo, target_path);
    std::fs::create_dir_all(&target)
        .with_context(|| format!("failed to create the target directory {}", target.display()))?;

    for spec in &project.volumes {
        let archive = target.join(format!("{}.tar.bz2", spec.name));
        let partial = target.join(format!("{}.tar.bz2.partial", spec.name));
        let backup = Invocation::new(
            [
                "docker",
                "run",
                "--rm",
                "--log-driver",
                "none",
                "-v",
                &format!("{}:/volume", spec.name),
                "loomchild/volume-backup",
                "backup",
            ],
            &root,
        )
        .with_env(merged.clone())
        .with_stdout_to_file(&partial);
        let outcome = runner.run(&backup)?;
        if outcome.exit_code != 0 {
            let _ = std::fs::remove_file(&partial);
            return Ok(outcome.exit_code);
        }
        std::fs::rename(&partial, &archive)
            .with_context(|| format!("failed to move the archive into {}", archive.display()))?;
    }
    Ok(0)
}

/// `restore-volumes <project>`: feed each of the project's
/// **own** volumes — no cascade — its archive from `<source>/<name>.tar.bz2`.
/// Every archive must exist before anything is spawned.
///
/// Returns the CLI exit code: `0` on success, the first failing child's
/// exit code otherwise.
///
/// # Errors
///
/// Fails on an unknown project name, unreadable dotenv files, a missing
/// archive, or when a child cannot be spawned at all.
pub fn restore_volumes(
    repo: &Repo,
    process_env: &BTreeMap<String, String>,
    name: &str,
    source_path: Option<&Path>,
    force: bool,
    runner: &mut dyn Runner,
) -> Result<i32> {
    let project = named_project(repo, name)?;
    let merged = env::merged_env(process_env, repo.root(), &project.dir)?;
    let root = repo.root().to_string_lossy().into_owned();
    let source = resolve(repo, source_path);

    // Every archive must be present before anything spawns —
    // a half-restored project is worse than a clear refusal.
    let archives: Vec<PathBuf> = project
        .volumes
        .iter()
        .map(|spec| source.join(format!("{}.tar.bz2", spec.name)))
        .collect();
    let missing: Vec<String> = archives
        .iter()
        .filter(|archive| !archive.is_file())
        .map(|archive| archive.display().to_string())
        .collect();
    if !missing.is_empty() {
        anyhow::bail!("missing archives, nothing restored: {}", missing.join(", "));
    }

    for (spec, archive) in project.volumes.iter().zip(archives) {
        let mut argv: Vec<String> = [
            "docker",
            "run",
            "--rm",
            "-i",
            "-v",
            &format!("{}:/volume", spec.name),
            "loomchild/volume-backup",
            "restore",
        ]
        .map(String::from)
        .to_vec();
        if force {
            argv.push("-f".to_owned());
        }
        let restore = Invocation::new(argv, &root)
            .with_env(merged.clone())
            .with_stdin_from_file(archive);
        let outcome = runner.run(&restore)?;
        if outcome.exit_code != 0 {
            return Ok(outcome.exit_code);
        }
    }
    Ok(0)
}

/// The backup directory a command works against: default
/// `.backups`; a relative path resolves against the repo root, an absolute
/// path is used as-is.
fn resolve(repo: &Repo, path: Option<&Path>) -> PathBuf {
    let path = path.unwrap_or(Path::new(".backups"));
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.root().join(path)
    }
}
