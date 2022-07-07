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

#[test]
fn weak_dependencies() {
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path("tests/features/Cargo.toml")
        .features(["serde"].into_iter().map(|f| f.to_owned()));

    let mdc: krates::cm::MetadataCommand = cmd.into();

    mdc.exec().unwrap();
}
