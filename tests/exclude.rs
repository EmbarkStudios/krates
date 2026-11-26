use krates::PkgSpec;

#[test]
fn excludes_workspace_member() {
    let mut kb = krates::Builder::new();
    kb.exclude(std::iter::once("a".parse::<PkgSpec>().unwrap()));

    ktest::assert_dotgraph!("all-features.json", kb);
}

#[test]
fn includes_by_path() {
    let mut kb = krates::Builder::new();
    kb.include_workspace_crates(["/home/jake/code/krates/tests/ws/c"]);

    ktest::assert_dotgraph!("all-features.json", kb);
}

#[test]
fn excludes_dependencies() {
    let mut kb = krates::Builder::new();

    let pkg_ids = [
        "bitflags",
        "bumpalo:3.11.0",
        "https://github.com/rust-lang/crates.io-index#byteorder",
        "https://github.com/rust-lang/crates.io-index#ring:0.16.20",
        "https://github.com/alexcrichton/cc-rs#cc",
    ];

    kb.exclude(pkg_ids.iter().map(|id| id.parse::<PkgSpec>().unwrap()));

    ktest::assert_dotgraph!("all-features.json", kb);
}

/// Ensures that we properly fail to generate a graph when all of the workspace members are excluded
#[test]
fn no_roots() {
    let mut kb = krates::Builder::new();

    let pkg_ids = ["a", "b", "path+file:///home/jake/code/krates/tests/ws/c"];

    kb.exclude(pkg_ids.iter().map(|id| id.parse::<PkgSpec>().unwrap()));

    let contents = std::fs::read_to_string("tests/all-features.json")
        .map_err(|e| format!("failed to load metadata file: {}", e))
        .unwrap();

    let md: krates::cm::Metadata = serde_json::from_str(&contents)
        .map_err(|e| format!("failed to deserialize metadata: {}", e))
        .unwrap();

    let err = kb
        .build_with_metadata::<ktest::JustId, krates::Edge, _>(md, |_f: krates::cm::Package| {
            panic!("shouldn't get here")
        })
        .expect_err("expected build to fail");

    assert!(matches!(err, krates::Error::NoRootKrates), "{err:?}");
}

/// Ensures that we properly handle matching package specs when excluding crates
/// <https://github.com/EmbarkStudios/krates/issues/124>
#[test]
fn exclude_rename() {
    let mut kb = krates::Builder::new();

    kb.exclude(std::iter::once(
        "https://github.com/rust-lang/crates.io-index#libc"
            .parse()
            .unwrap(),
    ));

    let mut cmd = krates::Cmd::new();
    cmd.current_dir("tests/package-rename");

    let graph = kb
        .build::<ktest::JustId, krates::Edge, _>(cmd, |_f: krates::cm::Package| {})
        .expect("failed to build graph");

    // The local crate is also named libc, but the pkgspec indicated the source so we should have only our local package
    assert_eq!(graph.len(), 1);

    let mut kb = krates::Builder::new();

    // This will match the local crate which is the only root
    kb.exclude(std::iter::once("libc".parse().unwrap()));

    let mut cmd = krates::Cmd::new();
    cmd.current_dir("tests/package-rename");

    let err = kb
        .build::<ktest::JustId, krates::Edge, _>(cmd, |_f: krates::cm::Package| {})
        .expect_err("we shouldn't have any crates");

    assert!(matches!(err, krates::Error::NoRootKrates), "{err:?}");
}
