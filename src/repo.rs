//! The repository model: root detection, project discovery, and
//! `project.json` parsing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// The discovered repository: its root and every project in it.
#[derive(Debug)]
pub struct Repo {
    root: PathBuf,
    projects: BTreeMap<String, Project>,
}

/// One project — a directory holding a `project.json`.
#[derive(Debug)]
pub struct Project {
    /// The unique repo-wide name; the CLI argument.
    pub name: String,
    /// The project directory, relative to the repo root.
    pub dir: PathBuf,
    /// Direct dependencies by project name.
    pub implicit_dependencies: Vec<String>,
    /// The `create-variables` target's specs.
    pub variables: Vec<VariableSpec>,
    /// The `create-volumes` target's specs.
    pub volumes: Vec<VolumeSpec>,
    /// The `create-networks` target's specs.
    pub networks: Vec<NetworkSpec>,
    /// A `docker-compose.yml` sits next to `project.json`; without one the
    /// project is a graph-only node — no `up`/`down`/`pull`.
    pub has_compose_file: bool,
}

impl Project {
    /// A retired project: its directory sits under the conventional
    /// `archive/` tree. Presentation-only — directory names are kept out of
    /// the behavioral contract, so this only affects how listings render.
    #[must_use]
    pub fn is_archived(&self) -> bool {
        self.dir.starts_with("archive")
    }
}

/// One variable spec of the `create-variables` target.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableSpec {
    pub name: String,
    /// Shown as the prompt text; required.
    pub description: String,
    /// May hold `${VAR}` references, expanded against the environment later.
    pub default_value: Option<String>,
    /// The special prompting behavior; `None` is the plain text prompt.
    #[serde(rename = "type")]
    pub kind: Option<VariableKind>,
    /// Optional = an empty answer is allowed.
    #[serde(default)]
    pub optional: bool,
    /// Secrets go to `.secrets/<NAME>`, never `.env.local`.
    #[serde(default)]
    pub is_secret: bool,
}

/// The five special variable types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VariableKind {
    Password,
    Generated,
    Boolean,
    DockerGroup,
    Hostname,
}

/// One volume spec of the `create-volumes` target, normalized
/// from either the object form or the plain-string (name-only) form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeSpec {
    pub name: String,
    /// Chown the volume to this uid after creation.
    pub user_id: Option<u32>,
    /// Image used for the chown; defaults to `busybox` at use time.
    pub image: Option<String>,
    /// Mount path used for the chown; defaults to `/volume`.
    pub mount_path: Option<String>,
}

/// One network spec of the `create-networks` target.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSpec {
    pub name: String,
    pub subnet: String,
    pub ip_range: String,
}

/// The parts of `project.json` the CLI reads; everything else is ignored.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectFile {
    name: String,
    #[serde(default)]
    implicit_dependencies: Vec<String>,
    #[serde(default)]
    targets: Targets,
}

// Field names mirror the on-disk target names, prefix and all.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Default, Deserialize)]
struct Targets {
    #[serde(rename = "create-variables", default)]
    create_variables: Target<VariableOptions>,
    #[serde(rename = "create-volumes", default)]
    create_volumes: Target<VolumeOptions>,
    #[serde(rename = "create-networks", default)]
    create_networks: Target<NetworkOptions>,
}

#[derive(Debug, Default, Deserialize)]
struct Target<T: Default> {
    #[serde(default)]
    options: T,
}

#[derive(Debug, Default, Deserialize)]
struct VariableOptions {
    #[serde(default)]
    variables: Vec<VariableSpec>,
}

#[derive(Debug, Default, Deserialize)]
struct VolumeOptions {
    #[serde(default)]
    volumes: Vec<VolumeSpecFile>,
}

#[derive(Debug, Default, Deserialize)]
struct NetworkOptions {
    #[serde(default)]
    networks: Vec<NetworkSpec>,
}

/// The two on-disk shapes of a volume spec.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum VolumeSpecFile {
    Name(String),
    Full {
        name: String,
        #[serde(rename = "userId")]
        user_id: Option<u32>,
        image: Option<String>,
        #[serde(rename = "mountPath")]
        mount_path: Option<String>,
    },
}

impl From<VolumeSpecFile> for VolumeSpec {
    fn from(file: VolumeSpecFile) -> Self {
        match file {
            VolumeSpecFile::Name(name) => Self {
                name,
                user_id: None,
                image: None,
                mount_path: None,
            },
            VolumeSpecFile::Full {
                name,
                user_id,
                image,
                mount_path,
            } => Self {
                name,
                user_id,
                image,
                mount_path,
            },
        }
    }
}

impl Repo {
    /// Find the repo root above `cwd` and discover every project in it.
    ///
    /// # Errors
    ///
    /// Fails when `cwd` is not inside a git repository, a `project.json`
    /// cannot be read or parsed, or the repository is invalid
    /// (duplicate names, unknown dependencies).
    pub fn discover(cwd: &Path) -> Result<Self> {
        let root = find_repo_root(cwd)?;
        let mut projects = BTreeMap::new();
        scan(&root, &root, &mut projects)?;
        for project in projects.values() {
            for dependency in &project.implicit_dependencies {
                if !projects.contains_key(dependency) {
                    bail!(
                        "project {:?} depends on unknown project {:?}",
                        project.name,
                        dependency,
                    );
                }
            }
        }
        Ok(Self { root, projects })
    }

    /// The repository root (the directory containing `.git`).
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Every project, in name order (deterministic across runs).
    pub fn projects(&self) -> impl Iterator<Item = &Project> {
        self.projects.values()
    }

    /// Look up one project by name.
    #[must_use]
    pub fn project(&self, name: &str) -> Option<&Project> {
        self.projects.get(name)
    }

    /// Look up one project by name, or fail listing the valid names — the
    /// shared "unknown project" guard of the graph query (`deps`) and the
    /// lifecycle commands, so a typo fails loudly with the same helpful message.
    ///
    /// # Errors
    ///
    /// Fails when `name` is not a known project.
    pub fn project_or_bail(&self, name: &str) -> Result<&Project> {
        if let Some(project) = self.project(name) {
            return Ok(project);
        }
        let valid: Vec<&str> = self.projects().map(|p| p.name.as_str()).collect();
        bail!(
            "unknown project {name:?}; valid projects are: {}",
            valid.join(", ")
        );
    }
}

/// Recursively collect projects, skipping `node_modules/` and dot-prefixed
/// directories.
fn scan(root: &Path, dir: &Path, projects: &mut BTreeMap<String, Project>) -> Result<()> {
    let manifest = dir.join("project.json");
    if manifest.is_file() {
        let text = std::fs::read_to_string(&manifest)
            .with_context(|| format!("failed to read {}", manifest.display()))?;
        let file: ProjectFile = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", manifest.display()))?;
        let dir_relative = dir.strip_prefix(root).expect("scan stays under root");
        if let Some(existing) = projects.get(&file.name) {
            bail!(
                "duplicate project name {:?}: declared in both {} and {}",
                file.name,
                existing.dir.join("project.json").display(),
                dir_relative.join("project.json").display(),
            );
        }
        projects.insert(
            file.name.clone(),
            Project {
                name: file.name,
                dir: dir_relative.to_path_buf(),
                implicit_dependencies: file.implicit_dependencies,
                variables: file.targets.create_variables.options.variables,
                volumes: file
                    .targets
                    .create_volumes
                    .options
                    .volumes
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                networks: file.targets.create_networks.options.networks,
                has_compose_file: dir.join("docker-compose.yml").is_file(),
            },
        );
    }
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "node_modules" || name.starts_with('.') {
            continue;
        }
        scan(root, &entry.path(), projects)?;
    }
    Ok(())
}

/// Walk up from `cwd` to the first directory containing `.git`.
///
/// # Errors
///
/// Fails when no ancestor of `cwd` contains a `.git` entry.
pub fn find_repo_root(cwd: &Path) -> Result<PathBuf> {
    for dir in cwd.ancestors() {
        if dir.join(".git").exists() {
            return Ok(dir.to_path_buf());
        }
    }
    bail!(
        "not inside a git repository: no .git found in {} or any parent",
        cwd.display()
    );
}
