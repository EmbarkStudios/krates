mod util;

use krates::cfg_expr::targets;
use util::build;

#[test]
fn ignores_non_linux() {
    let mut kb = krates::Builder::new();
    kb.include_targets(targets::ALL_BUILTINS.iter().filter_map(|ti| {
        if ti.os == Some(targets::Os::linux) {
            Some((ti.triple.clone(), Vec::new()))
        } else {
            None
        }
    }));

    let grafs = build("all-features.json", kb).unwrap();

    insta::assert_snapshot!(grafs.dotgraph());
}

#[test]
fn ignores_non_tier1() {
    let mut kb = krates::Builder::new();

    let targets = vec![
        targets::get_builtin_target_by_triple("i686-pc-windows-gnu").unwrap(),
        targets::get_builtin_target_by_triple("i686-pc-windows-msvc").unwrap(),
        targets::get_builtin_target_by_triple("i686-unknown-linux-gnu").unwrap(),
        targets::get_builtin_target_by_triple("x86_64-apple-darwin").unwrap(),
        targets::get_builtin_target_by_triple("x86_64-pc-windows-gnu").unwrap(),
        targets::get_builtin_target_by_triple("x86_64-pc-windows-msvc").unwrap(),
        targets::get_builtin_target_by_triple("x86_64-unknown-linux-gnu").unwrap(),
    ];

    kb.include_targets(targets.iter().map(|ti| (ti.triple.clone(), vec![])));

    let grafs = build("all-features.json", kb).unwrap();

    insta::assert_snapshot!(grafs.dotgraph());
}

#[test]
fn ignores_non_wasm() {
    let mut kb = krates::Builder::new();

    let targets = vec![targets::get_builtin_target_by_triple("wasm32-unknown-unknown").unwrap()];

    kb.include_targets(targets.iter().map(|ti| (ti.triple.clone(), vec![])));

    let grafs = build("all-features.json", kb).unwrap();

    insta::assert_snapshot!(grafs.dotgraph());
}

#[cfg(feature = "targets")]
#[test]
fn handles_non_builtin() {
    let mut kb = krates::Builder::new();

    kb.include_targets(std::iter::once(("x86_64-xboxone-windows-msvc", vec![])));

    let grafs = build("all-features.json", kb).unwrap();
    insta::assert_snapshot!(grafs.dotgraph());
}
