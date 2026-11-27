//! `Krates::krates_filtered` must give the same crates as a graph built with
//! `ignore_kind` for the same kind, including when the only path to an
//! optional dependency is a feature that only a filtered edge enables.
//! <https://github.com/EmbarkStudios/cargo-deny/issues/919>

fn names(krates: impl IntoIterator<Item = krates::Kid>) -> Vec<String> {
    let mut names: Vec<_> = krates
        .into_iter()
        .map(|kid| format!("{} {}", kid.name(), kid.version()))
        .collect();
    names.sort();
    names
}

/// Returns (`krates_filtered(Dev)` on the full graph, the graph built with
/// dev dependencies ignored)
fn filtered_and_expected(case: &str) -> (Vec<String>, Vec<String>) {
    let manifest = format!("tests/dev-features/{case}/app/Cargo.toml");

    let build = |kb: krates::Builder| -> krates::Krates {
        let mut cmd = krates::Cmd::new();
        cmd.manifest_path(&manifest);
        kb.build(cmd, krates::NoneFilter).unwrap()
    };

    let full = build(krates::Builder::new());
    let filtered = names(
        full.krates_filtered(krates::DepKind::Dev)
            .into_iter()
            .map(|k| k.id.clone().into()),
    );

    let mut kb = krates::Builder::new();
    kb.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
    let expected = names(build(kb).krates().map(|k| k.id.clone().into()));

    (filtered, expected)
}

macro_rules! case {
    ($name:ident, $dir:literal, [$($krate:literal),* $(,)?]) => {
        #[test]
        fn $name() {
            let (filtered, expected) = filtered_and_expected($dir);
            let want: Vec<String> = vec![$($krate.to_owned()),*];
            assert_eq!(expected, want, "ignore_kind graph changed");
            assert_eq!(filtered, expected);
        }
    };
}

// A feature only a dev-dependency enables is the only path to `dup 2.0.0`
case!(
    drops_optional_dep_enabled_only_by_dev,
    "only-dev",
    ["app 0.1.0", "dup 1.0.0", "liba 0.1.0"]
);
// The same optional dependency enabled by a default feature stays
case!(
    keeps_optional_dep_enabled_by_default_feature,
    "default",
    ["app 0.1.0", "dup 2.0.0", "liba 0.1.0"]
);
// A weak `dup?/x` on a normal edge does not keep a dep only dev enables
case!(
    weak_feature_does_not_enable_dep,
    "weak",
    ["app 0.1.0", "liba 0.1.0"]
);
// A weak `dup?/x` applies once a normal edge enables `dup`
case!(
    weak_feature_applies_when_dep_enabled,
    "weak-enabled",
    ["app 0.1.0", "dup 2.0.0", "leaf 0.1.0", "liba 0.1.0"]
);
// A strong `dup/x` on a normal edge enables the dep and its feature
case!(
    strong_feature_enables_dep,
    "strong",
    ["app 0.1.0", "dup 2.0.0", "leaf 0.1.0", "liba 0.1.0"]
);
// A dev-only feature that reaches an optional dep of a normal dependency
case!(
    drops_optional_dep_behind_dev_feature_chain,
    "chain",
    ["app 0.1.0", "dup 2.0.0", "liba 0.1.0"]
);

// An optional dep declared with features, enabled through a second path after
// the crate declaring it was already visited, keeps those features
case!(
    late_enabled_optional_dep_keeps_declared_features,
    "late-enable",
    [
        "app 0.1.0",
        "dup 2.0.0",
        "leaf 0.1.0",
        "liba 0.1.0",
        "libb 0.1.0"
    ]
);
case!(
    late_enabled_optional_dep_keeps_declared_features_other_order,
    "late-enable-2",
    [
        "aaa 0.1.0",
        "app 0.1.0",
        "dup 2.0.0",
        "leaf 0.1.0",
        "liba 0.1.0"
    ]
);
/// `krates_filtered` matches `ignore_kind` for dev and build dependencies on
/// the checked in metadata fixtures
#[test]
fn matches_ignore_kind_on_metadata_fixtures() {
    for fixture in [
        "all-features.json",
        "all-features2.json",
        "all-features-stable.json",
    ] {
        for kind in [krates::DepKind::Dev, krates::DepKind::Build] {
            let full = ktest::util::build(fixture, krates::Builder::new()).unwrap();
            let filtered = names(
                full.actual
                    .krates_filtered(kind)
                    .into_iter()
                    .map(|k| k.0.clone()),
            );

            let mut kb = krates::Builder::new();
            kb.ignore_kind(kind, krates::Scope::All);
            let expected = names(
                ktest::util::build(fixture, kb)
                    .unwrap()
                    .actual
                    .krates()
                    .map(|k| k.0.clone()),
            );

            assert_eq!(filtered, expected, "{fixture} {kind:?}");
        }
    }
}
