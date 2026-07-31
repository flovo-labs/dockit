//! Tests for the shell-completion engine: the grammar-walking
//! completer behind `tug __complete`, driven against fixture repos with a
//! hand-built grammar. The clap-introspection wiring is covered end-to-end
//! in `cli.rs`.

mod common;

use tug::completion::{self, CommandGrammar, Shell};
use tug::repo::Repo;

/// A small stand-in for tug's real grammar: a project-taking leaf with
/// flags (`up`), a bare leaf (`list`), another project leaf (`logs`), and
/// the nested `all` umbrella. Independent of clap so the engine is tested
/// on its own terms.
fn grammar() -> Vec<CommandGrammar> {
    fn leaf(name: &str, takes_project: bool, flags: &[&str]) -> CommandGrammar {
        CommandGrammar {
            name: name.to_owned(),
            takes_project,
            flags: flags.iter().map(|f| (*f).to_owned()).collect(),
            subcommands: Vec::new(),
        }
    }
    vec![
        leaf("up", true, &["--force", "--no-detach", "--dry-run"]),
        leaf("logs", true, &["--follow"]),
        leaf("list", false, &["--all"]),
        CommandGrammar {
            name: "all".to_owned(),
            takes_project: false,
            flags: Vec::new(),
            subcommands: vec![
                leaf("down", false, &[]),
                leaf("restart", false, &[]),
                leaf("pull", false, &[]),
            ],
        },
    ]
}

#[test]
fn an_empty_first_word_completes_every_subcommand() {
    let repo = Repo::discover(&common::fixture("basic")).unwrap();

    let got = completion::candidates(&grammar(), Some(&repo), &[""]);

    assert_eq!(got, vec!["all", "list", "logs", "up"]);
}

#[test]
fn a_partial_first_word_filters_subcommands_by_prefix() {
    let repo = Repo::discover(&common::fixture("basic")).unwrap();

    let got = completion::candidates(&grammar(), Some(&repo), &["l"]);

    assert_eq!(got, vec!["list", "logs"]);
}

#[test]
fn a_project_taking_subcommand_completes_discovered_project_names() {
    let repo = Repo::discover(&common::fixture("basic")).unwrap();

    let got = completion::candidates(&grammar(), Some(&repo), &["up", ""]);

    assert_eq!(got, vec!["networks", "traefik", "whoami"]);
}

#[test]
fn a_partial_project_name_is_prefix_filtered() {
    let repo = Repo::discover(&common::fixture("basic")).unwrap();

    let got = completion::candidates(&grammar(), Some(&repo), &["up", "wh"]);

    assert_eq!(got, vec!["whoami"]);
}

#[test]
fn a_subcommand_that_takes_no_project_offers_nothing_in_its_argument_slot() {
    let repo = Repo::discover(&common::fixture("basic")).unwrap();

    let got = completion::candidates(&grammar(), Some(&repo), &["list", ""]);

    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn a_dash_prefixed_word_completes_the_subcommands_flags() {
    let repo = Repo::discover(&common::fixture("basic")).unwrap();

    let got = completion::candidates(&grammar(), Some(&repo), &["up", "--"]);

    assert_eq!(got, vec!["--dry-run", "--force", "--no-detach"]);
}

#[test]
fn flags_are_still_offered_after_the_project_is_supplied() {
    let repo = Repo::discover(&common::fixture("basic")).unwrap();

    let got = completion::candidates(&grammar(), Some(&repo), &["up", "whoami", "--f"]);

    assert_eq!(got, vec!["--force"]);
}

#[test]
fn a_project_name_is_not_offered_twice_once_the_slot_is_filled() {
    let repo = Repo::discover(&common::fixture("basic")).unwrap();

    let got = completion::candidates(&grammar(), Some(&repo), &["up", "whoami", ""]);

    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn the_all_umbrella_completes_its_nested_verbs() {
    let repo = Repo::discover(&common::fixture("basic")).unwrap();

    let got = completion::candidates(&grammar(), Some(&repo), &["all", ""]);

    assert_eq!(got, vec!["down", "pull", "restart"]);
}

#[test]
fn a_nested_verb_is_prefix_filtered() {
    let repo = Repo::discover(&common::fixture("basic")).unwrap();

    let got = completion::candidates(&grammar(), Some(&repo), &["all", "d"]);

    assert_eq!(got, vec!["down"]);
}

#[test]
fn outside_a_repo_subcommands_still_complete_but_project_names_fall_away() {
    // No repo: subcommands and flags survive, the project slot is empty.
    assert_eq!(
        completion::candidates(&grammar(), None, &[""]),
        vec!["all", "list", "logs", "up"]
    );
    assert!(
        completion::candidates(&grammar(), None, &["up", ""]).is_empty(),
        "project names must not appear without a repo"
    );
    assert_eq!(
        completion::candidates(&grammar(), None, &["up", "--"]),
        vec!["--dry-run", "--force", "--no-detach"]
    );
}

#[test]
fn the_bash_script_wires_a_completion_function_to_tug_complete() {
    let script = completion::script(Shell::Bash);

    assert!(script.contains("complete -F"), "{script}");
    assert!(script.contains("tug __complete"), "{script}");
    assert!(script.trim_end().ends_with("tug"), "{script}");
}

#[test]
fn the_zsh_script_is_an_autoloaded_compdef_calling_tug_complete() {
    let script = completion::script(Shell::Zsh);

    assert!(script.starts_with("#compdef tug"), "{script}");
    assert!(script.contains("tug __complete"), "{script}");
    assert!(script.contains("compadd"), "{script}");
}

#[test]
fn the_fish_script_registers_a_completion_calling_tug_complete() {
    let script = completion::script(Shell::Fish);

    assert!(script.contains("complete -c tug"), "{script}");
    assert!(script.contains("tug __complete"), "{script}");
}
