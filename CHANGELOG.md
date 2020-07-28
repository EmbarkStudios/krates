# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate
### Fixed
- Fix to version `0.11.1` of `cargo_metadata`.

## [0.4.0] - 2020-07-28
### Fixed
- Align `semver` version with the same one used by `cargo_metadata`, again.

## [0.3.1] - 2020-07-18
### Fixed
- Align `semver` version with the same one used by `cargo_metadata`

## [0.3.0] - 2020-06-04
### Changed
- Updated `cfg-expr` to 0.4.0, and added the `targets` feature, will enable the `targets` feature in cfg-expr, allowing the use of matching cfg expressions against `target_lexicon::Triple` instead of only built-in targets/names.

## [0.2.0] - 2020-02-05
### Changed
- Updated `cfg-expr` to 0.2.0, so only 1.41.0 built-in targets are fully supported

## [0.1.1] - 2020-02-04
### Added
- Added `PkgSpec`, an implementation of cargo's [package id specifications](https://doc.rust-lang.org/cargo/reference/pkgid-spec.html)
- Added `Builder::workspace`, which allows the equivalent of `cargo <cmd> --workspace` when building the graph
- Added `Builder::exclude`, which allows the equivalent of `cargo <cmd> --exclude` when building the graph

## [0.1.0] - 2020-01-14
### Added
- Initial implementation

<!-- next-url -->
[Unreleased]: https://github.com/EmbarkStudios/krates/compare/0.4.0...HEAD
[0.4.0]: https://github.com/EmbarkStudios/krates/compare/0.3.1...0.4.0
[0.3.1]: https://github.com/EmbarkStudios/krates/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/EmbarkStudios/krates/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/EmbarkStudios/krates/compare/0.1.1...0.2.0
[0.1.1]: https://github.com/EmbarkStudios/krates/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/EmbarkStudios/krates/releases/tag/0.1.0
