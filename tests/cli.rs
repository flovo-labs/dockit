//! End-to-end tests of the `dockit` binary itself: argument wiring, repo
//! discovery from the working directory, exit codes. Only docker-free paths
//! (`--dry-run`, `list`, usage errors) run here.

mod common;

use std::process::Command;

fn dockit(cwd: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dockit"))
        .args(args)
        .current_dir(cwd)
        .env_remove("CI")
        .env_remove("ENVIRONMENT_NAME")
        .output()
        .unwrap()
}

#[test]
fn up_dry_run_prints_the_worked_example_lines_and_exits_zero() {
    let root = common::fixture("chain");

    let output = dockit(&root, &["up", "bonob", "--dry-run"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "docker compose --file apps/traefik/docker-compose.yml --file apps/traefik/docker-compose.local.yml up --yes --remove-orphans --detach\n\
         docker compose --file apps/navidrome/docker-compose.yml up --yes --remove-orphans --detach\n\
         docker compose --file apps/bonob/docker-compose.yml --file apps/bonob/docker-compose.local.yml up --yes --remove-orphans --detach\n"
    );
}

#[test]
fn up_dry_run_no_deps_prints_only_the_named_project() {
    // The contrast with the worked example above: same project, one line
    // instead of three, and no commentary about what was skipped — dry-run
    // prints invocations and nothing else.
    let root = common::fixture("chain");

    let output = dockit(&root, &["up", "bonob", "--dry-run", "--no-deps"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "docker compose --file apps/bonob/docker-compose.yml --file apps/bonob/docker-compose.local.yml up --yes --remove-orphans --detach\n"
    );
}

#[test]
fn up_no_deps_is_the_ci_shape_and_does_not_pass_composes_own_no_deps() {
    // What docker-host CI will run. dockit's --no-deps cuts dockit's graph only;
    // compose's like-named flag must not appear in the invocation.
    let root = common::fixture("chain");

    let output = dockit(&root, &["up", "bonob", "--no-deps", "--wait", "--dry-run"]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "one leg only: {stdout}");
    assert!(
        stdout
            .trim_end()
            .ends_with("up --yes --remove-orphans --detach --wait --wait-timeout 120"),
        "wait flags ride along, no compose --no-deps: {stdout}"
    );
}

#[test]
fn down_and_pull_dry_run_print_their_single_invocation() {
    let root = common::fixture("chain");

    let down = dockit(&root, &["down", "bonob", "--dry-run"]);
    let pull = dockit(&root, &["pull", "navidrome", "--dry-run"]);

    assert!(down.status.success(), "{down:?}");
    assert_eq!(
        String::from_utf8(down.stdout).unwrap(),
        "docker compose --file apps/bonob/docker-compose.yml --file apps/bonob/docker-compose.local.yml down --remove-orphans\n"
    );
    assert!(pull.status.success(), "{pull:?}");
    assert_eq!(
        String::from_utf8(pull.stdout).unwrap(),
        "docker compose --file apps/navidrome/docker-compose.yml pull\n"
    );
}

#[test]
fn restart_dry_run_prints_the_down_leg_then_the_up_leg() {
    let root = common::fixture("chain");

    let output = dockit(&root, &["restart", "bonob", "--dry-run"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "docker compose --file apps/bonob/docker-compose.yml --file apps/bonob/docker-compose.local.yml down --remove-orphans\n\
         docker compose --file apps/bonob/docker-compose.yml --file apps/bonob/docker-compose.local.yml up --yes --remove-orphans --detach\n"
    );
}

#[test]
fn restart_dry_run_honors_no_detach() {
    let root = common::fixture("chain");

    let output = dockit(&root, &["restart", "navidrome", "--no-detach", "--dry-run"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "docker compose --file apps/navidrome/docker-compose.yml down --remove-orphans\n\
         docker compose --file apps/navidrome/docker-compose.yml up --yes --remove-orphans\n"
    );
}

#[test]
fn up_dry_run_honors_flags_and_works_from_a_subdirectory() {
    let root = common::fixture("chain");

    // The CLI works from any subdirectory of the repository.
    let output = dockit(
        &root.join("apps/navidrome"),
        &["up", "navidrome", "--force", "--no-detach", "--dry-run"],
    );

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "docker compose --file apps/navidrome/docker-compose.yml up --yes --remove-orphans --force-recreate\n"
    );
}

#[test]
fn up_dry_run_wait_appends_wait_and_the_default_timeout() {
    // `dockit up <app> --wait` blocks until healthy — every leg gains
    // `--wait --wait-timeout 120`.
    let root = common::fixture("chain");

    let output = dockit(&root, &["up", "bonob", "--wait", "--dry-run"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "docker compose --file apps/traefik/docker-compose.yml --file apps/traefik/docker-compose.local.yml up --yes --remove-orphans --detach --wait --wait-timeout 120\n\
         docker compose --file apps/navidrome/docker-compose.yml up --yes --remove-orphans --detach --wait --wait-timeout 120\n\
         docker compose --file apps/bonob/docker-compose.yml --file apps/bonob/docker-compose.local.yml up --yes --remove-orphans --detach --wait --wait-timeout 120\n"
    );
}

#[test]
fn up_dry_run_wait_honors_a_custom_timeout() {
    // --wait-timeout <secs> overrides the default 120.
    let root = common::fixture("chain");

    let output = dockit(
        &root,
        &[
            "up",
            "navidrome",
            "--wait",
            "--wait-timeout",
            "45",
            "--dry-run",
        ],
    );

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "docker compose --file apps/navidrome/docker-compose.yml up --yes --remove-orphans --detach --wait --wait-timeout 45\n"
    );
}

#[test]
fn up_wait_and_no_detach_are_mutually_exclusive() {
    // compose rejects --wait in attached mode, so dockit refuses the
    // combination up front with a clear error rather than spawning a doomed
    // compose call.
    let root = common::fixture("chain");

    let output = dockit(
        &root,
        &["up", "bonob", "--wait", "--no-detach", "--dry-run"],
    );

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("--wait") && stderr.contains("--no-detach"),
        "names the conflicting flags: {stderr}"
    );
}

#[test]
fn up_wait_timeout_requires_wait() {
    // --wait-timeout is meaningless without --wait, so supplying
    // it alone is an error rather than a silent no-op.
    let root = common::fixture("chain");

    let output = dockit(&root, &["up", "bonob", "--wait-timeout", "45", "--dry-run"]);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--wait"), "points at --wait: {stderr}");
}

#[test]
fn list_prints_the_projects_sorted_by_name() {
    let root = common::fixture("chain");

    let output = dockit(&root, &["list"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "bonob (networks, traefik, navidrome)\n\
         navidrome (networks)\n\
         networks\n\
         traefik (networks)\n"
    );
}

#[test]
fn naming_a_project_inside_an_ignored_directory_reports_an_unknown_project() {
    // The end-to-end proof that ignored means nonexistent rather than
    // hidden: the name is not merely absent from listings, it is not a name.
    let root = common::fixture("ignored");

    let output = dockit(&root, &["up", "retired"]);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("unknown project") && stderr.contains("retired"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn deps_defaults_to_forward_direct_in_list_style() {
    let root = common::fixture("deps");

    let output = dockit(&root, &["deps", "bonob"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "bonob (navidrome, traefik)\n"
    );
}

#[test]
fn deps_reverse_transitive_json_is_the_ci_shape() {
    // The exact invocation CI issues per changed app: reverse-transitive JSON.
    let root = common::fixture("deps");

    let output = dockit(
        &root,
        &["deps", "networks", "--reverse", "--transitive", "--json"],
    );

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "{\"project\":\"networks\",\"direction\":\"reverse\",\"transitive\":true,\
         \"deps\":[\"baikal\",\"bonob\",\"navidrome\",\"traefik\",\"whoami\"]}\n"
    );
}

#[test]
fn deps_of_a_leaf_is_empty_and_exits_zero() {
    // "nothing depends on it" is a success with an empty set, not an error.
    let root = common::fixture("deps");

    let output = dockit(&root, &["deps", "bonob", "--reverse"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "bonob\n");
}

#[test]
fn deps_of_an_unknown_project_is_a_nonzero_exit_naming_the_valid_projects() {
    let root = common::fixture("deps");

    let output = dockit(&root, &["deps", "ghost"]);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ghost"), "{stderr}");
    assert!(stderr.contains("traefik"), "{stderr}");
}

#[test]
fn an_unknown_project_is_a_nonzero_exit_naming_the_valid_projects() {
    let root = common::fixture("chain");

    let output = dockit(&root, &["up", "sonarr", "--dry-run"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("sonarr"), "{stderr}");
    assert!(stderr.contains("traefik"), "{stderr}");
}

#[test]
fn a_bare_dockit_prints_the_usage_summary() {
    let root = common::fixture("chain");

    let output = dockit(&root, &[]);

    // clap's arg_required_else_help exits non-zero with usage on stderr.
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Usage"), "{stderr}");
    for subcommand in [
        "up",
        "down",
        "pull",
        "logs",
        "list",
        "ps",
        "new",
        "configure",
        "create-variables",
        "create-volumes",
        "create-networks",
        "backup-volumes",
        "restore-volumes",
    ] {
        assert!(stderr.contains(subcommand), "{stderr}");
    }
}

#[test]
fn new_scaffolds_an_app_and_a_collision_is_a_nonzero_exit() {
    let root = common::fixture("chain");

    let created = dockit(&root, &["new", "testapp"]);

    assert!(created.status.success(), "{created:?}");
    for file in [
        "docker-compose.yml",
        "docker-compose.local.yml",
        "project.json",
        ".env",
    ] {
        assert!(
            root.join("apps/testapp").join(file).is_file(),
            "{file} missing"
        );
    }

    // A name collision is an error before any file is written.
    let collided = dockit(&root, &["new", "Testapp"]);
    assert!(!collided.status.success(), "{collided:?}");
    let stderr = String::from_utf8(collided.stderr).unwrap();
    assert!(stderr.contains("testapp"), "{stderr}");
}

#[test]
fn backup_commands_are_wired_and_reject_unknown_projects_before_docker() {
    let root = common::fixture("backup");

    for args in [["backup-volumes", "sonarr"], ["restore-volumes", "sonarr"]] {
        let output = dockit(&root, &args);
        assert!(!output.status.success(), "{args:?}: {output:?}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("unknown project"), "{args:?}: {stderr}");
        assert!(stderr.contains("app"), "{args:?}: {stderr}");
    }
}

#[test]
fn restore_volumes_missing_archives_fail_end_to_end_before_docker() {
    let root = common::fixture("backup");

    let output = dockit(&root, &["restore-volumes", "app", "--source-path", "void"]);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("app-data.tar.bz2"), "{stderr}");
    assert!(stderr.contains("nothing restored"), "{stderr}");
}

#[test]
fn logs_is_wired_and_rejects_graph_only_and_unknown_projects_before_docker() {
    // The guard fires before any docker spawn, so this end-to-end
    // run stays docker-free. Asserting the guard's own message (not merely a
    // non-zero exit, which an unknown subcommand would also give) proves the
    // `logs` command is wired, reached, and applying the shared guard.
    let root = common::fixture("chain");

    let graph_only = dockit(&root, &["logs", "networks"]);
    assert!(!graph_only.status.success(), "{graph_only:?}");
    let stderr = String::from_utf8(graph_only.stderr).unwrap();
    assert!(
        stderr.contains("networks") && stderr.contains("compose"),
        "{stderr}"
    );

    let unknown = dockit(&root, &["logs", "sonarr"]);
    assert!(!unknown.status.success(), "{unknown:?}");
    let stderr = String::from_utf8(unknown.stderr).unwrap();
    assert!(stderr.contains("unknown project"), "{stderr}");
    assert!(stderr.contains("traefik"), "{stderr}");
}

#[test]
fn provisioning_commands_are_wired_and_reject_unknown_projects_before_docker() {
    let root = common::fixture("chain");

    // The guard errors list the valid names and fire before any docker
    // spawn, keeping these end-to-end runs docker-free.
    for args in [
        ["create-volumes", "sonarr"],
        ["create-networks", "sonarr"],
        ["configure", "sonarr"],
    ] {
        let output = dockit(&root, &args);
        assert!(!output.status.success(), "{args:?}: {output:?}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("unknown project"), "{args:?}: {stderr}");
        assert!(stderr.contains("traefik"), "{args:?}: {stderr}");
    }
}

#[test]
fn create_variables_use_defaults_runs_end_to_end_without_a_terminal() {
    let root = common::fixture("variables");

    let output = Command::new(env!("CARGO_BIN_EXE_dockit"))
        .args(["create-variables", "text", "--use-defaults"])
        .current_dir(&root)
        .env_remove("CI")
        .env_remove("ENVIRONMENT_NAME")
        .env("DOMAIN", "example.test")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        std::fs::read_to_string(root.join(".env.local")).unwrap(),
        "TEXT_REQUIRED=''\nTEXT_OPTIONAL=''\nTEXT_DEFAULT='svc.example.test'\n"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("TEXT_REQUIRED"), "{stderr}");
}

#[test]
fn all_subcommand_is_wired_with_its_three_verbs() {
    // Bare `dockit all` needs a verb — clap prints help listing the
    // three bulk verbs. Docker-free (no verb is ever run), and proves the
    // `all` command and its verbs are wired.
    let root = common::fixture("chain");

    let output = dockit(&root, &["all"]);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    for verb in ["down", "restart", "pull"] {
        assert!(stderr.contains(verb), "{stderr}");
    }
}

#[test]
fn completions_prints_the_bash_wrapper_script() {
    // `dockit completions bash` emits the wrapper to stdout. Repo-free
    // (script generation never touches discovery), so any cwd works.
    let root = common::fixture("basic");

    let output = dockit(&root, &["completions", "bash"]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("complete -F _dockit dockit"), "{stdout}");
    assert!(stdout.contains("dockit __complete"), "{stdout}");
}

#[test]
fn completions_rejects_an_unknown_shell() {
    let root = common::fixture("basic");

    let output = dockit(&root, &["completions", "powershell"]);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("powershell"), "{stderr}");
}

#[test]
fn complete_lists_the_real_subcommands_and_hides_its_own_helper() {
    // The clap-derived grammar drives `__complete`: every visible subcommand
    // appears, the hidden `__complete` never does.
    let root = common::fixture("listing");

    let output = dockit(&root, &["__complete", ""]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    for expected in ["up", "down", "logs", "list", "ps", "all", "completions"] {
        assert!(lines.contains(&expected), "{lines:?} missing {expected}");
    }
    assert!(!lines.contains(&"__complete"), "helper leaked: {lines:?}");
}

#[test]
fn complete_offers_discovered_project_names_after_a_project_verb() {
    let root = common::fixture("listing");

    let output = dockit(&root, &["__complete", "up", "wh"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "whoami\n");
}

#[test]
fn complete_stays_silent_and_successful_outside_a_repository() {
    // Fires in the user's shell anywhere: outside a repo it must exit 0 with
    // subcommands (not project names), never an error on stderr.
    let dir = std::env::temp_dir();

    let output = dockit(&dir, &["__complete", "up", ""]);

    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
}
