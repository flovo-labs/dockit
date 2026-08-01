//! The `.tugignore` pattern list: what each line means and which directories
//! it matches. Paths are always relative to the repo root.

use std::path::Path;

use tug::ignore::IgnoreList;

#[test]
fn a_bare_name_ignores_that_directory_at_any_depth() {
    // A line without a slash names a directory, not a place: it matches
    // wherever that name appears, which is what makes one `node_modules`
    // line enough for a repository that has several.
    let list = IgnoreList::parse("node_modules\n");
    assert!(list.ignores(Path::new("node_modules")));
    assert!(list.ignores(Path::new("apps/whoami/node_modules")));
}

#[test]
fn a_commented_out_pattern_ignores_nothing() {
    // Commenting a line out is how a pattern is disabled without deleting it.
    // The `#archive` line is the case that actually bites: a comment is not a
    // pattern even when its text would match a directory verbatim.
    let list = IgnoreList::parse("# node_modules\n#archive\nlibs\n");
    assert!(!list.ignores(Path::new("node_modules")));
    assert!(!list.ignores(Path::new("#archive")));
    assert!(list.ignores(Path::new("libs")));
}

#[test]
fn a_hash_later_in_a_line_is_an_ordinary_character() {
    // Only a line that opens with `#` is a comment, exactly as in .gitignore.
    // Stripping inline comments instead would leave no way to name a
    // directory whose name genuinely contains a hash.
    let list = IgnoreList::parse("archive # retired\n");
    assert!(!list.ignores(Path::new("archive")));
    assert!(list.ignores(Path::new("archive # retired")));
}

#[test]
fn surrounding_whitespace_is_not_part_of_a_pattern() {
    // Trailing spaces are invisible in an editor; a pattern that silently
    // stopped matching because of one would be near-impossible to diagnose.
    let list = IgnoreList::parse("  archive  \n");
    assert!(list.ignores(Path::new("archive")));
}

#[test]
fn a_pattern_with_a_slash_names_one_place_not_a_name() {
    // The counterpart to the bare name: once a pattern says where, it stops
    // matching the same directory name elsewhere in the repository.
    let list = IgnoreList::parse("apps/whoami/vendor\n");
    assert!(list.ignores(Path::new("apps/whoami/vendor")));
    assert!(!list.ignores(Path::new("libs/base/vendor")));
}

#[test]
fn a_leading_slash_anchors_a_root_level_directory() {
    // Without it there is no way to say "the archive at the root, and only
    // that one" — a bare name always spreads to every depth.
    let list = IgnoreList::parse("/archive\n");
    assert!(list.ignores(Path::new("archive")));
    assert!(!list.ignores(Path::new("apps/whoami/archive")));
}

#[test]
fn a_trailing_slash_marks_a_directory_without_anchoring_it() {
    // Only directories are ever matched, so a trailing slash restates what is
    // already true. Letting it change the meaning would make `archive/` and
    // `archive` two different patterns for no reason a reader could guess.
    let list = IgnoreList::parse("archive/\n");
    assert!(list.ignores(Path::new("archive")));
    assert!(list.ignores(Path::new("apps/whoami/archive")));
}
