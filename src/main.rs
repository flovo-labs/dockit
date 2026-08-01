//! Thin binary adapter: argument parsing lives here, behavior lives in the
//! library so tests can drive it through the seam.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use tug::backup;
use tug::commands::{self, DepsOptions, RestartOptions, UpOptions};
use tug::completion;
use tug::graph::{Depth, Direction};
use tug::process::{DryRunRunner, Runner, SystemRunner};
use tug::provision;
use tug::repo::Repo;
use tug::scaffold;
use tug::vars::{self, HostFacts, TerminalConsole};

#[derive(Parser)]
#[command(
    name = "tug",
    about = "Orchestrates the repository's Docker Compose projects",
    version,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start a project and all its transitive dependencies, in order
    Up {
        /// The project to start
        project: String,
        /// Start only this project, skipping its dependencies
        #[arg(long)]
        no_deps: bool,
        /// Recreate containers even if unchanged (compose --force-recreate)
        #[arg(long)]
        force: bool,
        /// Stay attached instead of passing --detach to compose
        #[arg(long, conflicts_with = "wait")]
        no_detach: bool,
        /// Block until every container is healthy (compose --wait, per leg)
        #[arg(long)]
        wait: bool,
        /// Seconds to wait for health before failing (compose --wait-timeout)
        #[arg(long, default_value_t = 120, requires = "wait", value_name = "SECS")]
        wait_timeout: u32,
        /// Print the docker invocations without executing anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Take down a project (no dependency cascade)
    Down {
        /// The project to take down
        project: String,
        /// Print the docker invocations without executing anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Bounce a project: down then up, no dependency cascade either way
    Restart {
        /// The project to restart
        project: String,
        /// Stay attached instead of passing --detach to the up leg
        #[arg(long)]
        no_detach: bool,
        /// Print the docker invocations without executing anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Pull a project's images (no dependency cascade)
    Pull {
        /// The project to pull
        project: String,
        /// Print the docker invocations without executing anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Stream a project's compose logs (no dependency cascade)
    Logs {
        /// The project whose logs to show
        project: String,
        /// Keep streaming new log output until interrupted
        #[arg(long)]
        follow: bool,
    },
    /// Run a lifecycle verb over every running project
    All {
        #[command(subcommand)]
        verb: AllVerb,
    },
    /// List every project with its direct dependencies
    List,
    /// Query a project's dependency edges in either direction
    ///
    /// Direction: by default `deps` reports the project's dependencies (the
    /// edges in implicitDependencies); `--reverse` reports its dependents (what
    /// depends on it). Depth: by default only direct edges are shown;
    /// `--transitive` expands to the full closure in the chosen direction. The
    /// queried project is excluded; results are deduplicated and sorted.
    ///
    /// `--json` emits a stable object — {"project","direction","transitive",
    /// "deps"} — for machine consumers; the default is a human `list`-style
    /// line. A leaf yields an empty result and exit 0; an unknown project errors.
    Deps {
        /// The project whose edges to report
        project: String,
        /// Report dependents (what depends on the project) instead of dependencies
        #[arg(long)]
        reverse: bool,
        /// Expand to the full transitive closure instead of direct edges only
        #[arg(long)]
        transitive: bool,
        /// Emit a stable machine-readable JSON object instead of a text line
        #[arg(long)]
        json: bool,
    },
    /// List the repository's running projects with their compose status
    Ps,
    /// Scaffold a new app under apps/ from the built-in templates
    New {
        /// The project name, in any casing (normalized to kebab-case)
        name: String,
    },
    /// Create the variables of a project and its dependencies
    CreateVariables {
        /// The project whose variable cascade to create
        project: String,
        /// Never prompt; take computed defaults (CI mode)
        #[arg(long)]
        use_defaults: bool,
    },
    /// Create the missing volumes of a project and its dependencies
    CreateVolumes {
        /// The project whose volume cascade to create
        project: String,
        /// Remove and recreate volumes that already exist
        #[arg(long)]
        force: bool,
    },
    /// Create the missing networks of a project and its dependencies
    CreateNetworks {
        /// The project whose network cascade to create
        project: String,
    },
    /// Archive a project's own volumes to compressed backup files
    BackupVolumes {
        /// The project whose volumes to back up
        project: String,
        /// Where the archives go (default .backups; relative to the repo root)
        #[arg(long)]
        target_path: Option<std::path::PathBuf>,
    },
    /// Refill a project's own volumes from compressed backup files
    RestoreVolumes {
        /// The project whose volumes to restore
        project: String,
        /// Where the archives come from (default .backups; relative to the repo root)
        #[arg(long)]
        source_path: Option<std::path::PathBuf>,
        /// Restore into non-empty volumes, overwriting their contents
        #[arg(long)]
        force: bool,
    },
    /// Provision a project: variables, volumes, and networks in one go
    Configure {
        /// The project to provision
        project: String,
        /// Never prompt; take computed defaults (CI mode)
        #[arg(long)]
        use_defaults: bool,
    },
    /// Print a shell completion script (source it to enable TAB completion)
    Completions {
        /// The shell to generate the script for
        #[arg(value_enum)]
        shell: ShellArg,
    },
    /// The completer the emitted scripts call at TAB-time; hidden
    /// because it is machinery, not a user command. Prints the candidates for
    /// the words typed so far, one per line.
    #[command(name = "__complete", hide = true)]
    Complete {
        /// The words after `tug`, the current (possibly empty) word last.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        words: Vec<String>,
    },
}

/// The shell argument of `completions`; a `clap` enum so an unknown shell is
/// rejected with the valid list, mapped to the clap-free [`completion::Shell`].
#[derive(Clone, Copy, clap::ValueEnum)]
enum ShellArg {
    Bash,
    Zsh,
    Fish,
}

impl From<ShellArg> for completion::Shell {
    fn from(shell: ShellArg) -> Self {
        match shell {
            ShellArg::Bash => Self::Bash,
            ShellArg::Zsh => Self::Zsh,
            ShellArg::Fish => Self::Fish,
        }
    }
}

/// The verbs `tug all` fans out over the running set.
#[derive(Subcommand)]
enum AllVerb {
    /// Take down every running project (dependents first)
    Down,
    /// Bounce every running project: down then up (dependencies first)
    Restart,
    /// Pull every running project's images (dependencies first)
    Pull,
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::exit(1);
        }
    }
}

// A flat one-arm-per-subcommand dispatcher: its length tracks the CLI
// surface, not complexity.
#[allow(clippy::too_many_lines)]
fn run() -> Result<i32> {
    let cli = Cli::parse();

    // Completion runs inside the user's shell, so it must never hard-fail on
    // a missing repo — it is handled before the mandatory discovery below.
    match &cli.command {
        Command::Completions { shell } => {
            print!("{}", completion::script((*shell).into()));
            return Ok(0);
        }
        Command::Complete { words } => return Ok(complete(words)),
        _ => {}
    }

    let process_env: BTreeMap<String, String> = std::env::vars().collect();
    let cwd = std::env::current_dir().context("failed to resolve the working directory")?;
    let repo = Repo::discover(&cwd)?;
    match cli.command {
        Command::Up {
            project,
            no_deps,
            force,
            no_detach,
            wait,
            wait_timeout,
            dry_run,
        } => {
            let options = UpOptions {
                force,
                detach: !no_detach,
                // `--wait-timeout` defaults to 120 and requires `--wait`, so a
                // timeout only counts when the user actually asked to wait.
                wait_timeout: wait.then_some(wait_timeout),
                no_deps,
            };
            let mut runner = runner_for(dry_run);
            commands::up(&repo, &process_env, &project, &options, runner.as_mut())
        }
        Command::Down { project, dry_run } => {
            let mut runner = runner_for(dry_run);
            commands::down(&repo, &process_env, &project, runner.as_mut())
        }
        Command::Restart {
            project,
            no_detach,
            dry_run,
        } => {
            let options = RestartOptions { detach: !no_detach };
            let mut runner = runner_for(dry_run);
            commands::restart(&repo, &process_env, &project, &options, runner.as_mut())
        }
        Command::Pull { project, dry_run } => {
            let mut runner = runner_for(dry_run);
            commands::pull(&repo, &process_env, &project, runner.as_mut())
        }
        Command::Logs { project, follow } => {
            // No `--dry-run`, and `--follow` is long-running, so it goes
            // straight to the real spawner (like `ps`), never the dry runner.
            commands::logs(
                &repo,
                &process_env,
                &project,
                follow,
                &mut SystemRunner::new(),
            )
        }
        Command::All { verb } => {
            let verb = match verb {
                AllVerb::Down => commands::BulkVerb::Down,
                AllVerb::Restart => commands::BulkVerb::Restart,
                AllVerb::Pull => commands::BulkVerb::Pull,
            };
            // Needs docker for the running-set query and never previews, so
            // it goes straight to the real spawner (like `ps`/`logs`). The
            // summary joins the `+ …` progress on stderr; stdout stays clean.
            let (exit_code, summary) =
                commands::all(&repo, &process_env, verb, &mut SystemRunner::new())?;
            eprint!("{summary}");
            Ok(exit_code)
        }
        Command::List => {
            print!("{}", commands::list(&repo));
            Ok(0)
        }
        Command::Deps {
            project,
            reverse,
            transitive,
            json,
        } => {
            let options = DepsOptions {
                direction: if reverse {
                    Direction::Reverse
                } else {
                    Direction::Forward
                },
                depth: if transitive {
                    Depth::Transitive
                } else {
                    Depth::Direct
                },
                json,
            };
            print!("{}", commands::deps(&repo, &project, &options)?);
            Ok(0)
        }
        Command::Ps => {
            let (exit_code, listing) = commands::ps(&repo, &process_env, &mut SystemRunner::new())?;
            print!("{listing}");
            Ok(exit_code)
        }
        Command::New { name } => {
            scaffold::new_project(&repo, &name)?;
            Ok(0)
        }
        Command::CreateVariables {
            project,
            use_defaults,
        } => {
            vars::create_variables(
                &repo,
                &process_env,
                &project,
                use_defaults,
                &HostFacts::detect(),
                &mut TerminalConsole::new(),
            )?;
            Ok(0)
        }
        Command::CreateVolumes { project, force } => provision::create_volumes(
            &repo,
            &process_env,
            &project,
            force,
            &mut SystemRunner::new(),
        ),
        Command::CreateNetworks { project } => {
            provision::create_networks(&repo, &process_env, &project, &mut SystemRunner::new())
        }
        Command::BackupVolumes {
            project,
            target_path,
        } => backup::backup_volumes(
            &repo,
            &process_env,
            &project,
            target_path.as_deref(),
            &mut SystemRunner::new(),
        ),
        Command::RestoreVolumes {
            project,
            source_path,
            force,
        } => backup::restore_volumes(
            &repo,
            &process_env,
            &project,
            source_path.as_deref(),
            force,
            &mut SystemRunner::new(),
        ),
        Command::Configure {
            project,
            use_defaults,
        } => provision::configure(
            &repo,
            &process_env,
            &project,
            use_defaults,
            &HostFacts::detect(),
            &mut TerminalConsole::new(),
            &mut SystemRunner::new(),
        ),
        // Both are handled before discovery, above.
        Command::Completions { .. } | Command::Complete { .. } => unreachable!(),
    }
}

/// The hidden `__complete` handler: print the candidates for `words` (one per
/// line) off the clap-derived grammar and a leniently-discovered repo. Repo
/// discovery is best-effort so completion outside a repository stays silent
/// and successful rather than erroring into the user's shell.
fn complete(words: &[String]) -> i32 {
    let repo = std::env::current_dir()
        .ok()
        .and_then(|cwd| Repo::discover(&cwd).ok());
    let grammar = grammar();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    for candidate in completion::candidates(&grammar, repo.as_ref(), &words) {
        println!("{candidate}");
    }
    0
}

/// The CLI grammar the completer walks, read once from clap's own command
/// tree so it never drifts from the real surface. Hidden subcommands (the
/// `__complete` helper itself) are excluded.
fn grammar() -> Vec<completion::CommandGrammar> {
    Cli::command()
        .get_subcommands()
        .filter(|command| !command.is_hide_set())
        .map(command_grammar)
        .collect()
}

/// One clap subcommand, projected onto the completer's [`completion::CommandGrammar`]:
/// its name, whether it takes an existing `project`, its long flags (minus the
/// auto `--help`/`--version`), and any nested subcommands (`all`).
fn command_grammar(command: &clap::Command) -> completion::CommandGrammar {
    completion::CommandGrammar {
        name: command.get_name().to_owned(),
        takes_project: command
            .get_positionals()
            .any(|arg| arg.get_id().as_str() == "project"),
        flags: command
            .get_arguments()
            .filter_map(clap::Arg::get_long)
            .filter(|long| *long != "help" && *long != "version")
            .map(|long| format!("--{long}"))
            .collect(),
        subcommands: command
            .get_subcommands()
            .filter(|command| !command.is_hide_set())
            .map(command_grammar)
            .collect(),
    }
}

/// The seam adapter for this invocation: printing for `--dry-run`, the real
/// spawner otherwise.
fn runner_for(dry_run: bool) -> Box<dyn Runner> {
    if dry_run {
        Box::new(DryRunRunner::new(std::io::stdout()))
    } else {
        Box::new(SystemRunner::new())
    }
}
