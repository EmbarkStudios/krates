mod util;

#[test]
fn multiple_features() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/features/Cargo.toml").features(
        ["blocking", "json", "multipart", "stream"]
            .into_iter()
            .map(|f| f.to_owned()),
    );

    let mdc: krates::cm::MetadataCommand = cmd.into();

    mdc.exec().unwrap();
}

// Ensures weak dependencies are properly pruned if not explicitly pulled in
// https://github.com/EmbarkStudios/krates/issues/41
#[test]
fn prunes_weak_dependencies() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/features/Cargo.toml")
        .features(["zlib".to_owned()]);

    let mdc: krates::cm::MetadataCommand = cmd.into();

    let mut builder = krates::Builder::new();
    builder.include_targets([("x86_64-unknown-linux-gnu", vec![])]);
    let md: util::Graph = builder
        .build(mdc, |pkg: krates::cm::Package| {
            // if pkg.name == "git2" {
            //     dbg!(pkg);
            // }
        })
        .unwrap();

    let actual = krates::petgraph::dot::Dot::new(&md.graph()).to_string();

    std::fs::write("ohno.dot", actual).unwrap();

    // While the zlib features brings in git2, the openssl dependency for both
    // git2 and git2-sys is optional and weak, and since we've not explicitly
    // enabled a feature to bring it in, it should not be present in the graph,
    // even though `cargo metadata` will list it in the graph
    //assert_eq!(0, md.krates_by_name("openssl-sys").count());
    // cmake is brought in via the zlib-ng-compat feature. gross.
    //assert_eq!(1, md.krates_by_name("cmake").count());

    panic!("oh no");
}
