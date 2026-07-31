//! The `new` command: casing normalization and scaffolding
//! `apps/<kebab>/` from the embedded templates.

use anyhow::{Context, Result, bail};

use crate::repo::Repo;

/// The templates, embedded verbatim; `«kebab»`, `«CONSTANT»`, and
/// `«Class»` are the placeholders, everything else (including `${…}`) is
/// literal output.
const TEMPLATES: [(&str, &str); 4] = [
    (
        "docker-compose.yml",
        include_str!("templates/docker-compose.yml"),
    ),
    (
        "docker-compose.local.yml",
        include_str!("templates/docker-compose.local.yml"),
    ),
    ("project.json", include_str!("templates/project.json")),
    (".env", include_str!("templates/.env")),
];

/// Scaffold `apps/<kebab>/` from the templates.
///
/// # Errors
///
/// Fails before writing anything when the name normalizes to no words, a
/// project with the kebab name already exists, or the target directory does;
/// and on I/O errors while writing.
pub fn new_project(repo: &Repo, name: &str) -> Result<()> {
    let casings = Casings::of(name)?;
    if let Some(existing) = repo.project(&casings.kebab) {
        bail!(
            "project {:?} already exists in {}",
            casings.kebab,
            existing.dir.display(),
        );
    }
    let dir = repo.root().join("apps").join(&casings.kebab);
    if dir.exists() {
        bail!("directory {} already exists", dir.display());
    }
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    for (file_name, template) in TEMPLATES {
        let path = dir.join(file_name);
        std::fs::write(&path, render(template, &casings))
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

fn render(template: &str, casings: &Casings) -> String {
    template
        .replace("«kebab»", &casings.kebab)
        .replace("«CONSTANT»", &casings.constant)
        .replace("«Class»", &casings.class)
}

/// The three casings of a project name.
#[derive(Debug, PartialEq, Eq)]
pub struct Casings {
    /// Lower-case, `-`-separated: `my-app`.
    pub kebab: String,
    /// Upper-case, `_`-separated: `MY_APP`.
    pub constant: String,
    /// Capitalized, concatenated: `MyApp`.
    pub class: String,
}

impl Casings {
    /// Normalize a raw name into the three casings. Word boundaries are `-`,
    /// `_`, whitespace, and lower→upper camelCase transitions.
    ///
    /// # Errors
    ///
    /// Fails when the input contains no words at all.
    pub fn of(input: &str) -> Result<Self> {
        let words = words(input);
        if words.is_empty() {
            bail!("invalid project name {input:?}: it contains no words");
        }
        Ok(Self {
            kebab: words.join("-").to_lowercase(),
            constant: words.join("_").to_uppercase(),
            class: words.iter().map(|word| capitalize(word)).collect(),
        })
    }
}

fn words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut previous: Option<char> = None;
    for c in input.chars() {
        if c == '-' || c == '_' || c.is_whitespace() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            previous = None;
            continue;
        }
        if previous.is_some_and(char::is_lowercase) && c.is_uppercase() && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        current.push(c);
        previous = Some(c);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
    }
}
