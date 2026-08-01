//! `tug` — orchestration on top of plain `docker compose`.
//!
//! Module layout:
//!
//! - [`backup`] — the backup commands: `backup-volumes`/`restore-volumes`,
//!   archives wired natively through child stdio.
//! - [`commands`] — the lifecycle commands: `up`/`down`/`pull`/`list`/`deps`,
//!   built on the seam so `--dry-run` and tests are adapters.
//! - [`completion`] — shell completion: the `tug __complete` grammar-walker
//!   and the `tug completions <shell>` script generator.
//! - [`env`] — the environment model: env name, compose file layering,
//!   dotenv merge, persistence files.
//! - [`process`] — the process-spawn seam: every external `docker` invocation
//!   flows through it as data.
//! - [`provision`] — the provisioning commands: `create-volumes`,
//!   `create-networks`, and the `configure` orchestrator.
//! - [`repo`] — the repository model: root detection, project discovery,
//!   `project.json` parsing.
//! - [`ignore`] — the `.tugignore` pattern list: which directories discovery
//!   never descends into.
//! - [`graph`] — the dependency graph: transitive closure, cycle detection,
//!   deterministic ordering.
//! - [`scaffold`] — the `new` command: casing normalization and scaffolding
//!   from the embedded templates.
//! - [`vars`] — the `create-variables` command: five variable types,
//!   prompting through the line-IO boundary, persistence.

pub mod backup;
pub mod commands;
pub mod completion;
pub mod env;
pub mod graph;
pub mod ignore;
pub mod process;
pub mod provision;
pub mod repo;
pub mod scaffold;
pub mod vars;
