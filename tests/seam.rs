//! Tests for the process-spawn seam: every docker
//! invocation flows through the `Runner` trait as an `Invocation` value, so a
//! test can record the exact argv sequence a command would spawn.

use tug::process::{Invocation, Outcome, RecordingRunner, Runner, SystemRunner, render};

#[test]
fn render_produces_the_one_line_command_rendering() {
    // Plain args verbatim; args with whitespace or
    // quotes single-quoted so the line is unambiguous.
    let plain = Invocation::new(
        [
            "docker",
            "compose",
            "--file",
            "apps/bonob/docker-compose.yml",
            "up",
            "--yes",
        ],
        "/repo",
    );
    let quoted = Invocation::new(
        [
            "docker",
            "run",
            "--entrypoint",
            "/bin/sh",
            "busybox",
            "-c",
            "chown -R 65001:65001 /volume",
        ],
        "/repo",
    );

    assert_eq!(
        render(&plain),
        "docker compose --file apps/bonob/docker-compose.yml up --yes"
    );
    assert_eq!(
        render(&quoted),
        "docker run --entrypoint /bin/sh busybox -c 'chown -R 65001:65001 /volume'"
    );
}

#[test]
fn system_runner_propagates_the_child_exit_code() {
    let mut runner = SystemRunner::new();
    let ok = runner
        .run(&Invocation::new(["/bin/sh", "-c", "exit 0"], "/"))
        .unwrap();
    let failed = runner
        .run(&Invocation::new(["/bin/sh", "-c", "exit 3"], "/"))
        .unwrap();

    assert_eq!(ok.exit_code, 0);
    assert_eq!(failed.exit_code, 3);
}

#[test]
fn system_runner_captures_stdout_when_asked() {
    // The shape used by read-only queries like `docker volume ls --format json`.
    let mut runner = SystemRunner::new();
    let invocation = Invocation::new(["/bin/sh", "-c", "printf 'hi'"], "/").with_captured_stdout();

    let outcome = runner.run(&invocation).unwrap();

    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.stdout.as_deref(), Some("hi"));
}

#[test]
fn system_runner_wires_stdout_to_a_file_natively() {
    // The shape used by backup-volumes: child stdout becomes the archive file.
    let dir = temp_dir("stdout-to-file");
    let target = dir.join("volume.tar.bz2");
    let mut runner = SystemRunner::new();
    let invocation = Invocation::new(["/bin/sh", "-c", "printf 'archive-bytes'"], "/")
        .with_stdout_to_file(&target);

    let outcome = runner.run(&invocation).unwrap();

    assert_eq!(outcome.exit_code, 0);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "archive-bytes");
}

#[test]
fn system_runner_wires_stdin_from_a_file_natively() {
    // The shape used by restore-volumes: the archive file becomes child stdin.
    let dir = temp_dir("stdin-from-file");
    let source = dir.join("volume.tar.bz2");
    std::fs::write(&source, "archive-bytes").unwrap();
    let mut runner = SystemRunner::new();
    let invocation = Invocation::new(["/bin/sh", "-c", "cat"], "/")
        .with_stdin_from_file(&source)
        .with_captured_stdout();

    let outcome = runner.run(&invocation).unwrap();

    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.stdout.as_deref(), Some("archive-bytes"));
}

#[test]
fn system_runner_gives_the_child_exactly_the_invocation_env() {
    // The invocation env is the whole child environment: values in
    // it are visible, and the CLI's own env (HOME is always set) does not leak.
    let mut runner = SystemRunner::new();
    let invocation = Invocation::new(
        ["/bin/sh", "-c", r#"printf '%s|%s' "$GREETING" "$HOME""#],
        "/",
    )
    .with_env([("GREETING".to_string(), "hello".to_string())])
    .with_captured_stdout();

    let outcome = runner.run(&invocation).unwrap();

    assert_eq!(outcome.stdout.as_deref(), Some("hello|"));
}

/// A fresh scratch directory unique to this test process.
fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tug-seam-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn recording_runner_records_argv_sequences_in_order() {
    // Expected argv taken from a worked example.
    let first = Invocation::new(
        [
            "docker",
            "compose",
            "--file",
            "apps/traefik/docker-compose.yml",
            "up",
            "--yes",
            "--remove-orphans",
            "--detach",
        ],
        "/repo",
    );
    let second = Invocation::new(
        [
            "docker",
            "compose",
            "--file",
            "apps/navidrome/docker-compose.yml",
            "up",
            "--yes",
            "--remove-orphans",
            "--detach",
        ],
        "/repo",
    );

    let mut runner = RecordingRunner::new();
    let outcome = runner.run(&first).unwrap();
    runner.run(&second).unwrap();

    assert_eq!(outcome, Outcome::success());
    let recorded: Vec<Vec<String>> = runner
        .recorded()
        .iter()
        .map(|inv| inv.argv.clone())
        .collect();
    assert_eq!(
        recorded,
        vec![
            vec![
                "docker",
                "compose",
                "--file",
                "apps/traefik/docker-compose.yml",
                "up",
                "--yes",
                "--remove-orphans",
                "--detach",
            ],
            vec![
                "docker",
                "compose",
                "--file",
                "apps/navidrome/docker-compose.yml",
                "up",
                "--yes",
                "--remove-orphans",
                "--detach",
            ],
        ]
    );
}
