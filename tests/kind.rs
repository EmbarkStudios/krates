mod util;

use util::build;

#[test]
fn all_the_things() {
    let grafs = build("all-features.json", krates::Builder::new()).unwrap();

    assert!(grafs.filtered.is_empty());
    insta::assert_snapshot!(grafs.dotgraph());
}

#[test]
fn filters_dev() {
    // Just non-workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::NonWorkspace);

        let grafs = build("all-features.json", kb).unwrap();

        // This shouldn't actually affect anything, as dev dependencies
        // for non-workspace crates are already not resolved
        assert!(grafs.filtered.is_empty());
        insta::assert_snapshot!(grafs.dotgraph());
    }

    // Just workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::Workspace);

        let grafs = build("all-features.json", kb).unwrap();
        insta::assert_snapshot!(grafs.dotgraph());
    }

    // Both
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::All);

        // This will be equivalent to to filtering workspace dev crates
        let grafs = build("all-features.json", kb).unwrap();
        insta::assert_snapshot!(grafs.dotgraph());
    }
}

#[test]
fn filters_build() {
    // Just non-workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::NonWorkspace);

        let grafs = build("all-features.json", kb).unwrap();
        insta::assert_snapshot!(grafs.dotgraph());
    }

    // Just workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::Workspace);

        let grafs = build("all-features.json", kb).unwrap();
        insta::assert_snapshot!(grafs.dotgraph());
    }

    // Both
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::All);

        let grafs = build("all-features.json", kb).unwrap();
        insta::assert_snapshot!(grafs.dotgraph());
    }
}

#[test]
fn filters_normal() {
    // Just non-workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Normal, krates::Scope::NonWorkspace);

        let grafs = build("all-features.json", kb).unwrap();
        insta::assert_snapshot!(grafs.dotgraph());
    }

    // Just workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Normal, krates::Scope::Workspace);

        let grafs = build("all-features.json", kb).unwrap();
        insta::assert_snapshot!(grafs.dotgraph());
    }

    // Both
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Normal, krates::Scope::All);

        let grafs = build("all-features.json", kb).unwrap();
        insta::assert_snapshot!(grafs.dotgraph());
    }
}

#[test]
fn filters_build_and_dev() {
    // Just non-workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::NonWorkspace);
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::NonWorkspace);

        let grafs = build("all-features.json", kb).unwrap();
        insta::assert_snapshot!(grafs.dotgraph());
    }

    // Just workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::Workspace);
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::Workspace);

        let grafs = build("all-features.json", kb).unwrap();
        insta::assert_snapshot!(grafs.dotgraph());
    }

    // Both
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::All);

        let grafs = build("all-features.json", kb).unwrap();
        insta::assert_snapshot!(grafs.dotgraph());
    }
}

#[test]
fn only_b() {
    let mut kb = krates::Builder::new();
    kb.include_workspace_crates(["/home/jake/code/krates/tests/ws/b/Cargo.toml"]);

    let grafs = build("all-features.json", kb).unwrap();
    insta::assert_snapshot!(grafs.dotgraph());
}

#[test]
fn filters_after_build() {
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::NonWorkspace);

        let grafs = build("all-features.json", kb).unwrap();

        let filtered = grafs.actual.krates_filtered(krates::DepKind::Dev);

        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::All);

        let grafs = build("all-features.json", kb).unwrap();

        let expected = format!("{:#?}", grafs.actual.krates().collect::<Vec<_>>());
        let actual = format!("{filtered:#?}");

        similar_asserts::assert_eq!(expected, actual);
    }

    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::NonWorkspace);

        let grafs = build("all-features.json", kb).unwrap();

        let filtered = grafs.actual.krates_filtered(krates::DepKind::Build);

        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::All);

        let grafs = build("all-features.json", kb).unwrap();

        let expected = format!("{:#?}", grafs.actual.krates().collect::<Vec<_>>());
        let actual = format!("{filtered:#?}");

        similar_asserts::assert_eq!(expected, actual);
    }
}
