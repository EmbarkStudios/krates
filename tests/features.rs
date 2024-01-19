mod util;

macro_rules! feats {
    ($feats:expr) => {
        $feats.into_iter().map(|f| f.to_owned())
    };
}

/// Ensures weak dependencies are properly pruned if not explicitly pulled in
/// <https://github.com/EmbarkStudios/krates/issues/41>
#[test]
fn prunes_multiple_weak_features() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/features/Cargo.toml")
        .features(feats!(["blocking", "json", "multipart", "stream"]))
        .no_default_features();

    let mut builder = krates::Builder::new();
    builder.include_targets([("x86_64-unknown-linux-gnu", vec![])]);
    let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

    // All of the specified features have a weak dependency on reqwest, so it
    // shouldn't be present in the graph
    assert_eq!(0, md.krates_by_name("reqwest").count());
    assert_eq!(0, md.krates_by_name("reqest").count());
}

/// While the zlib features brings in git2, the openssl dependency for both
/// git2 and git2-sys is optional and weak, and since we've not explicitly
/// enabled a feature to bring it in, it should not be present in the graph,
/// even though `cargo metadata` will list it in the graph
#[test]
fn prunes_mixed_dependencies() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/features/Cargo.toml")
        .features(feats!(["zlib"]));

    let mut builder = krates::Builder::new();
    builder.include_targets([("x86_64-unknown-linux-gnu", vec![])]);
    let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

    assert_eq!(0, md.krates_by_name("openssl-sys").count());
    // cmake is brought in via the zlib-ng-compat feature. gross.
    assert_eq!(1, md.krates_by_name("cmake").count());
}

macro_rules! assert_features {
    ($graph:expr, $name:expr, $features:expr) => {
        let krates::KrateMatch { kid, .. } = $graph.krates_by_name($name).next().unwrap();

        let expected_features: std::collections::BTreeSet<_> =
            $features.into_iter().map(|s| s.to_owned()).collect();

        assert_eq!(
            $graph.get_enabled_features(kid).unwrap(),
            &expected_features
        );
    };
}

/// Ensures we can enable crate features even when that crate has been renamed
/// as well as features being precise
#[test]
fn handles_features_for_renamed_crates() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/features/Cargo.toml")
        .no_default_features()
        .features(feats!(["audio", "midi"]));

    let mut builder = krates::Builder::new();
    builder.include_targets([("aarch64-apple-darwin", vec![])]);
    let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

    assert_eq!(1, md.krates_by_name("coreaudio-rs").count());

    // Ensure only the single feature is enabled, not all of them
    assert_features!(md, "coreaudio-sys", ["core_midi"]);
}

#[test]
fn ignores_excluded_crates() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/features/Cargo.toml")
        .no_default_features()
        .features(feats!(["audio", "midi"]));

    let mut builder = krates::Builder::new();
    builder.include_targets([("aarch64-apple-darwin", vec![])]);
    builder.exclude(["coreaudio-rs".parse().unwrap()]);
    let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

    assert_eq!(0, md.krates_by_name("coreaudio-rs").count());
}

/// Ensures that explicitly toggling on an optional dependency works
#[test]
fn handles_explicit_weak_features() {
    {
        let mut cmd = krates::Cmd::new();
        cmd.manifest_path("tests/features/Cargo.toml")
            .no_default_features()
            .features(feats!(["reqest", "tls"]));

        let mut builder = krates::Builder::new();
        builder.include_targets([("x86_64-unknown-linux-musl", vec![])]);
        let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

        // let dg = krates::petgraph::dot::Dot::new(md.graph()).to_string();

        // std::fs::write("ohno.dot", dg).unwrap();

        assert_features!(
            md,
            "reqwest",
            [
                // Rustls
                "__rustls",
                "__tls",
                "hyper-rustls",
                "rustls",
                "rustls-tls",
                "rustls-pemfile",
                "rustls-tls-webpki-roots",
                "tokio-rustls",
                "webpki-roots",
                // Brotli
                "async-compression",
                "brotli",
                "tokio-util",
                // Cookies
                "cookies",
                "cookie_crate",
                "cookie_store",
                "proc-macro-hack",
            ]
        );
    }

    {
        let mut cmd = krates::Cmd::new();
        cmd.manifest_path("tests/features/Cargo.toml")
            .no_default_features()
            .features(feats!(["reqest"]));

        let mut builder = krates::Builder::new();
        builder.include_targets([("x86_64-unknown-linux-musl", vec![])]);

        // By filtering out the "normal" crates we're removing the 'brotli'
        // feature enabled on reqwest
        builder.ignore_kind(krates::DepKind::Normal, krates::Scope::All);
        let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

        assert_features!(
            md,
            "reqwest",
            ["cookie_crate", "cookie_store", "cookies", "proc-macro-hack"]
        );
    }

    {
        let mut cmd = krates::Cmd::new();
        cmd.manifest_path("tests/features/Cargo.toml")
            // We explicitly enable reqwest, and use default features, which
            // should thus pull in json
            .features(feats!(["reqest"]));

        let mut builder = krates::Builder::new();
        builder.include_targets([("x86_64-unknown-linux-musl", vec![])]);
        let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

        assert_features!(
            md,
            "reqwest",
            [
                // Cookies
                "cookie_crate",
                "cookie_store",
                "cookies",
                "proc-macro-hack",
                // Json
                "json",
                "serde_json",
                // Brotli
                "async-compression",
                "brotli",
                "tokio-util",
            ]
        );
    }
}

/// Ensures that having an optional dependency enabled by one crate doesn't add
/// an edge from another crate that has a weak dependency on the same crate
#[test]
fn ensure_weak_features_dont_add_edges() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/features/Cargo.toml")
        .no_default_features()
        .features(feats!(["tls-no-reqwest", "reqest"]));

    let mut builder = krates::Builder::new();
    builder.include_targets([("x86_64-unknown-linux-musl", vec![])]);
    let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

    assert_features!(
        md,
        "reqwest",
        [
            // Cookies
            "cookie_crate",
            "cookie_store",
            "cookies",
            "proc-macro-hack",
            // Brotli
            "async-compression",
            "brotli",
            "tokio-util",
        ]
    );
}

/// Ensures we handle cyclic features
#[test]
fn handles_cyclic_features() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/features/Cargo.toml")
        .no_default_features()
        .features(feats!(["cycle"]));

    let mut builder = krates::Builder::new();
    builder.include_targets([("x86_64-unknown-linux-musl", vec![])]);
    let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

    assert_features!(md, "features-galore", ["cycle", "midi", "subfeatcycle"]);
}

/// Ensures that features only brought in by eg dev-dependencies are not used if
/// dev-dependencies are ignored
/// <https://github.com/EmbarkStudios/krates/issues/60>
#[test]
fn ignores_features_for_ignored_kinds() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/feature-bug/Cargo.toml")
        .all_features();

    let mut builder = krates::Builder::new();
    builder.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
    let md: krates::Krates<util::JustId> = builder.build(cmd, krates::NoneFilter).unwrap();

    let dotgraph = krates::petgraph::dot::Dot::new(md.graph()).to_string();
    insta::assert_snapshot!(dotgraph);
}

/// Tests validating <https://github.com/EmbarkStudios/krates/issues/46>
mod prefer_index {
    fn confirm_index_snapshot(builder: krates::Builder) {
        let mut cmd = krates::Cmd::new();
        cmd.manifest_path("tests/bug/Cargo.toml");

        let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

        assert_features!(md, "conv", ["default", "std"]);
    }

    /// Validates we can use the sparse index to fix features
    #[test]
    fn uses_sparse_index() {
        let mut b = krates::Builder::new();

        let cache_index = |krates: std::collections::BTreeSet<String>| {
            let index = tame_index::index::ComboIndexCache::new(tame_index::IndexLocation::new(
                tame_index::IndexUrl::CratesIoSparse,
            ))
            .unwrap();

            let lock_opt = tame_index::utils::flock::LockOptions::cargo_package_lock(None).unwrap();
            let lock = lock_opt.lock(|_p| None).unwrap();

            let mut cache = std::collections::BTreeMap::new();
            for name in krates {
                let read = || -> Option<krates::index::IndexKrate> {
                    let name = name.as_str().try_into().ok()?;
                    let krate = index.cached_krate(name, &lock).ok()??;
                    let versions = krate
                        .versions
                        .into_iter()
                        .filter_map(|kv| {
                            // The index (currently) can have both features, and
                            // features2, the features method gives us an iterator
                            // over both
                            kv.version.parse::<semver::Version>().ok().map(|version| {
                                krates::index::IndexKrateVersion {
                                    version,
                                    features: kv
                                        .features()
                                        .map(|(k, v)| (k.clone(), v.clone()))
                                        .collect(),
                                }
                            })
                        })
                        .collect();

                    Some(krates::index::IndexKrate { versions })
                };

                let krate = read();
                cache.insert(name, krate);
            }

            cache
        };

        b.with_crates_io_index(Box::new(cache_index));
        confirm_index_snapshot(b);
    }
}
