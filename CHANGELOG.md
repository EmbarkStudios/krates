<!-- markdownlint-disable blanks-around-headings blanks-around-lists no-duplicate-heading -->

# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate
## [0.16.10] - 2024-04-12
### Fixed
- [PR#83](https://github.com/EmbarkStudios/krates/pull/83) resolved [#82](https://github.com/EmbarkStudios/krates/issues/82) by properly handling `cfg()` specific dependencies for the same crate with different features enabled.
- [PR#83](https://github.com/EmbarkStudios/krates/pull/83) fixed an issue where `cfg(any())` crates would be pulled into the graph erroneously if not performing target filtering.

## [0.16.9] - 2024-04-09
### Fixed
- [PR#81](https://github.com/EmbarkStudios/krates/pull/81) re-resolved [#79](https://github.com/EmbarkStudios/krates/issues/79) because the PR#80 completely broke in the presence of cargo patches.

## [0.16.8] - 2024-04-09
### Fixed
- [PR#80](https://github.com/EmbarkStudios/krates/pull/80) resolved [#79](https://github.com/EmbarkStudios/krates/issues/79) by fixing an extreme edge case with dependency renaming.

## [0.16.7] - 2024-03-20
### Fixed
- [PR#78](https://github.com/EmbarkStudios/krates/pull/78) fixed an issue where setting manifest_path to `Cargo.toml` without preceding `./` would cause the current directory be set to empty, and cargo_metadata to fail.

## [0.16.6] - 2024-01-24
### Fixed
- [PR#77](https://github.com/EmbarkStudios/krates/pull/77) resolved [#76](https://github.com/EmbarkStudios/krates/issues/76) by special casing "wildcard" version requirements if the version being tested is a pre-release, as pre-releases must have at least one comparator.

## [0.16.5] - 2024-01-24
### Fixed
- [PR#75](https://github.com/EmbarkStudios/krates/pull/75) resolved [#74](https://github.com/EmbarkStudios/krates/issues/74) by just always checking version requirements for dependencies. Sigh.

## [0.16.4] - 2024-01-22
### Fixed
- [PR#73](https://github.com/EmbarkStudios/krates/pull/73) resolved [#72](https://github.com/EmbarkStudios/krates/issues/72) by correctly parsing the new stable package ids where a specifier was not used.

## [0.16.3] - 2024-01-22
### Fixed
- [PR#71](https://github.com/EmbarkStudios/krates/pull/71) fixed an issue introduced in [PR#70](https://github.com/EmbarkStudios/krates/pull/70) that would cause duplicates to not be detected correctly. Thanks [@louisdewar](https://github.com/louisdewar)!

## [0.16.2] - 2024-01-21
### Fixed
- [PR#70](https://github.com/EmbarkStudios/krates/pull/70) resolved [#68](https://github.com/EmbarkStudios/krates/issues/68) and [#69](https://github.com/EmbarkStudios/krates/issues/69) by additionally checking the version of resolve dependencies if there were 2 or more of the same name referenced by the same crate.

## [0.16.1] - 2024-01-20
### Fixed
- [PR#67](https://github.com/EmbarkStudios/krates/pull/67) resolved [#66](https://github.com/EmbarkStudios/krates/issues/66) by ignore features that reference crates that aren't resolved, instead of panicing, as there should only be one case where that occurs.

## [0.16.0] - 2024-01-19
### Fixed
- [PR#65](https://github.com/EmbarkStudios/krates/pull/65) resolved [#64](https://github.com/EmbarkStudios/krates/issues/64) by adding support for the newly stabilized (currently nightly only) package id format.

### Changed
- [PR#65](https://github.com/EmbarkStudios/krates/pull/65) changed `Kid` from just a type alias for `cargo_metadata::PackageId` to an actual type that has accessors for the various components of the id. It also specifies its own `Ord` etc implementation so that those ids are sorted the exact same as the old version.

## [0.15.3] - 2024-01-12
### Fixed
- [PR#63](https://github.com/EmbarkStudios/krates/pull/63) resolved [#62](https://github.com/EmbarkStudios/krates/issues/62) which was a bug introduced in [PR#61](https://github.com/EmbarkStudios/krates/pull/61)

## [0.15.2] - 2024-01-12
### Fixed
- [PR#61](https://github.com/EmbarkStudios/krates/pull/61) resolved [#60](https://github.com/EmbarkStudios/krates/issues/60) by refactoring the building of the crate graph to do its own crate and feature resolution to properly handle pruning based on the user's desires.

## [0.15.1] - 2023-09-03
### Added
- [PR#59](https://github.com/EmbarkStudios/krates/pull/59) added `Krates::krates_filtered`, allowing filtering of crates based upon their edge kinds.

## [0.15.0] - 2023-08-23
### Changed
- [PR#58](https://github.com/EmbarkStudios/krates/pull/58) removed the `prefer-index` feature, which brought in `tame-index`, in favor of just letting the user provide a callback that can be used to gather index information, freeing this crate from dependency issues and allowing downstream crates more flexibility.

## [0.14.1] - 2023-08-21
### Changed
- [PR#57](https://github.com/EmbarkStudios/krates/pull/57) bumped `tame-index` to `0.4`.

## [0.14.0] - 2023-07-25
### Changed
- [PR#55](https://github.com/EmbarkStudios/krates/pull/55) and [PR#56](https://github.com/EmbarkStudios/krates/pull/56) replaced `crates-index` with `tame-index`
- [PR#56](https://github.com/EmbarkStudios/krates/pull/56) changed `Krates::lock_path` -> `Krates::workspace_root`, which can then be joined with `Cargo.lock` to get the same path, but workspace root is more generally useful.

## [0.13.1] - 2023-06-13
### Fixed
- [PR#54](https://github.com/EmbarkStudios/krates/pull/54) fixed an issue where the crates.io index was unconditionally opened, and synced, if the `prefer-index` feature was enabled, causing long stalls if using the crates.io sparse index instead.

## [0.13.0] - 2023-04-04
### Changed
- [PR#53](https://github.com/EmbarkStudios/krates/pull/53) updated `cfg-expr` to 0.14 and `crates-index` to 0.19.

### Fixed
- [PR#53](https://github.com/EmbarkStudios/krates/pull/53) added support for using the HTTP sparse index for crates.io. If the sparse index was enabled and there wasn't a regular git index (for example, if you use `dtolnay/rust-toolchain@stable` in your CI) this would cause no index to be used to fix crate features if `prefer-index` was enabled.

## [0.12.6] - 2022-11-25
### Changed
- [PR#52](https://github.com/EmbarkStudios/krates/pull/52) updated cfg-expr to 0.12.
- [PR#52](https://github.com/EmbarkStudios/krates/pull/52) changed `Krates::search_matches` and `Krates::search_by_name` to use `impl Into<String>` for the name to search, so that the lifetime of it is not paired with the graph itself.

## [0.12.5] - 2022-11-08
### Fixed
- [PR#51](https://github.com/EmbarkStudios/krates/pull/51) resolved [#50](https://github.com/EmbarkStudios/krates/issues/50) by no longer treating the feature set in the index as authoritative, but rather just merging in the keys that were not already located in the feature set from the crate itself. This would mean that features that are present in both but with different sub-features from the index will now be lost, but that can be fixed later if it is actually an issue.

## [0.12.4] - 2022-11-02
### Fixed
- [PR#49](https://github.com/EmbarkStudios/krates/pull/49) resolved [#48](https://github.com/EmbarkStudios/krates/issues/48) by not entering into an infinite loop in the presence of cyclic features. Oops.

## [0.12.3] - 2022-11-01
### Fixed
- [PR#47](https://github.com/EmbarkStudios/krates/pull/47) resolved [#46](https://github.com/EmbarkStudios/krates/issues/46) by both adding the `prefer-index` feature to get the actual correct feature information for a crate from the index, rather than the cargo metadata, as well as silently ignoring features that are resolved, but not available from the package manifest if the feature is not enabled.

## [0.12.2] - 2022-10-28
### Fixed
- [PR#45](https://github.com/EmbarkStudios/krates/pull/45) fixed a bug where optional dependencies could be pruned if their name differed from the feature that enabled them.

### Added
- [PR#45](https://github.com/EmbarkStudios/krates/pull/45) added `Krates::direct_dependencies` as a complement to `Krates::direct_dependents`.

## [0.12.1] - 2022-10-25
### Added
- [PR#43](https://github.com/EmbarkStudios/krates/pull/43) and [PR#44](https://github.com/EmbarkStudios/krates/pull/44) added `Krates::direct_dependents` to more easily obtain the crates that directly depend on the specified crate/node, regardless of any features in between those crates.

## [0.12.0] - 2022-10-06
### Added
- [PR#42](https://github.com/EmbarkStudios/krates/pull/42) added support for features, adding nodes for each unique future, and linking edges between dependencies and features themselves. This (hopefully) properly takes into account the existing ways of pruning the graph via targets, exclusions etc. It also allows the retrieval of that final feature set via `Krates::get_enabled_features`.

### Fixed
- [PR#42](https://github.com/EmbarkStudios/krates/pull/42) resolved [#41](https://github.com/EmbarkStudios/krates/issues/41) by properly pruning weak dependencies that were improperly resolved by cargo.

## [0.11.0] - 2022-07-04
### Changed
- [PR#40](https://github.com/EmbarkStudios/krates/pull/40) updated `cargo_metadata` to 0.15. Thanks [@pinkforest](https://github.com/pinkforest)!

## [0.10.1] - 2022-02-16
### Fixed
- [PR#38](https://github.com/EmbarkStudios/krates/pull/38) fixed [#37](https://github.com/EmbarkStudios/krates/issues/37) by properly adding multiple features if specified.

## [0.10.0] - 2022-02-04
### Changed
- [PR#36](https://github.com/EmbarkStudios/krates/pull/36) updated `cfg-expr` and fixed up crates.io metadata.

## [0.9.0] - 2021-10-21
### Fixed
- [PR#35](https://github.com/EmbarkStudios/krates/pull/35) changed `Krates::search_matches` to get rid of unnecessary lifetime coupling.

### Changed
- [PR#35](https://github.com/EmbarkStudios/krates/pull/35) updated `cfg-expr` to 0.9.

## [0.8.1] - 2021-07-20
### Added
- [PR#34](https://github.com/EmbarkStudios/krates/pull/34) added support for the [`--locked`, `--offline`, and `--frozen`](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html#manifest-options) arguments.

## [0.8.0] - 2021-07-16
### Changed
- [PR#32](https://github.com/EmbarkStudios/krates/pull/32) replaced the use of `difference` with `similar`. Thanks [@j-k](https://github.com/06kellyjac)!
- [PR#33](https://github.com/EmbarkStudios/krates/pull/33) updated `semver`, `cargo_metadata`, `petgraph`, and `cfg-expr` to their latest versions.

## [0.7.0] - 2021-03-11
### Changed
- Updated `cargo_metadata` to 0.13.0, which uses [`camino`](https://docs.rs/camino/1.0.2/camino/) for path information, so it is reexported and used for `Krates::lock_path`

## [0.6.0] - 2021-02-12
### Changed
- Updated `cfg-expr` to 0.7.0, which brings targets as of 1.50.0

## [0.5.0] - 2020-10-20
### Added
- Added `impl PartialEq<cargo_metadata::DependencyKind> for DepKind`

### Changed
- Updated `semver`, `cargo_metadata`, and `cfg-expr.

## [0.4.2] - 2020-10-13
### Fixed
- [PR#19](https://github.com/EmbarkStudios/krates/pull/19) Fixed an issue where `git` sources could differ in package id representation between the actual source, and the id used to specify it as a dependency from another package.

## [0.4.1] - 2020-07-28
### Fixed
- Fix to version `0.11.1` of `cargo_metadata`.

## [0.4.0] - 2020-07-28q
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
[Unreleased]: https://github.com/EmbarkStudios/krates/compare/0.16.10...HEAD
[0.16.10]: https://github.com/EmbarkStudios/krates/compare/0.16.9...0.16.10
[0.16.9]: https://github.com/EmbarkStudios/krates/compare/0.16.8...0.16.9
[0.16.8]: https://github.com/EmbarkStudios/krates/compare/0.16.7...0.16.8
[0.16.7]: https://github.com/EmbarkStudios/krates/compare/0.16.6...0.16.7
[0.16.6]: https://github.com/EmbarkStudios/krates/compare/0.16.5...0.16.6
[0.16.5]: https://github.com/EmbarkStudios/krates/compare/0.16.4...0.16.5
[0.16.4]: https://github.com/EmbarkStudios/krates/compare/0.16.3...0.16.4
[0.16.3]: https://github.com/EmbarkStudios/krates/compare/0.16.2...0.16.3
[0.16.2]: https://github.com/EmbarkStudios/krates/compare/0.16.1...0.16.2
[0.16.1]: https://github.com/EmbarkStudios/krates/compare/0.16.0...0.16.1
[0.16.0]: https://github.com/EmbarkStudios/krates/compare/0.15.3...0.16.0
[0.15.3]: https://github.com/EmbarkStudios/krates/compare/0.15.2...0.15.3
[0.15.2]: https://github.com/EmbarkStudios/krates/compare/0.15.1...0.15.2
[0.15.1]: https://github.com/EmbarkStudios/krates/compare/0.15.0...0.15.1
[0.15.0]: https://github.com/EmbarkStudios/krates/compare/0.14.1...0.15.0
[0.14.1]: https://github.com/EmbarkStudios/krates/compare/0.14.0...0.14.1
[0.14.0]: https://github.com/EmbarkStudios/krates/compare/0.13.1...0.14.0
[0.13.1]: https://github.com/EmbarkStudios/krates/compare/0.13.0...0.13.1
[0.13.0]: https://github.com/EmbarkStudios/krates/compare/0.12.6...0.13.0
[0.12.6]: https://github.com/EmbarkStudios/krates/compare/0.12.5...0.12.6
[0.12.5]: https://github.com/EmbarkStudios/krates/compare/0.12.4...0.12.5
[0.12.4]: https://github.com/EmbarkStudios/krates/compare/0.12.3...0.12.4
[0.12.3]: https://github.com/EmbarkStudios/krates/compare/0.12.2...0.12.3
[0.12.2]: https://github.com/EmbarkStudios/krates/compare/0.12.1...0.12.2
[0.12.1]: https://github.com/EmbarkStudios/krates/compare/0.12.0...0.12.1
[0.12.0]: https://github.com/EmbarkStudios/krates/compare/0.11.0...0.12.0
[0.11.0]: https://github.com/EmbarkStudios/krates/compare/0.10.1...0.11.0
[0.10.1]: https://github.com/EmbarkStudios/krates/compare/0.10.0...0.10.1
[0.10.0]: https://github.com/EmbarkStudios/krates/compare/0.9.0...0.10.0
[0.9.0]: https://github.com/EmbarkStudios/krates/compare/0.8.1...0.9.0
[0.8.1]: https://github.com/EmbarkStudios/krates/compare/0.8.0...0.8.1
[0.8.0]: https://github.com/EmbarkStudios/krates/compare/0.7.0...0.8.0
[0.7.0]: https://github.com/EmbarkStudios/krates/compare/0.6.0...0.7.0
[0.6.0]: https://github.com/EmbarkStudios/krates/compare/0.5.0...0.6.0
[0.5.0]: https://github.com/EmbarkStudios/krates/compare/0.4.2...0.5.0
[0.1.0]: https://github.com/EmbarkStudios/krates/releases/tag/0.1.0
