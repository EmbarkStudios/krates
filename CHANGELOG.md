# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate
### Added
- Added `PkgSpec`, an implementation of cargo's [package id specifications](https://doc.rust-lang.org/cargo/reference/pkgid-spec.html)
- Added `Builder::workspace`, which allows the equivalent of `cargo <cmd> --workspace` when building the graph
- Added `Builder::exclude`, which allows the equivalent of `cargo <cmd> --exclude` when building the graph

## [0.1.0] - 2020-01-14
### Added
- Initial implementation

<!-- next-url -->
[Unreleased]: https://github.com/EmbarkStudios/krates/compare/0.1.0...HEAD
[0.1.0]: https://github.com/EmbarkStudios/krates/releases/tag/0.1.0
