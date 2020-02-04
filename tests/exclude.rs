mod util;

use krates::PkgSpec;
use util::{build, cmp};

#[test]
fn excludes_workspace_member() {
    let mut kb = krates::Builder::new();
    kb.exclude(std::iter::once("a".parse::<PkgSpec>().unwrap()));

    let grafs = build("all-features.json", kb).unwrap();

    cmp(
        grafs,
        |pkg| pkg.repr == "a 0.1.0 (path+file:///home/jake/code/krates/tests/ws/a)",
        |_| false,
    );
}

#[test]
fn excludes_dependencies() {
    let mut kb = krates::Builder::new();

    // To ease testing, we just remove leaves that have no dependencies
    // themselves
    let pkg_ids = [
        "bitflags",
        "bumpalo:3.1.2",
        "https://github.com/rust-lang/crates.io-index#byteorder",
        "https://github.com/rust-lang/crates.io-index#anyhow:1.0.26",
        "https://github.com/alexcrichton/cc-rs#cc",
    ];

    kb.exclude(pkg_ids.iter().map(|id| id.parse::<PkgSpec>().unwrap()));

    let grafs = build("all-features.json", kb).unwrap();

    cmp(
        grafs,
        |pkg| {
            let ids = [
                "anyhow 1.0.26 (registry+https://github.com/rust-lang/crates.io-index)",
                "bitflags 1.2.1 (registry+https://github.com/rust-lang/crates.io-index)",
                "bumpalo 3.1.2 (registry+https://github.com/rust-lang/crates.io-index)",
                "byteorder 1.3.2 (registry+https://github.com/rust-lang/crates.io-index)",
                "cc 1.0.50 (git+https://github.com/alexcrichton/cc-rs#1d82f457ad4f60f7545eaaa673956806eca35d78)",
            ];

            ids.contains(&pkg.repr.as_str())
        },
        |_| false,
    );
}

#[test]
fn no_roots() {
    let mut kb = krates::Builder::new();

    // To ease testing, we just remove leaves that have no dependencies
    // themselves
    let pkg_ids = ["a", "b", "c"];

    kb.exclude(pkg_ids.iter().map(|id| id.parse::<PkgSpec>().unwrap()));

    let contents = std::fs::read_to_string("tests/all-features.json")
        .map_err(|e| format!("failed to load metadata file: {}", e))
        .unwrap();

    let md: krates::cm::Metadata = serde_json::from_str(&contents)
        .map_err(|e| format!("failed to deserialize metadata: {}", e))
        .unwrap();

    match kb.build_with_metadata::<util::JustId, krates::Edge, _>(md, |_f: krates::cm::Package| {
        panic!("shouldn't get here")
    }) {
        Err(krates::Error::NoRootKrates) => {}
        _ => panic!("expected no root crates error!"),
    }
}
