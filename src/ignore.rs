//! The `.tugignore` pattern list: the repo-owned half of what discovery
//! skips.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The file, at the repo root and nowhere else.
const FILE_NAME: &str = ".tugignore";

/// The parsed `.tugignore`: the patterns naming directories discovery never
/// descends into.
#[derive(Debug, Default)]
pub struct IgnoreList {
    patterns: Vec<Pattern>,
}

impl IgnoreList {
    /// Read the repository's `.tugignore`. An absent file is the empty list —
    /// the overwhelmingly common case.
    ///
    /// # Errors
    ///
    /// Fails when the file exists but cannot be read: falling back to "ignore
    /// nothing" would silently widen every command to trees the repository
    /// asked tug to stay out of.
    pub fn read(root: &Path) -> Result<Self> {
        let path = root.join(FILE_NAME);
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Self::parse(&text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
        }
    }

    /// Parse the contents of a `.tugignore`, one pattern per line. Blank
    /// lines and lines opening with `#` declare nothing. A `#` further along
    /// a line is an ordinary character, as in `.gitignore` — there are no
    /// inline comments, so a pattern is always the whole line.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        Self {
            patterns: text
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(Pattern::new)
                .collect(),
        }
    }

    /// Whether `relative` — a directory path relative to the repo root — is
    /// ignored.
    #[must_use]
    pub fn ignores(&self, relative: &Path) -> bool {
        self.patterns
            .iter()
            .any(|pattern| pattern.matches(relative))
    }
}

/// One `.tugignore` line: a directory name matched wherever it appears, or a
/// path matched in exactly one place.
#[derive(Debug)]
enum Pattern {
    /// A line without a slash — the name alone, at any depth.
    Name(String),
    /// A line with a slash — the path, relative to the repo root.
    Anchored(PathBuf),
}

impl Pattern {
    /// Read one trimmed, non-empty line. A leading `/` anchors the pattern
    /// without being part of the path it names, which is the only way to
    /// pin a root-level directory whose name also occurs deeper. A trailing
    /// one is dropped: only directories are ever matched, so it carries no
    /// information and must not push a bare name into the anchored branch.
    fn new(line: &str) -> Self {
        let line = line.strip_suffix('/').unwrap_or(line);
        if let Some(anchored) = line.strip_prefix('/') {
            return Self::Anchored(PathBuf::from(anchored));
        }
        if line.contains('/') {
            return Self::Anchored(PathBuf::from(line));
        }
        Self::Name(line.to_owned())
    }

    fn matches(&self, relative: &Path) -> bool {
        match self {
            Self::Name(name) => relative
                .file_name()
                .is_some_and(|candidate| candidate == name.as_str()),
            Self::Anchored(path) => relative == path,
        }
    }
}
