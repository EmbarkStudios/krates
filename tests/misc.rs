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

#[test]
#[cfg(all(feature = "serialize", not(feature = "metadata")))]
fn roundtrip() {
    let contents = std::fs::read_to_string("tests/all-features.json").unwrap();
    let md: krates::cm::Metadata = serde_json::from_str(&contents).unwrap();
    insta::assert_json_snapshot!(md);
}

/// Tests that manifest deserialization ignores unknown fields from eg. unstable features
#[test]
fn ignores_unknown_fields() {
    use serde_json::json;

    let json = json!({
        "packages": [
            json!({
                "name": "fake",
                "version": "1.0.9",
                "authors": ["boop"],
                "id": "registry+https://github.com/rust-lang/crates.io-index#fake@1.0.9",
                "source": "registry+https://github.com/rust-lang/crates.io-index",
                "description": "fake",
                "__extra__": null,
                "dependencies": [
                    json!({
                        "name": "dep",
                        "source": "registry+https://github.com/rust-lang/crates.io-index",
                        "req": "^1.0",
                        "kind": null,
                        "rename": null,
                        "optional": true,
                        "uses_default_features": true,
                        "features": ["feature"],
                        "target": null,
                        "__extra__": json!({"a":"b"}),
                        "registry": null,
                    })
                ],
                "license": "MIT OR Apache-2.0",
                "license_file": null,
                "targets": [
                    json!({
                        "kind": ["lib"],
                        "crate_types": ["lib"],
                        "name": "fake",
                        "src_path": "/home/jake/.cargo/registry/src/index.crates.io-6f17d22bba15001f/fake-1.0.9/src/lib.rs",
                        "edition": "2024",
                        "__extra__": ["a", "b"],
                        "doc": true,
                        "doctest": true,
                        "test": true
                    })
                ],
                "features": json!({
                    "a": ["b"],
                    "b": []
                }),
                "manifest_path": "path",
                "categories": ["one", "two"],
                "keywords": ["key1", "key2"],
                "readme": "README.md",
                "repository": "https://github.com/fake/fake",
                "homepage": "https://github.com/fake/fake",
                "documentation": "https://docs.rs/fake",
                "edition": "2024",
                "metadata": json!({
                    "docs": json!({
                        "rs": json!({
                            "features": ["b"]
                        })
                    })
                }),
                "links": null,
                "publish": null,
                "default_run": null,
                "rust_version": "1.85.0"
            }),
        ],
        "__extra__": "",
        "workspace_members": ["path+file:///home/jake/code/fake/fake#1.0.9"],
        "workspace_default_members": ["path+file:///home/jake/code/fake/fake#1.0.9"],
        "resolve": json!({
            "__extra__": -99999999999999999i64,
            "nodes": [
                json!({
                    "id": "registry+https://github.com/rust-lang/crates.io-index#bitflags@2.4.2",
                    "dependencies": ["git+https://github.com/madsmtm/objc2?rev=65de002#objc-sys@0.2.0-beta.2"],
                    "__extra__": true,
                    "deps": [
                        json!({
                            "name": "objc_sys",
                            "pkg": "git+https://github.com/madsmtm/objc2?rev=65de002#objc-sys@0.2.0-beta.2",
                            "dep_kinds": [
                                json!({
                                    "kind": null,
                                    "target":null,
                                    "__extra__": 1,
                                })
                            ]
                        })
                    ],
                    "features": ["alloc","apple","std"]
                })
            ],
            "root": "path+file:///home/jake/code/krates/tests/pid#0.1.0"
        }),
        "target_directory": "/home/jake/code/krates/tests/pid/target",
        "version": 1,
        "workspace_root": "/home/jake/code/krates/tests/pid",
        "metadata": null
    });

    let _cm = serde_json::from_str::<krates::cm::Metadata>(&json.to_string()).unwrap();
}
