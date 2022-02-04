<div align="center">

# `📦 krates`

[![Embark](https://img.shields.io/badge/embark-open%20source-blueviolet.svg)](https://embark.dev)
[![Embark](https://img.shields.io/badge/discord-ark-%237289da.svg?logo=discord)](https://discord.gg/dAuKfZS)
[![Crates.io](https://img.shields.io/crates/v/krates.svg)](https://crates.io/crates/krates)
[![Docs](https://docs.rs/krates/badge.svg)](https://docs.rs/krates)
[![dependency status](https://deps.rs/repo/github/EmbarkStudios/krates/status.svg)](https://deps.rs/repo/github/EmbarkStudios/krates)
[![Build Status](https://github.com/EmbarkStudios/krates/workflows/CI/badge.svg)](https://github.com/EmbarkStudios/krates/actions?workflow=CI)

Creates graphs of crates from [cargo_metadata](https://crates.io/crates/cargo_metadata) metadata.

</div>

## Usage

```rust
use krates::{Builder, Cmd, Krates, cm, petgraph};
fn main() -> Result<(), krates::Error> {
    let mut cmd = Cmd::new();
    cmd.manifest_path("path/to/a/Cargo.toml");
    // Enable all features, works for either an entire workspace or a single crate
    cmd.all_features();

    let mut builder = Builder::new();
    // Let's filter out any crates that aren't used by x86_64 windows
    builder.include_targets(std::iter::once(("x86_64-pc-windows-msvc", vec![])));

    let krates: Krates = builder.build(cmd, |pkg: cm::Package| {
        println!("Crate {} was filtered out", pkg.id);
    })?;

    // Print a dot graph of the entire crate graph
    println!("{:?}", petgraph::dot::Dot::new(krates.graph()));

    Ok(())
}
```

`krates` can also be used if you use `cargo` as a dependency. It doesn't depend on `cargo` itself since cargo moves quickly and we don't want to artificially limit which versions you use, but, at least with the current stable `cargo` crate, the following code works well.

```rust
fn get_metadata(
    no_default_features: bool,
    all_features: bool,
    features: Vec<String>,
    manifest_path: PathBuf,
) -> Result<krates::cm::Metadata, anyhow::Error> {
    let config = cargo::util::Config::default()?;
    let ws = cargo::core::Workspace::new(&manifest_path, &config)?;
    let options = cargo::ops::OutputMetadataOptions {
        features,
        no_default_features,
        all_features,
        no_deps: false,
        version: 1,
        filter_platforms: vec![],
    };

    let md = cargo::ops::output_metadata(&ws, &options)?;
    let md_value = serde_json::to_value(md)?;

    Ok(serde_json::from_value(md_value)?)
}
```

## Contributing

[![Contributor Covenant](https://img.shields.io/badge/contributor%20covenant-v1.4-ff69b4.svg)](../CODE_OF_CONDUCT.md)

We welcome community contributions to this project.

Please read our [Contributor Guide](CONTRIBUTING.md) for more information on how to get started.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
