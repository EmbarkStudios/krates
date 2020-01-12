#[macro_use]
mod util;

use util::{build, cmp};

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

        let mut filtered = [
            // c dev
            util::make_kid("difference 2.0.0"),
            // b dev
            util::make_kid("ring 0.16.9"),
            // unique dependency of ring
            //util::make_kid("untrusted 0.7.0"),
        ];

        //util::assert_filtered(&grafs.filtered, &mut filtered);
        cmp(
            grafs,
            |kid| filtered.contains(kid),
            |ef| util::is_workspace(ef.source) && ef.kind == krates::DepKind::Dev,
        );
    }

    // Both
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Dev, krates::Scope::Workspace);

        let grafs = build("all-features.json", kb).unwrap();

        let mut filtered = [
            // c dev
            util::make_kid("difference 2.0.0"),
            // b dev
            util::make_kid("ring 0.16.9"),
            // unique dependency of ring
            //util::make_kid("untrusted 0.7.0"),
        ];

        //util::assert_filtered(&grafs.filtered, &mut filtered);
        cmp(
            grafs,
            |kid| filtered.contains(kid),
            |ef| ef.kind == krates::DepKind::Dev,
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

        let mut filtered = [
            util::make_kid("bindgen 0.51.1"),
            util::make_kid("cc 1.0.50"),
            //util::make_kid("anyhow 1.0.26"),
            //util::make_kid("regex 1.3.3"),
            util::make_kid("wasm-bindgen-webidl 0.2.58"),
        ];

        //util::assert_filtered(&grafs.filtered, &mut filtered);
        cmp(grafs, |nid| filtered.contains(nid), |ef| false);
    }

    // Just workspace crates
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::Workspace);

        let grafs = build("all-features.json", kb).unwrap();

        let mut filtered = [
            // c dev
            //util::make_kid("difference 2.0.0"),
            // b dev
            //util::make_kid("ring 0.16.9"),
            // unique dependency of ring
            //util::make_kid("untrusted 0.7.0"),
        ];

        util::assert_filtered(&grafs.filtered, &mut filtered);
        cmp(
            grafs,
            |kid| filtered.contains(kid),
            |ef| util::is_workspace(ef.source) && ef.kind == krates::DepKind::Dev,
        );
    }

    // Both
    {
        let mut kb = krates::Builder::new();
        kb.ignore_kind(krates::DepKind::Build, krates::Scope::Workspace);

        let grafs = build("all-features.json", kb).unwrap();

        let mut filtered = [
            // c dev
            //util::make_kid("difference 2.0.0"),
            // b dev
            //util::make_kid("ring 0.16.9"),
            // unique dependency of ring
            //util::make_kid("untrusted 0.7.0"),
        ];

        util::assert_filtered(&grafs.filtered, &mut filtered);
        cmp(
            grafs,
            |kid| filtered.contains(kid),
            |ef| ef.kind == krates::DepKind::Dev,
        );
    }
}
