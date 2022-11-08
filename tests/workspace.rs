mod util;

use util::build;

#[test]
fn includes() {
    let mut kb = krates::Builder::new();

    kb.include_workspace_crates([
        "/home/jake/code/krates/tests/ws2/b",
        "/home/jake/code/krates/tests/ws2/c/Cargo.toml",
    ]);

    let grafs = build("all-features2.json", kb).unwrap();
    insta::assert_snapshot!(grafs.dotgraph());
}

#[test]
fn root() {
    // The ws2 workspace has a top level crate that is also a virtual manifest,
    // so it will have a resolution root, which will be used instead of the
    // list of workspace members, and since it doesn't depend on any of the
    // others in the workspace, it will be a graph of one
    let kb = krates::Builder::new();

    let grafs = build("all-features2.json", kb).unwrap();

    assert_eq!(grafs.actual.len(), 1);
    insta::assert_snapshot!(grafs.dotgraph());
}

#[test]
fn workspace_with_root() {
    let mut kb = krates::Builder::new();
    // Setting the workspace true flag will mean to include all of the workspace
    // members, regardless of whether the resolution root is set or not
    kb.workspace(true);

    let grafs = build("all-features2.json", kb).unwrap();
    insta::assert_snapshot!(grafs.dotgraph());
}

#[test]
fn workspace_with_root_exclude() {
    let mut kb = krates::Builder::new();
    kb.workspace(true);
    kb.exclude(std::iter::once("c".parse::<krates::PkgSpec>().unwrap()));

    let grafs = build("all-features2.json", kb).unwrap();
    insta::assert_snapshot!(grafs.dotgraph());
}
