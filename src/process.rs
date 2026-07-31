//! The process-spawn seam.
//!
//! All external effects of the CLI are spawns of the `docker` binary. They
//! flow through this module as [`Invocation`] values handed to a [`Runner`].
//! Adapters at this seam: a real spawner, a recording test double, and (for
//! `--dry-run`) a printing implementation — all sharing one interface.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};

/// A single child-process invocation, described as pure data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// Full argv; `argv[0]` is the program.
    pub argv: Vec<String>,
    /// Working directory of the child (the repo root for compose).
    pub cwd: String,
    /// Where the child's stdout goes.
    pub stdout: StdoutSink,
    /// Where the child's stdin comes from.
    pub stdin: StdinSource,
    /// The child's *entire* environment — the merged env, not
    /// additions on top of the CLI's own (keeps spawns deterministic).
    pub env: BTreeMap<String, String>,
}

/// Destination of a child's stdout.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StdoutSink {
    /// Stream through to the CLI's own stdout.
    #[default]
    Inherit,
    /// Return it on the [`Outcome`] — for machine-readable queries.
    Capture,
    /// Write it to a file natively, no shell redirection — for backups.
    File(PathBuf),
}

/// Source of a child's stdin.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StdinSource {
    /// Stream through from the CLI's own stdin.
    #[default]
    Inherit,
    /// Feed it from a file natively, no shell redirection — for restores.
    File(PathBuf),
}

impl Invocation {
    pub fn new<I, S>(argv: I, cwd: impl Into<String>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            argv: argv.into_iter().map(Into::into).collect(),
            cwd: cwd.into(),
            stdout: StdoutSink::Inherit,
            stdin: StdinSource::Inherit,
            env: BTreeMap::new(),
        }
    }

    /// Set the child's entire environment.
    #[must_use]
    pub fn with_env(mut self, env: impl IntoIterator<Item = (String, String)>) -> Self {
        self.env = env.into_iter().collect();
        self
    }

    /// Return the child's stdout on the [`Outcome`] instead of streaming it.
    #[must_use]
    pub fn with_captured_stdout(mut self) -> Self {
        self.stdout = StdoutSink::Capture;
        self
    }

    /// Write the child's stdout to `path` natively.
    #[must_use]
    pub fn with_stdout_to_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.stdout = StdoutSink::File(path.into());
        self
    }

    /// Feed the child's stdin from `path` natively.
    #[must_use]
    pub fn with_stdin_from_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.stdin = StdinSource::File(path.into());
        self
    }
}

/// What running an [`Invocation`] produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The child's exit code, propagated as the CLI's own on failure.
    pub exit_code: i32,
    /// The child's stdout, present iff it was run with [`StdoutSink::Capture`].
    pub stdout: Option<String>,
}

impl Outcome {
    #[must_use]
    pub fn success() -> Self {
        Self {
            exit_code: 0,
            stdout: None,
        }
    }
}

/// One-line rendering of an invocation: echoed before each spawn
/// and printed verbatim by `--dry-run`.
#[must_use]
pub fn render(invocation: &Invocation) -> String {
    let words: Vec<String> = invocation
        .argv
        .iter()
        .map(|arg| {
            if arg.is_empty() || arg.contains([' ', '\t', '\n', '\'', '"']) {
                format!("'{}'", arg.replace('\'', r"'\''"))
            } else {
                arg.clone()
            }
        })
        .collect();
    words.join(" ")
}

/// The seam: one method through which every invocation flows.
pub trait Runner {
    /// Run one invocation to completion and report its outcome.
    ///
    /// # Errors
    ///
    /// Fails when the child cannot be spawned or waited on at all; a child
    /// that runs and exits non-zero is an `Ok` outcome carrying that code.
    fn run(&mut self, invocation: &Invocation) -> Result<Outcome>;
}

/// The real adapter: spawns the child and waits for it, sequentially.
#[derive(Debug, Default)]
pub struct SystemRunner {}

impl SystemRunner {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// SIGINT/SIGTERM forwarding: the CLI must hand the signal to the
// running child and then exit with the child's exit code. std exposes no
// signal API, and the curated crate set has no signal crate, so the two
// libc calls are declared by hand — std already links libc on this target.
unsafe extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn kill(pid: i32, sig: i32) -> i32;
}

const SIGINT: i32 = 2;
const SIGTERM: i32 = 15;

/// The pid of the currently running child, `0` between children. Written
/// around each spawn/wait; read from the signal handler.
static CHILD_PID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// Async-signal-safe handler: forward the signal to the running child (whose
/// death then ends the wait) and otherwise swallow it — the CLI itself exits
/// via the propagated exit code, not via signal death.
extern "C" fn forward_to_child(sig: i32) {
    let pid = CHILD_PID.load(std::sync::atomic::Ordering::Relaxed);
    if pid > 0 {
        unsafe { kill(pid, sig) };
    }
}

/// Install the forwarding handler once per process.
fn install_signal_forwarding() {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| unsafe {
        signal(SIGINT, forward_to_child as *const () as usize);
        signal(SIGTERM, forward_to_child as *const () as usize);
    });
}

impl Runner for SystemRunner {
    fn run(&mut self, invocation: &Invocation) -> Result<Outcome> {
        let (program, args) = invocation
            .argv
            .split_first()
            .ok_or_else(|| anyhow::anyhow!("invocation has an empty argv"))?;
        let mut command = std::process::Command::new(program);
        command
            .args(args)
            .current_dir(&invocation.cwd)
            .env_clear()
            .envs(&invocation.env);
        match &invocation.stdout {
            StdoutSink::Inherit => {}
            StdoutSink::Capture => {
                command.stdout(std::process::Stdio::piped());
            }
            StdoutSink::File(path) => {
                let file = std::fs::File::create(path)
                    .with_context(|| format!("failed to create {}", path.display()))?;
                command.stdout(file);
            }
        }
        if let StdinSource::File(path) = &invocation.stdin {
            let file = std::fs::File::open(path)
                .with_context(|| format!("failed to open {}", path.display()))?;
            command.stdin(file);
        }
        install_signal_forwarding();
        eprintln!("+ {}", render(invocation));
        let child = command
            .spawn()
            .with_context(|| format!("failed to spawn {program}"))?;
        #[allow(clippy::cast_possible_wrap)]
        CHILD_PID.store(child.id() as i32, std::sync::atomic::Ordering::Relaxed);
        let output = child.wait_with_output();
        CHILD_PID.store(0, std::sync::atomic::Ordering::Relaxed);
        let output = output.with_context(|| format!("failed to wait for {program}"))?;
        let status = output.status;
        let exit_code = status.code().unwrap_or_else(|| {
            use std::os::unix::process::ExitStatusExt;
            status.signal().map_or(1, |sig| 128 + sig)
        });
        let stdout = (invocation.stdout == StdoutSink::Capture)
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned());
        Ok(Outcome { exit_code, stdout })
    }
}

/// The `--dry-run` adapter: prints each invocation's one-line
/// rendering instead of spawning anything, reporting success so the caller
/// walks the full chain.
#[derive(Debug)]
pub struct DryRunRunner<W: std::io::Write> {
    out: W,
}

impl<W: std::io::Write> DryRunRunner<W> {
    pub fn new(out: W) -> Self {
        Self { out }
    }
}

impl<W: std::io::Write> Runner for DryRunRunner<W> {
    fn run(&mut self, invocation: &Invocation) -> Result<Outcome> {
        writeln!(self.out, "{}", render(invocation)).context("failed to print the dry-run line")?;
        Ok(Outcome::success())
    }
}

/// Test adapter: records every invocation instead of spawning anything.
#[derive(Debug, Default)]
pub struct RecordingRunner {
    recorded: Vec<Invocation>,
}

impl RecordingRunner {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The invocations run so far, in order.
    #[must_use]
    pub fn recorded(&self) -> &[Invocation] {
        &self.recorded
    }
}

impl Runner for RecordingRunner {
    fn run(&mut self, invocation: &Invocation) -> Result<Outcome> {
        self.recorded.push(invocation.clone());
        Ok(Outcome::success())
    }
}
