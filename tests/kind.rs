mod util;

use util::{build, cmp};

macro_rules! matches_kind {
    ($ef:expr, $($tail:tt)*) => {
        if let Some(kind) = $ef.dep.map(|d| d.kind) {
            matches!(kind, $($tail)*)
        } else {
            false
        }
    }
}

#[test]
fn all_the_things() {
    let grafs = build("all-features.json", krates::Builder::new()).unwrap();

    assert!(grafs.filtered.is_empty());
    cmp(grafs, |_| false, |_| false);
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
        cmp(grafs, |_| false, |_| false);
    }

    // Just workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::Workspace);

        let grafs = build("all-features.json", kb).unwrap();

        let filtered = [
            util::make_kid("difference 2.0.0"),
            util::make_kid("ring 0.16.9"),
        ];

        cmp(
            grafs,
            |kid| filtered.contains(kid),
            |ef| util::is_workspace(ef.source) && matches_kind!(ef, krates::DepKind::Dev),
        );
    }

    // Both
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::All);

        let grafs = build("all-features.json", kb).unwrap();
        cmp(
            grafs,
            |_| false,
            |ef| matches_kind!(ef, krates::DepKind::Dev),
        );
    }
}

#[test]
fn filters_build() {
    // Just non-workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::NonWorkspace);

        let grafs = build("all-features.json", kb).unwrap();

        cmp(
            grafs,
            |_| false,
            |ef| !util::is_workspace(ef.source) && matches_kind!(ef, krates::DepKind::Build),
        );
    }

    // Just workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::Workspace);

        let grafs = build("all-features.json", kb).unwrap();

        cmp(
            grafs,
            |_| false,
            |ef| util::is_workspace(ef.source) && matches_kind!(ef, krates::DepKind::Build),
        );
    }

    // Both
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::All);

        let grafs = build("all-features.json", kb).unwrap();

        cmp(
            grafs,
            |_| false,
            |ef| matches_kind!(ef, krates::DepKind::Build),
        );
    }
}

#[test]
fn filters_normal() {
    // Just non-workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Normal, krates::Scope::NonWorkspace);

        let grafs = build("all-features.json", kb).unwrap();

        cmp(
            grafs,
            |_| false,
            |ef| !util::is_workspace(ef.source) && matches_kind!(ef, krates::DepKind::Normal),
        );
    }

    // Just workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Normal, krates::Scope::Workspace);

        let grafs = build("all-features.json", kb).unwrap();

        cmp(
            grafs,
            |_| false,
            |ef| util::is_workspace(ef.source) && matches_kind!(ef, krates::DepKind::Normal),
        );
    }

    // Both
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Normal, krates::Scope::All);

        let grafs = build("all-features.json", kb).unwrap();

        cmp(
            grafs,
            |_| false,
            |ef| matches_kind!(ef, krates::DepKind::Normal),
        );
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

        cmp(
            grafs,
            |_| false,
            |ef| {
                if util::is_workspace(ef.source) {
                    return false;
                }

                matches_kind!(ef, krates::DepKind::Build | krates::DepKind::Dev)
            },
        );
    }

    // Just workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::Workspace);
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::Workspace);

        let grafs = build("all-features.json", kb).unwrap();

        cmp(
            grafs,
            |_| false,
            |ef| {
                if !util::is_workspace(ef.source) {
                    return false;
                }

                matches_kind!(ef, krates::DepKind::Build | krates::DepKind::Dev)
            },
        );
    }

    // Both
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::All);

        let grafs = build("all-features.json", kb).unwrap();

        cmp(
            grafs,
            |_| false,
            |ef| matches_kind!(ef, krates::DepKind::Build | krates::DepKind::Dev),
        );
    }
}

#[test]
fn only_b() {
    let mut kb = krates::Builder::new();
    kb.include_workspace_crates(&["/home/jake/code/krates/tests/ws/b/Cargo.toml"]);

    let grafs = build("all-features.json", kb).unwrap();

    cmp(grafs, |kid| kid.repr.starts_with("a "), |_| false);
}
