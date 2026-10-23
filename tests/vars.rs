//! `create-variables`: prompting through the line-IO boundary,
//! the five types, `--use-defaults`, skip-if-set, and persistence.

mod common;

use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

use dockit::repo::Repo;
use dockit::vars::{Console, HostFacts, create_variables, docker_gid_from_group};

/// The scripted side of the line-IO boundary: answers are dealt in order,
/// every prompt and warning is recorded for assertions.
struct ScriptedConsole {
    answers: VecDeque<String>,
    asked: Vec<String>,
    warnings: Vec<String>,
}

impl ScriptedConsole {
    fn with_answers(answers: &[&str]) -> Self {
        Self {
            answers: answers.iter().map(ToString::to_string).collect(),
            asked: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl Console for ScriptedConsole {
    fn ask(&mut self, prompt: &str) -> anyhow::Result<String> {
        self.asked.push(prompt.to_owned());
        Ok(self
            .answers
            .pop_front()
            .unwrap_or_else(|| panic!("unexpected prompt: {prompt}")))
    }

    fn warn(&mut self, message: &str) {
        self.warnings.push(message.to_owned());
    }
}

fn process_env(vars: &[(&str, &str)]) -> BTreeMap<String, String> {
    vars.iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn no_host_facts() -> HostFacts {
    HostFacts {
        docker_gid: None,
        hostname: None,
    }
}

fn env_local(root: &Path) -> String {
    std::fs::read_to_string(root.join(".env.local")).unwrap_or_default()
}

#[test]
fn text_answers_are_persisted_to_the_root_env_local_in_order() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&["alpha", "beta", "gamma"]);

    create_variables(
        &repo,
        &process_env(&[]),
        "text",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        env_local(&root),
        "TEXT_REQUIRED='alpha'\nTEXT_OPTIONAL='beta'\nTEXT_DEFAULT='gamma'\n"
    );
    assert!(
        console.asked[0].contains("Required text."),
        "{:?}",
        console.asked
    );
}

#[test]
fn variables_already_in_the_merged_environment_are_skipped() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&["beta", "gamma"]);

    create_variables(
        &repo,
        &process_env(&[("TEXT_REQUIRED", "preset")]),
        "text",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        env_local(&root),
        "TEXT_OPTIONAL='beta'\nTEXT_DEFAULT='gamma'\n"
    );
    assert_eq!(console.asked.len(), 2, "{:?}", console.asked);
}

#[test]
fn variables_set_in_a_dotenv_layer_are_skipped() {
    let root = common::fixture("variables");
    std::fs::write(root.join(".env"), "TEXT_REQUIRED=from-dotenv\n").unwrap();
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&["beta", "gamma"]);

    create_variables(
        &repo,
        &process_env(&[]),
        "text",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        env_local(&root),
        "TEXT_OPTIONAL='beta'\nTEXT_DEFAULT='gamma'\n"
    );
}

#[test]
fn an_empty_answer_for_a_required_variable_is_reprompted() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&["", "", "alpha", "beta", "gamma"]);

    create_variables(
        &repo,
        &process_env(&[]),
        "text",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        env_local(&root),
        "TEXT_REQUIRED='alpha'\nTEXT_OPTIONAL='beta'\nTEXT_DEFAULT='gamma'\n"
    );
    assert_eq!(console.asked.len(), 5, "{:?}", console.asked);
}

#[test]
fn an_empty_answer_for_an_optional_variable_persists_the_empty_string() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&["alpha", "", "gamma"]);

    create_variables(
        &repo,
        &process_env(&[]),
        "text",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        env_local(&root),
        "TEXT_REQUIRED='alpha'\nTEXT_OPTIONAL=''\nTEXT_DEFAULT='gamma'\n"
    );
}

#[test]
fn an_empty_answer_accepts_the_expanded_default_value_suggestion() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&["alpha", "beta", ""]);

    create_variables(
        &repo,
        &process_env(&[("DOMAIN", "example.test")]),
        "text",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        env_local(&root),
        "TEXT_REQUIRED='alpha'\nTEXT_OPTIONAL='beta'\nTEXT_DEFAULT='svc.example.test'\n"
    );
    assert!(
        console.asked[2].contains("svc.example.test"),
        "the suggestion is shown in the prompt: {:?}",
        console.asked
    );
}

fn is_generated_secret(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_alphanumeric())
}

#[test]
fn a_typed_password_answer_is_written_to_the_secrets_file_and_generated_never_prompts() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&["hunter2"]);

    create_variables(
        &repo,
        &process_env(&[]),
        "secrets",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(root.join(".secrets/PASS")).unwrap(),
        "hunter2"
    );
    let generated = std::fs::read_to_string(root.join(".secrets/GEN")).unwrap();
    assert!(is_generated_secret(&generated), "{generated:?}");
    assert_eq!(env_local(&root), "", "secrets never touch .env.local");
    assert_eq!(console.asked.len(), 1, "{:?}", console.asked);
    assert!(
        console.asked[0].contains("leave empty to generate"),
        "{:?}",
        console.asked
    );
}

#[test]
fn an_empty_password_answer_generates_a_random_secret() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&[""]);

    create_variables(
        &repo,
        &process_env(&[]),
        "secrets",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    let generated = std::fs::read_to_string(root.join(".secrets/PASS")).unwrap();
    assert!(is_generated_secret(&generated), "{generated:?}");
}

#[test]
fn secrets_with_an_existing_file_are_skipped_even_when_unset_in_the_environment() {
    let root = common::fixture("variables");
    std::fs::create_dir_all(root.join(".secrets")).unwrap();
    std::fs::write(root.join(".secrets/PASS"), "kept").unwrap();
    std::fs::write(root.join(".secrets/GEN"), "also-kept").unwrap();
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&[]);

    create_variables(
        &repo,
        &process_env(&[]),
        "secrets",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(root.join(".secrets/PASS")).unwrap(),
        "kept"
    );
    assert_eq!(
        std::fs::read_to_string(root.join(".secrets/GEN")).unwrap(),
        "also-kept"
    );
    assert!(console.asked.is_empty(), "{:?}", console.asked);
}

#[test]
fn boolean_answers_and_host_fact_suggestions_are_persisted() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let host = HostFacts {
        docker_gid: Some("988".to_owned()),
        hostname: Some("box".to_owned()),
    };
    // "y" answers the boolean; empty answers accept the GID and hostname
    // suggestions.
    let mut console = ScriptedConsole::with_answers(&["y", "", ""]);

    create_variables(
        &repo,
        &process_env(&[]),
        "special",
        false,
        &host,
        &mut console,
    )
    .unwrap();

    assert_eq!(
        env_local(&root),
        "FLAG='true'\nDOCKER_GID='988'\nHOST='box'\n"
    );
    assert!(console.asked[1].contains("988"), "{:?}", console.asked);
    assert!(console.asked[2].contains("box"), "{:?}", console.asked);
}

#[test]
fn a_negative_boolean_answer_persists_false_and_garbage_is_reprompted() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let host = HostFacts {
        docker_gid: Some("988".to_owned()),
        hostname: Some("box".to_owned()),
    };
    let mut console = ScriptedConsole::with_answers(&["maybe", "no", "", ""]);

    create_variables(
        &repo,
        &process_env(&[]),
        "special",
        false,
        &host,
        &mut console,
    )
    .unwrap();

    assert_eq!(
        env_local(&root),
        "FLAG='false'\nDOCKER_GID='988'\nHOST='box'\n"
    );
    assert_eq!(console.asked.len(), 4, "{:?}", console.asked);
}

#[test]
fn use_defaults_never_prompts_and_warns_for_empty_required_variables() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&[]);

    create_variables(
        &repo,
        &process_env(&[("DOMAIN", "example.test")]),
        "text",
        true,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        env_local(&root),
        "TEXT_REQUIRED=''\nTEXT_OPTIONAL=''\nTEXT_DEFAULT='svc.example.test'\n"
    );
    assert!(console.asked.is_empty(), "{:?}", console.asked);
    assert_eq!(console.warnings.len(), 1, "{:?}", console.warnings);
    assert!(
        console.warnings[0].contains("TEXT_REQUIRED"),
        "{:?}",
        console.warnings
    );
}

#[test]
fn use_defaults_generates_secrets_and_takes_host_facts() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let host = HostFacts {
        docker_gid: Some("988".to_owned()),
        hostname: Some("box".to_owned()),
    };
    let mut console = ScriptedConsole::with_answers(&[]);

    create_variables(
        &repo,
        &process_env(&[]),
        "secrets",
        true,
        &host,
        &mut console,
    )
    .unwrap();
    create_variables(
        &repo,
        &process_env(&[]),
        "special",
        true,
        &host,
        &mut console,
    )
    .unwrap();

    let pass = std::fs::read_to_string(root.join(".secrets/PASS")).unwrap();
    let generated = std::fs::read_to_string(root.join(".secrets/GEN")).unwrap();
    assert!(is_generated_secret(&pass), "{pass:?}");
    assert!(is_generated_secret(&generated), "{generated:?}");
    assert_eq!(env_local(&root), "FLAG=''\nDOCKER_GID='988'\nHOST='box'\n");
    assert!(console.asked.is_empty(), "{:?}", console.asked);
    assert_eq!(
        console.warnings.len(),
        1,
        "the boolean without a default warns"
    );
}

#[test]
fn use_defaults_falls_back_to_an_existing_environment_value() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&[]);

    create_variables(
        &repo,
        &process_env(&[("TOKEN", "issued-elsewhere")]),
        "token",
        true,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(root.join(".secrets/TOKEN")).unwrap(),
        "issued-elsewhere"
    );
    assert!(console.warnings.is_empty(), "{:?}", console.warnings);
}

#[test]
fn the_cascade_runs_dependencies_first_dedupes_by_name_and_sees_values_from_this_run() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    // shared's variables come first; DUP keeps shared's declaration; the
    // empty answer accepts CHILD_VALUE's suggestion, expanded with the
    // SHARED_VALUE persisted moments earlier in this same run.
    let mut console = ScriptedConsole::with_answers(&["melody", "dup-answer", ""]);

    create_variables(
        &repo,
        &process_env(&[]),
        "child",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        env_local(&root),
        "SHARED_VALUE='melody'\nDUP='dup-answer'\nCHILD_VALUE='melody-suffix'\n"
    );
    assert_eq!(console.asked.len(), 3, "{:?}", console.asked);
    assert!(
        console.asked[1].contains("Dup declared by shared."),
        "first occurrence wins: {:?}",
        console.asked
    );
}

#[test]
fn an_unknown_project_is_an_error() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&[]);

    let error = create_variables(
        &repo,
        &process_env(&[]),
        "nope",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap_err();

    assert!(error.to_string().contains("nope"), "{error}");
}

#[test]
fn the_docker_gid_is_parsed_from_group_file_text() {
    let group = "root:x:0:\nwheel:x:10:alice\ndocker:x:988:alice,bob\nusers:x:100:\n";
    assert_eq!(docker_gid_from_group(group), Some("988".to_owned()));
    assert_eq!(docker_gid_from_group("root:x:0:\n"), None);
    assert_eq!(
        docker_gid_from_group("dockerd:x:989:\ndocker:x:988:\n"),
        Some("988".to_owned()),
        "the name must match exactly"
    );
}

#[test]
fn an_unset_reference_in_a_default_value_expands_to_the_empty_string() {
    let root = common::fixture("variables");
    let repo = Repo::discover(&root).unwrap();
    let mut console = ScriptedConsole::with_answers(&["alpha", "beta", ""]);

    create_variables(
        &repo,
        &process_env(&[]),
        "text",
        false,
        &no_host_facts(),
        &mut console,
    )
    .unwrap();

    assert_eq!(
        env_local(&root),
        "TEXT_REQUIRED='alpha'\nTEXT_OPTIONAL='beta'\nTEXT_DEFAULT='svc.'\n"
    );
}
