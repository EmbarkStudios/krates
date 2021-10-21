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
    assert_eq!(win28.1.krate.name, "winapi");
    assert_eq!(
        win28.1.krate.version,
        krates::semver::Version::parse("0.2.8").unwrap()
    );

    let win38 = iter.next().unwrap();
    assert_eq!(win38.1.krate.name, "winapi");
    assert_eq!(
        win38.1.krate.version,
        krates::semver::Version::parse("0.3.8").unwrap()
    );

    assert!(iter.next().is_none());

    let mut iter = krates.krates_by_name("a");

    let a = iter.next().unwrap();
    assert_eq!(a.1.krate.name, "a");
    assert_eq!(
        a.1.krate.version,
        krates::semver::Version::parse("0.1.0").unwrap()
    );

    assert!(iter.next().is_none());

    let mut iter = krates.krates_by_name("winapi-x86_64-pc-windows-gnu");

    let wingnu = iter.next().unwrap();
    assert_eq!(wingnu.1.krate.name, "winapi-x86_64-pc-windows-gnu");
    assert_eq!(
        wingnu.1.krate.version,
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
        assert_eq!(win28.1.krate.name, "winapi");
        assert_eq!(
            win28.1.krate.version,
            krates::semver::Version::parse("0.2.8").unwrap()
        );

        let win38 = iter.next().unwrap();
        assert_eq!(win38.1.krate.name, "winapi");
        assert_eq!(
            win38.1.krate.version,
            krates::semver::Version::parse("0.3.8").unwrap()
        );

        assert!(iter.next().is_none());
    }

    {
        let two = krates::semver::VersionReq::parse("=0.2").unwrap();
        let mut iter = krates.search_matches("winapi", two);

        let win28 = iter.next().unwrap();
        assert_eq!(win28.1.krate.name, "winapi");
        assert_eq!(
            win28.1.krate.version,
            krates::semver::Version::parse("0.2.8").unwrap()
        );

        assert!(iter.next().is_none());
    }

    {
        let grtr = krates::semver::VersionReq::parse(">0.2.8").unwrap();
        let mut iter = krates.search_matches("winapi", grtr);

        let win38 = iter.next().unwrap();
        assert_eq!(win38.1.krate.name, "winapi");
        assert_eq!(
            win38.1.krate.version,
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
