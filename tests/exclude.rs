mod util;

use krates::PkgSpec;
use util::build;

#[test]
fn excludes_workspace_member() {
    let mut kb = krates::Builder::new();
    kb.exclude(std::iter::once("a".parse::<PkgSpec>().unwrap()));

    let grafs = build("all-features.json", kb).unwrap();

    insta::assert_snapshot!(grafs.dotgraph());
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

    let grafs = build("all-features.json", kb).unwrap();
    insta::assert_snapshot!(grafs.dotgraph());
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
