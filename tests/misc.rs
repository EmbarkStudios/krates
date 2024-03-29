mod util;

#[test]
fn iter_names() {
    let contents = std::fs::read_to_string("tests/all-features.json")
        .map_err(|e| format!("failed to load metadata file: {}", e))
        .unwrap();

    let md: krates::cm::Metadata = serde_json::from_str(&contents)
        .map_err(|e| format!("failed to deserialize metadata: {}", e))
        .unwrap();

    let krates: krates::Krates = krates::Builder::new()
        .build_with_metadata(md, |_| {})
        .unwrap();

    let mut iter = krates.krates_by_name("winapi");

    let win28 = iter.next().unwrap();
    assert_eq!(win28.krate.name, "winapi");
    assert_eq!(
        win28.krate.version,
        krates::semver::Version::parse("0.2.8").unwrap()
    );

    let win38 = iter.next().unwrap();
    assert_eq!(win38.krate.name, "winapi");
    assert_eq!(
        win38.krate.version,
        krates::semver::Version::parse("0.3.8").unwrap()
    );

    assert!(iter.next().is_none());

    let mut iter = krates.krates_by_name("a");

    let a = iter.next().unwrap();
    assert_eq!(a.krate.name, "a");
    assert_eq!(
        a.krate.version,
        krates::semver::Version::parse("0.1.0").unwrap()
    );

    assert!(iter.next().is_none());

    let mut iter = krates.krates_by_name("winapi-x86_64-pc-windows-gnu");

    let wingnu = iter.next().unwrap();
    assert_eq!(wingnu.krate.name, "winapi-x86_64-pc-windows-gnu");
    assert_eq!(
        wingnu.krate.version,
        krates::semver::Version::parse("0.4.0").unwrap()
    );

    assert!(iter.next().is_none());
}

#[test]
fn iter_matches() {
    let contents = std::fs::read_to_string("tests/all-features.json")
        .map_err(|e| format!("failed to load metadata file: {}", e))
        .unwrap();

    let md: krates::cm::Metadata = serde_json::from_str(&contents)
        .map_err(|e| format!("failed to deserialize metadata: {}", e))
        .unwrap();

    let krates: krates::Krates = krates::Builder::new()
        .build_with_metadata(md, |_| {})
        .unwrap();

    {
        let any = krates::semver::VersionReq::STAR;
        let mut iter = krates.search_matches("winapi", any);

        let win28 = iter.next().unwrap();
        assert_eq!(win28.krate.name, "winapi");
        assert_eq!(
            win28.krate.version,
            krates::semver::Version::parse("0.2.8").unwrap()
        );

        let win38 = iter.next().unwrap();
        assert_eq!(win38.krate.name, "winapi");
        assert_eq!(
            win38.krate.version,
            krates::semver::Version::parse("0.3.8").unwrap()
        );

        assert!(iter.next().is_none());
    }

    {
        let two = krates::semver::VersionReq::parse("=0.2").unwrap();
        let mut iter = krates.search_matches("winapi", two);

        let win28 = iter.next().unwrap();
        assert_eq!(win28.krate.name, "winapi");
        assert_eq!(
            win28.krate.version,
            krates::semver::Version::parse("0.2.8").unwrap()
        );

        assert!(iter.next().is_none());
    }

    {
        let grtr = krates::semver::VersionReq::parse(">0.2.8").unwrap();
        let mut iter = krates.search_matches("winapi", grtr);

        let win38 = iter.next().unwrap();
        assert_eq!(win38.krate.name, "winapi");
        assert_eq!(
            win38.krate.version,
            krates::semver::Version::parse("0.3.8").unwrap()
        );

        assert!(iter.next().is_none());
    }

    {
        let none = krates::semver::VersionReq::parse("=0.4").unwrap();
        let mut iter = krates.search_matches("winapi", none);

        assert!(iter.next().is_none());
    }
}

#[test]
fn direct_dependents() {
    let mut kb = krates::Builder::new();
    kb.include_targets(std::iter::once((
        krates::cfg_expr::targets::get_builtin_target_by_triple("x86_64-unknown-linux-gnu")
            .unwrap()
            .triple
            .clone(),
        vec![],
    )));

    let grafs = util::build("direct.json", kb).unwrap();

    let id = grafs
        .actual
        .krates()
        .find(|k| k.0.repr.starts_with("reqwest"))
        .unwrap();

    let mut ids: Vec<_> = grafs
        .actual
        .direct_dependents(grafs.actual.nid_for_kid(&id.0).unwrap())
        .into_iter()
        .map(|jid| jid.krate.0.repr.as_str())
        .collect();

    ids.sort();
    let dd = ids.join("\n");

    insta::assert_snapshot!(dd);
}

#[test]
fn direct_dependencies() {
    let mut kb = krates::Builder::new();
    kb.include_targets(std::iter::once((
        krates::cfg_expr::targets::get_builtin_target_by_triple("x86_64-unknown-linux-gnu")
            .unwrap()
            .triple
            .clone(),
        vec![],
    )));

    let grafs = util::build("direct.json", kb).unwrap();

    let id = grafs
        .actual
        .krates()
        .find(|k| k.0.repr.starts_with("reqwest"))
        .unwrap();

    let mut ids: Vec<_> = grafs
        .actual
        .direct_dependencies(grafs.actual.nid_for_kid(&id.0).unwrap())
        .into_iter()
        .map(|jid| jid.krate.0.repr.as_str())
        .collect();

    ids.sort();
    let dd = ids.join("\n");

    insta::assert_snapshot!(dd);
}

/// Validates that there is no difference between the OG "opaque" package id
/// format and the newly stabilized one
#[test]
fn opaque_matches_stable() {
    let opaque = util::build("all-features.json", krates::Builder::new()).unwrap();
    let stable = util::build("all-features-stable.json", krates::Builder::new()).unwrap();

    similar_asserts::assert_eq!(opaque.dotgraph(), stable.dotgraph());
}

/// Validates we can correctly find package ids for duplicated packages in both
/// the opaque and stable formats
///
/// <https://github.com/EmbarkStudios/krates/issues/68>
/// <https://github.com/EmbarkStudios/krates/issues/69>
#[test]
fn finds_duplicates() {
    let opaque = util::build("pid-opaque.json", krates::Builder::new()).unwrap();
    let stable = util::build("pid-stable.json", krates::Builder::new()).unwrap();

    let opaque = opaque.dotgraph();
    similar_asserts::assert_eq!(opaque, stable.dotgraph());

    insta::assert_snapshot!(opaque);
}
