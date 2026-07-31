//! The provisioning commands: `create-volumes`,
//! `create-networks`, and the `configure` orchestrator. Docker state is
//! queried machine-readably and mutated through the injected [`Runner`],
//! one spec at a time, stopping at the first failure.

use std::collections::{BTreeMap, HashSet};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::env;
use crate::process::{Invocation, Runner};
use crate::repo::{Project, Repo};
use crate::vars::{self, Console, HostFacts};

/// `configure <project>`: the onboarding orchestrator — run
/// `create-variables` (passing `--use-defaults` through), `create-volumes`,
/// then `create-networks`, stopping at the first failure.
///
/// Returns the CLI exit code: `0` on success, the first failing child's
/// exit code otherwise.
///
/// # Errors
///
/// Fails on an unknown project name or when any of the three steps fails
/// before reaching docker (unreadable files, console errors, spawn failures).
pub fn configure(
    repo: &Repo,
    process_env: &BTreeMap<String, String>,
    name: &str,
    use_defaults: bool,
    host: &HostFacts,
    console: &mut dyn Console,
    runner: &mut dyn Runner,
) -> Result<i32> {
    named_project(repo, name)?;
    vars::create_variables(repo, process_env, name, use_defaults, host, console)?;
    let code = create_volumes(repo, process_env, name, false, runner)?;
    if code != 0 {
        return Ok(code);
    }
    create_networks(repo, process_env, name, runner)
}

/// `create-volumes <project>`: create the cascade's missing
/// volumes, chowning those with a `userId`.
///
/// Returns the CLI exit code: `0` on success, the first failing child's
/// exit code otherwise.
///
/// # Errors
///
/// Fails on an unknown project name, unreadable dotenv files, an unparsable
/// `docker volume ls` listing, or when a child cannot be spawned at all.
pub fn create_volumes(
    repo: &Repo,
    process_env: &BTreeMap<String, String>,
    name: &str,
    force: bool,
    runner: &mut dyn Runner,
) -> Result<i32> {
    let project = named_project(repo, name)?;
    let specs = repo.cascade(name, |project| project.volumes.as_slice())?;
    let merged = env::merged_env(process_env, repo.root(), &project.dir)?;
    let root = repo.root().to_string_lossy().into_owned();

    let query = Invocation::new(["docker", "volume", "ls", "--format", "json"], &root)
        .with_env(merged.clone())
        .with_captured_stdout();
    let outcome = runner.run(&query)?;
    if outcome.exit_code != 0 {
        return Ok(outcome.exit_code);
    }
    let existing = names_in_listing(outcome.stdout.as_deref().unwrap_or_default())
        .context("failed to parse the docker volume ls listing")?;

    for spec in specs {
        if existing.contains(&spec.name) {
            if !force {
                continue;
            }
            // `--force`: remove the existing volume, recreate it.
            let remove = Invocation::new(["docker", "volume", "rm", &spec.name], &root)
                .with_env(merged.clone());
            let outcome = runner.run(&remove)?;
            if outcome.exit_code != 0 {
                return Ok(outcome.exit_code);
            }
        }
        let create = Invocation::new(["docker", "volume", "create", &spec.name], &root)
            .with_env(merged.clone());
        let outcome = runner.run(&create)?;
        if outcome.exit_code != 0 {
            return Ok(outcome.exit_code);
        }
        if let Some(uid) = spec.user_id {
            let image = spec.image.as_deref().unwrap_or("busybox");
            let mount_path = spec.mount_path.as_deref().unwrap_or("/volume");
            let chown = Invocation::new(
                [
                    "docker",
                    "run",
                    "--rm",
                    "-v",
                    &format!("{}:{mount_path}", spec.name),
                    "--entrypoint",
                    "/bin/sh",
                    image,
                    "-c",
                    &format!("chown -R {uid}:{uid} {mount_path}"),
                ],
                &root,
            )
            .with_env(merged.clone());
            let outcome = runner.run(&chown)?;
            if outcome.exit_code != 0 {
                return Ok(outcome.exit_code);
            }
        }
    }
    Ok(0)
}

/// `create-networks <project>`: create the cascade's missing
/// networks with their subnet and IP range.
///
/// Returns the CLI exit code: `0` on success, the first failing child's
/// exit code otherwise.
///
/// # Errors
///
/// Fails on an unknown project name, unreadable dotenv files, an unparsable
/// `docker network ls` listing, or when a child cannot be spawned at all.
pub fn create_networks(
    repo: &Repo,
    process_env: &BTreeMap<String, String>,
    name: &str,
    runner: &mut dyn Runner,
) -> Result<i32> {
    let project = named_project(repo, name)?;
    let specs = repo.cascade(name, |project| project.networks.as_slice())?;
    let merged = env::merged_env(process_env, repo.root(), &project.dir)?;
    let root = repo.root().to_string_lossy().into_owned();

    let query = Invocation::new(["docker", "network", "ls", "--format", "json"], &root)
        .with_env(merged.clone())
        .with_captured_stdout();
    let outcome = runner.run(&query)?;
    if outcome.exit_code != 0 {
        return Ok(outcome.exit_code);
    }
    let existing = names_in_listing(outcome.stdout.as_deref().unwrap_or_default())
        .context("failed to parse the docker network ls listing")?;

    for spec in specs {
        if existing.contains(&spec.name) {
            continue;
        }
        let create = Invocation::new(
            [
                "docker",
                "network",
                "create",
                "--subnet",
                &spec.subnet,
                "--ip-range",
                &spec.ip_range,
                &spec.name,
            ],
            &root,
        )
        .with_env(merged.clone());
        let outcome = runner.run(&create)?;
        if outcome.exit_code != 0 {
            return Ok(outcome.exit_code);
        }
    }
    Ok(0)
}

/// Look up the project a provisioning command names: an unknown name errors
/// listing the valid names. Graph-only projects are fine here —
/// provisioning needs no compose file.
pub(crate) fn named_project<'repo>(repo: &'repo Repo, name: &str) -> Result<&'repo Project> {
    let Some(project) = repo.project(name) else {
        let valid: Vec<&str> = repo.projects().map(|p| p.name.as_str()).collect();
        bail!(
            "unknown project {name:?}; valid projects are: {}",
            valid.join(", ")
        );
    };
    Ok(project)
}

/// One line of a `docker … ls --format json` listing: only the name is read.
#[derive(Debug, Deserialize)]
struct ListingRow {
    #[serde(rename = "Name")]
    name: String,
}

/// The names in a `docker … ls --format json` listing — one JSON object per
/// line — machine-readable, never scraped from the table.
fn names_in_listing(listing: &str) -> Result<HashSet<String>> {
    listing
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let row: ListingRow = serde_json::from_str(line)
                .with_context(|| format!("unparsable listing line: {line}"))?;
            Ok(row.name)
        })
        .collect()
}
