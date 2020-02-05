# 📦 krates

[![Build Status](https://github.com/EmbarkStudios/krates/workflows/CI/badge.svg)](https://github.com/EmbarkStudios/krates/actions?workflow=CI)
[![Crates.io](https://img.shields.io/crates/v/krates.svg)](https://crates.io/crates/krates)
[![Docs](https://docs.rs/krates/badge.svg)](https://docs.rs/krates)
[![Rust Version](https://img.shields.io/badge/Rust%20Version-1.41.0-blue.svg)](https://forge.rust-lang.org/release/platform-support.html)
[![Contributor Covenant](https://img.shields.io/badge/contributor%20covenant-v1.4%20adopted-ff69b4.svg)](CODE_OF_CONDUCT.md)
[![Embark](https://img.shields.io/badge/embark-open%20source-blueviolet.svg)](https://embark.dev)

Creates graphs of crates from [cargo_metadata](https://crates.io/crates/cargo_metadata) metadata.

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

## Contributing

We welcome community contributions to this project.

Please read our [Contributor Guide](CONTRIBUTING.md) for more information on how to get started.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
