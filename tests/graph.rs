//! Tests for the dependency graph: transitive closure, cycle
//! detection, deterministic topological order, and the cascade-collection
//! helper.

mod common;

use tug::repo::Repo;

#[test]
fn dependency_order_is_topological_with_declaration_order_ties() {
    // The worked example: bonob → [networks, traefik,
    // navidrome], traefik → [networks], navidrome → [networks]. networks is
    // visited once (first visit wins), dependencies come first.
    let root = common::fixture("chain");
    let repo = Repo::discover(&root).unwrap();

    let order: Vec<&str> = repo
        .dependency_order("bonob")
        .unwrap()
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    assert_eq!(order, ["networks", "traefik", "navidrome", "bonob"]);
}

#[test]
fn ordering_an_unknown_project_is_an_error() {
    let root = common::fixture("chain");
    let repo = Repo::discover(&root).unwrap();

    let message = format!("{:#}", repo.dependency_order("ghost").unwrap_err());

    assert!(
        message.contains("unknown project") && message.contains("ghost"),
        "unexpected error: {message}"
    );
}

#[test]
fn a_dependency_cycle_is_an_error_naming_the_projects_involved() {
    // The cyclic fixture: alpha → beta → gamma → alpha.
    let root = common::fixture("cyclic");
    let repo = Repo::discover(&root).unwrap();

    let message = format!("{:#}", repo.dependency_order("alpha").unwrap_err());

    assert!(
        message.contains("cycle")
            && message.contains("alpha")
            && message.contains("beta")
            && message.contains("gamma"),
        "unexpected error: {message}"
    );
}

#[test]
fn cascade_collects_dependency_specs_first_and_dedupes_by_name() {
    // Cascade collection: transitive dependencies' specs in topological
    // order, then the named project's own, first occurrence of a name wins.
    // All four chain projects declare DEFAULT_DOMAIN_SUFFIX with different
    // descriptions, so the survivor identifies whose declaration won.
    let root = common::fixture("chain");
    let repo = Repo::discover(&root).unwrap();

    let variables = repo.cascade("bonob", |p| p.variables.as_slice()).unwrap();

    let names: Vec<&str> = variables.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "DEFAULT_DOMAIN_SUFFIX",
            "TRAEFIK_DOMAIN",
            "NAVIDROME_DOMAIN",
            "BONOB_DOMAIN",
        ]
    );
    assert_eq!(variables[0].description, "Declared by traefik.");
}
