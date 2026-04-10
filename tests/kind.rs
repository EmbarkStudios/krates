#[test]
fn all_the_things() {
    ktest::assert_dotgraph!(default "all-features.json");
}

mod filters_dev {
    #[test]
    fn all() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::All);

        // This will be equivalent to filtering workspace dev crates
        ktest::assert_dotgraph!("all-features.json", kb);
    }

    #[test]
    fn non_workspace() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::NonWorkspace);

        let grafs = ktest::util::build("all-features.json", kb).unwrap();

        // This shouldn't actually affect anything, as dev dependencies
        // for non-workspace crates are already not resolved
        assert!(grafs.filtered.is_empty());
        ktest::assert_snapshot!(grafs.dotgraph());
    }

    #[test]
    fn workspace() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::Workspace);
        ktest::assert_dotgraph!("all-features.json", kb);
    }
}

mod filters_build {
    #[test]
    fn all() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::All);

        // This will be equivalent to filtering workspace dev crates
        ktest::assert_dotgraph!("all-features.json", kb);
    }

    #[test]
    fn non_workspace() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::NonWorkspace);

        ktest::assert_dotgraph!("all-features.json", kb);
    }

    #[test]
    fn workspace() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::Workspace);
        ktest::assert_dotgraph!("all-features.json", kb);
    }
}

mod filters_normal {
    #[test]
    fn all() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Normal, krates::Scope::All);

        // This will be equivalent to filtering workspace dev crates
        ktest::assert_dotgraph!("all-features.json", kb);
    }

    #[test]
    fn non_workspace() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Normal, krates::Scope::NonWorkspace);

        ktest::assert_dotgraph!("all-features.json", kb);
    }

    #[test]
    fn workspace() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Normal, krates::Scope::Workspace);
        ktest::assert_dotgraph!("all-features.json", kb);
    }
}

mod filters_build_and_dev {
    #[test]
    fn all() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::All);
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::All);

        // This will be equivalent to filtering workspace dev crates
        ktest::assert_dotgraph!("all-features.json", kb);
    }

    #[test]
    fn non_workspace() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::NonWorkspace);
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::NonWorkspace);

        ktest::assert_dotgraph!("all-features.json", kb);
    }

    #[test]
    fn workspace() {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::Workspace);
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::Workspace);

        ktest::assert_dotgraph!("all-features.json", kb);
    }
}

#[test]
fn only_b() {
    let mut kb = krates::Builder::new();
    kb.include_workspace_crates(["/home/jake/code/krates/tests/ws/b/Cargo.toml"]);

    ktest::assert_dotgraph!("all-features.json", kb);
}

#[test]
fn filters_after_build() {
    use ktest::util::build;

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

        ktest::similar_asserts::assert_eq!(expected, actual);
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

        ktest::similar_asserts::assert_eq!(expected, actual);
    }
}
