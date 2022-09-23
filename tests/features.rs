mod util;

/// Ensures weak dependencies are properly pruned if not explicitly pulled in
/// https://github.com/EmbarkStudios/krates/issues/41
#[test]
fn prunes_multiple_weak_features() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/features/Cargo.toml")
        .features(
            ["blocking", "json", "multipart", "stream"]
                .into_iter()
                .map(|f| f.to_owned()),
        )
        .no_default_features();

    let mut builder = krates::Builder::new();
    builder.include_targets([("x86_64-unknown-linux-gnu", vec![])]);
    let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

    for name in md.krates().map(|k| &k.name) {
        println!("{name}");
    }

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
        .features(["zlib".to_owned()]);

    let mut builder = krates::Builder::new();
    builder.include_targets([("x86_64-unknown-linux-gnu", vec![])]);
    let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

    assert_eq!(0, md.krates_by_name("openssl-sys").count());
    // cmake is brought in via the zlib-ng-compat feature. gross.
    assert_eq!(1, md.krates_by_name("cmake").count());
}

macro_rules! assert_features {
    ($graph:expr, $name:expr, $features:expr) => {
        let (_, krate) = $graph.krates_by_name($name).next().unwrap();

        assert_eq!($graph.get_enabled_features(&krate.id).unwrap(), $features);
    };
}

/// Ensures we can enable crate features even when that crate has been renamed
/// as well as features being precise
#[test]
fn handles_features_for_renamed_crates() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/features/Cargo.toml")
        .no_default_features()
        .features(["midi".to_owned()]);

    let mut builder = krates::Builder::new();
    builder.include_targets([("aarch64-apple-darwin", vec![])]);
    let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

    assert_eq!(1, md.krates_by_name("coreaudio-rs").count());

    // Ensure only the single feature is enabled, not all of them
    assert_features!(md, "coreaudio-sys", ["core_midi"]);
}

/// Ensures that explicitly toggling on an optional dependency works
#[test]
fn handles_explicit_weak_features() {
    {
        let mut cmd = krates::Cmd::new();
        cmd.manifest_path("tests/features/Cargo.toml")
            .no_default_features()
            .features(["reqest".to_owned()]);

        let mut builder = krates::Builder::new();
        builder.include_targets([("x86_64-unknown-linux-musl", vec![])]);
        let md: util::Graph = builder.build(cmd, krates::NoneFilter).unwrap();

        let dg = krates::petgraph::dot::Dot::new(md.graph()).to_string();

        std::fs::write("ohno.dot", dg).unwrap();

        // assert_features!(
        //     md,
        //     "reqwest",
        //     [
        //         "__rustls",
        //         "__tls",
        //         "async-compression",
        //         "brotli",
        //         "cookie_crate",
        //         "cookie_store",
        //         "cookies",
        //         "hyper-rustls",
        //         "proc-macro-hack",
        //         "rustls",
        //         "rustls-pemfile",
        //         "rustls-tls",
        //         "rustls-tls-webpki-roots",
        //         "tokio-rustls",
        //         "tokio-util",
        //         "webpki-roots"
        //     ]
        // );
    }

    // {
    //     let mut cmd = krates::Cmd::new();
    //     cmd.manifest_path("tests/features/Cargo.toml")
    //         .no_default_features()
    //         .features(["reqest".to_owned()]);

    //     let mut builder = krates::Builder::new();
    //     builder.include_targets([("x86_64-unknown-linux-musl", vec![])]);
    //     builder.ignore_kind(krates::DepKind::Normal, krates::Scope::All);
    //     let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

    //     assert_features!(
    //         md,
    //         "reqwest",
    //         ["cookie_crate", "cookie_store", "cookies", "proc-macro-hack"]
    //     );
    // }

    // let mut cmd = krates::Cmd::new();
    // cmd.manifest_path("tests/features/Cargo.toml")
    //     .features(["reqest".to_owned()]);

    // let mut builder = krates::Builder::new();
    // builder.include_targets([("x86_64-unknown-linux-musl", vec![])]);
    // let md: krates::Krates = builder.build(cmd, krates::NoneFilter).unwrap();

    // assert_features!(md, "reqwest", ["json", "cookies"]);
}
