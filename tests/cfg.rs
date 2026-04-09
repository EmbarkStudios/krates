use krates::cfg_expr::targets;

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

    ktest::assert_dotgraph!("all-features.json", kb);
}

#[test]
fn ignores_non_tier1() {
    let mut kb = krates::Builder::new();

    let targets = [
        "i686-pc-windows-gnu",
        "i686-pc-windows-msvc",
        "i686-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
    ];

    kb.include_targets(targets.iter().map(|ti| {
        (
            targets::get_builtin_target_by_triple(ti)
                .unwrap()
                .triple
                .clone(),
            vec![],
        )
    }));

    ktest::assert_dotgraph!("all-features.json", kb);
}

#[test]
fn ignores_non_wasm() {
    let mut kb = krates::Builder::new();

    kb.include_targets(std::iter::once((
        targets::get_builtin_target_by_triple("wasm32-unknown-unknown")
            .unwrap()
            .triple
            .clone(),
        vec![],
    )));

    ktest::assert_dotgraph!("all-features.json", kb);
}

#[cfg(feature = "targets")]
#[test]
fn handles_non_builtin() {
    let mut kb = krates::Builder::new();

    kb.include_targets(std::iter::once(("x86_64-xboxone-windows-msvc", vec![])));

    ktest::assert_dotgraph!("all-features.json", kb);
}
