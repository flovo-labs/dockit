//! The dependency graph: transitive closure of
//! `implicitDependencies`, cycle detection, and deterministic topological
//! ordering.

use std::collections::HashSet;

use anyhow::{Result, bail};

use crate::repo::{NetworkSpec, Project, Repo, VariableSpec, VolumeSpec};

/// Which way a `deps` query walks the graph.
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    /// Dependencies: the edges already in `implicitDependencies`.
    Forward,
    /// Dependents: the inverse edges — who depends on the project.
    Reverse,
}

impl Direction {
    /// The stable label echoed in `--json` output.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Reverse => "reverse",
        }
    }
}

/// How far a `deps` query reaches from the queried project.
#[derive(Debug, Clone, Copy)]
pub enum Depth {
    /// Immediate neighbors only.
    Direct,
    /// The full transitive closure in the chosen direction.
    Transitive,
}

/// A spec that can cascade: its name is the deduplication key.
pub trait Named {
    fn spec_name(&self) -> &str;
}

impl Named for VariableSpec {
    fn spec_name(&self) -> &str {
        &self.name
    }
}

impl Named for VolumeSpec {
    fn spec_name(&self) -> &str {
        &self.name
    }
}

impl Named for NetworkSpec {
    fn spec_name(&self) -> &str {
        &self.name
    }
}

impl Repo {
    /// The effective order for `name`: its transitive dependencies first,
    /// topologically sorted, then the project itself. Ties are broken by
    /// depth-first traversal in declaration order; duplicates are visited
    /// once, first visit wins.
    ///
    /// # Errors
    ///
    /// Fails when `name` is not a known project.
    pub fn dependency_order(&self, name: &str) -> Result<Vec<&Project>> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        visit(self, name, &mut visited, &mut stack, &mut order)?;
        Ok(order)
    }

    /// Every project in one deterministic topological order, dependencies
    /// first — the whole graph, not one project's closure. Used
    /// to order `all <verb>` across the running set. Projects are
    /// seeded in name order, so ties break the same way across runs.
    ///
    /// # Errors
    ///
    /// Fails when the graph contains a cycle, naming the projects involved.
    pub fn topological_order(&self) -> Result<Vec<&Project>> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        let names: Vec<&str> = self
            .projects()
            .map(|project| project.name.as_str())
            .collect();
        for name in names {
            visit(self, name, &mut visited, &mut stack, &mut order)?;
        }
        Ok(order)
    }

    /// The cascade collection: the specs of all transitive
    /// dependencies in topological order, then the named project's own,
    /// deduplicated by spec name — first occurrence wins.
    ///
    /// # Errors
    ///
    /// Fails when `name` is not a known project.
    pub fn cascade<'repo, T: Named>(
        &'repo self,
        name: &str,
        specs: impl Fn(&'repo Project) -> &'repo [T],
    ) -> Result<Vec<&'repo T>> {
        let mut seen = HashSet::new();
        let mut collected = Vec::new();
        for project in self.dependency_order(name)? {
            for spec in specs(project) {
                if seen.insert(spec.spec_name()) {
                    collected.push(spec);
                }
            }
        }
        Ok(collected)
    }

    /// The `deps <project>` query: the names of `name`'s dependencies or
    /// dependents (per `direction`), direct or the full transitive closure
    /// (per `depth`). The queried project is excluded from its own result;
    /// the names are deduplicated and sorted lexicographically.
    ///
    /// Transitive traversal is guarded by a visited set, so it terminates on
    /// any cycle even though the graph is expected acyclic.
    ///
    /// # Errors
    ///
    /// Fails when `name` is not a known project, listing the valid names.
    pub fn related(&self, name: &str, direction: Direction, depth: Depth) -> Result<Vec<&str>> {
        let start = self.project_or_bail(name)?.name.as_str();

        let mut result: HashSet<&str> = HashSet::new();
        match depth {
            Depth::Direct => {
                for neighbor in self.neighbors(start, direction) {
                    result.insert(neighbor);
                }
            }
            Depth::Transitive => {
                let mut seen: HashSet<&str> = HashSet::from([start]);
                let mut stack = vec![start];
                while let Some(current) = stack.pop() {
                    for neighbor in self.neighbors(current, direction) {
                        if seen.insert(neighbor) {
                            result.insert(neighbor);
                            stack.push(neighbor);
                        }
                    }
                }
            }
        }
        // A self-edge (or a cycle back through it) would otherwise re-admit the
        // queried project; it is always excluded from its own result set.
        result.remove(start);

        let mut names: Vec<&str> = result.into_iter().collect();
        names.sort_unstable();
        Ok(names)
    }

    /// The immediate neighbors of `name` in the chosen direction: its declared
    /// dependencies (forward) or the projects that declare it as one of theirs
    /// (reverse). `name` is assumed to be a known project.
    fn neighbors(&self, name: &str, direction: Direction) -> Vec<&str> {
        match direction {
            Direction::Forward => self.project(name).map_or_else(Vec::new, |project| {
                project
                    .implicit_dependencies
                    .iter()
                    .map(String::as_str)
                    .collect()
            }),
            Direction::Reverse => self
                .projects()
                .filter(|project| {
                    project
                        .implicit_dependencies
                        .iter()
                        .any(|dependency| dependency == name)
                })
                .map(|project| project.name.as_str())
                .collect(),
        }
    }
}

/// Depth-first post-order walk: a project is emitted after everything it
/// depends on. `stack` is the in-progress path; meeting a project already on
/// it is a cycle.
fn visit<'repo>(
    repo: &'repo Repo,
    name: &str,
    visited: &mut HashSet<&'repo str>,
    stack: &mut Vec<&'repo str>,
    order: &mut Vec<&'repo Project>,
) -> Result<()> {
    let Some(project) = repo.project(name) else {
        bail!("unknown project {name:?}");
    };
    if let Some(start) = stack.iter().position(|entry| *entry == project.name) {
        bail!(
            "dependency cycle: {} -> {}",
            stack[start..].join(" -> "),
            project.name,
        );
    }
    if visited.contains(project.name.as_str()) {
        return Ok(());
    }
    stack.push(&project.name);
    for dependency in &project.implicit_dependencies {
        visit(repo, dependency, visited, stack, order)?;
    }
    stack.pop();
    visited.insert(&project.name);
    order.push(project);
    Ok(())
}
