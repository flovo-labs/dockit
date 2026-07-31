# Test fixtures

Each directory here is a **fixture repo**: a fake repository tree (`project.json`
files, compose files, dotenv files) that tests run the CLI against. Tests never
depend on `../docker-host` or on real docker state — external effects stay behind
the process-spawn seam (`tug::process::Runner`), where a `RecordingRunner`
captures the argv sequence instead of spawning anything.

Conventions:

- One directory per scenario: `tests/fixtures/<name>/` (e.g. `simple-chain/`,
  `dependency-cycle/`). Keep fixtures minimal — only the files the scenario needs.
- Git cannot track a nested `.git` directory, so fixtures don't contain the repo
  root marker. A test helper copies the fixture into a scratch directory under
  `std::env::temp_dir()` and creates an empty `.git/` there, giving repo-root
  detection something real to find.
- A test points the CLI at a fixture by passing the scratch path as the working
  directory to the library entry point — the CLI never reads the process-global
  cwd outside `main`, so tests stay parallel-safe.
