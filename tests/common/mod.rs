//! Shared helpers for integration tests.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);

/// Copy `tests/fixtures/<name>` into a fresh scratch directory and create the
/// `.git/` repo-root marker there (fixtures cannot commit one — see the
/// fixtures README). Each call gets its own copy, keeping tests parallel-safe.
pub fn fixture(name: &str) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
    let target =
        std::env::temp_dir().join(format!("dockit-fixture-{}-{id}-{name}", std::process::id()));
    if target.exists() {
        std::fs::remove_dir_all(&target).unwrap();
    }
    copy_tree(&source, &target);
    std::fs::create_dir_all(target.join(".git")).unwrap();
    target
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let dest = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), &dest).unwrap();
        }
    }
}
