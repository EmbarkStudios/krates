//! Internal version of [`cargo_metadata`](https://github.com/oli-obk/cargo_metadata)

pub use camino::Utf8PathBuf as PathBuf;
use semver::Version;
use std::{collections::BTreeMap, fmt, str::FromStr};

mod cmd;
mod de;
mod errors;
#[cfg(feature = "serialize")]
mod ser;

pub use cmd::MetadataCommand;
pub use errors::Error;

/// An "opaque" identifier for a package.
///
/// It is possible to inspect the `repr` field, if the need arises, but its
/// precise format is an implementation detail and is subject to change.
///
/// `Metadata` can be indexed by `PackageId`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId {
    /// The underlying string representation of id.
    pub repr: String,
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.repr, f)
    }
}

#[derive(Clone, Debug)]
/// Starting point for metadata returned by `cargo metadata`
pub struct Metadata {
    /// A list of all crates referenced by this crate (and the crate itself)
    pub packages: Vec<Package>,
    /// A list of all workspace members
    pub workspace_members: Vec<PackageId>,
    /// The list of default workspace members
    ///
    /// This not available if running with a version of Cargo older than 1.71.
    pub workspace_default_members: WorkspaceDefaultMembers,
    /// Dependencies graph
    pub resolve: Option<Resolve>,
    /// Workspace root
    pub workspace_root: PathBuf,
    /// Build directory
    pub target_directory: PathBuf,
    /// The workspace-level metadata object. Null if non-existent.
    pub workspace_metadata: serde_json::Value,
    /// The metadata format version
    pub version: usize,
}

impl Metadata {
    /// Get the workspace's root package of this metadata instance.
    pub fn root_package(&self) -> Option<&Package> {
        if let Some(resolve) = &self.resolve {
            // if dependencies are resolved, use Cargo's answer
            let root = resolve.root.as_ref()?;
            self.packages.iter().find(|pkg| &pkg.id == root)
        } else {
            // if dependencies aren't resolved, check for a root package manually
            let root_manifest_path = self.workspace_root.join("Cargo.toml");
            self.packages
                .iter()
                .find(|pkg| pkg.manifest_path == root_manifest_path)
        }
    }

    /// Get the workspace packages.
    pub fn workspace_packages(&self) -> Vec<&Package> {
        self.packages
            .iter()
            .filter(|&p| self.workspace_members.contains(&p.id))
            .collect()
    }

    /// Get the workspace default packages.
    ///
    /// # Panics
    ///
    /// This will panic if running with a version of Cargo older than 1.71.
    pub fn workspace_default_packages(&self) -> Vec<&Package> {
        self.packages
            .iter()
            .filter(|&p| self.workspace_default_members.contains(&p.id))
            .collect()
    }
}

impl<'a> std::ops::Index<&'a PackageId> for Metadata {
    type Output = Package;

    fn index(&self, idx: &'a PackageId) -> &Self::Output {
        self.packages
            .iter()
            .find(|p| p.id == *idx)
            .unwrap_or_else(|| panic!("no package with this id: {:?}", idx))
    }
}

#[derive(Clone, Debug)]
/// A list of default workspace members.
///
/// See [`Metadata::workspace_default_members`].
///
/// It is only available if running a version of Cargo of 1.71 or newer.
pub struct WorkspaceDefaultMembers(pub Option<Vec<PackageId>>);

/// We need to implement this so we can seamlessly swap between this implementation
/// and the `cargo_metadata` one, even though it is terrible
impl core::ops::Deref for WorkspaceDefaultMembers {
    type Target = [PackageId];

    fn deref(&self) -> &Self::Target {
        self.0
            .as_ref()
            .expect("WorkspaceDefaultMembers should only be dereferenced on Cargo versions >= 1.71")
    }
}

#[derive(Clone, Debug)]
/// A dependency graph
pub struct Resolve {
    /// Nodes in a dependencies graph
    pub nodes: Vec<Node>,
    /// The crate for which the metadata was read.
    pub root: Option<PackageId>,
}

#[derive(Clone, Debug)]
/// A node in a dependencies graph
pub struct Node {
    /// An opaque identifier for a package
    pub id: PackageId,
    /// Dependencies in a structured format.
    ///
    /// `deps` handles renamed dependencies whereas `dependencies` does not.
    pub deps: Vec<NodeDep>,
    /// List of opaque identifiers for this node's dependencies.
    /// It doesn't support renamed dependencies. See `deps`.
    pub dependencies: Vec<PackageId>,
    /// Features enabled on the crate
    pub features: Vec<String>,
}

#[derive(Clone, Debug)]
/// A dependency in a node
pub struct NodeDep {
    /// The name of the dependency's library target.
    /// If the crate was renamed, it is the new name.
    pub name: String,
    /// Package ID (opaque unique identifier)
    pub pkg: PackageId,
    /// The kinds of dependencies.
    ///
    /// This field was added in Rust 1.41.
    pub dep_kinds: Vec<DepKindInfo>,
}

#[derive(Clone, Debug)]
/// Information about a dependency kind.
pub struct DepKindInfo {
    /// The kind of dependency.
    pub kind: DependencyKind,
    /// The target platform for the dependency.
    ///
    /// This is `None` if it is not a target dependency.
    pub target: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, Copy, Hash)]
/// Dependencies can come in three kinds
pub enum DependencyKind {
    /// The 'normal' kind
    Normal,
    /// Those used in tests only
    Development,
    /// Those used in build scripts only
    Build,
}

impl Default for DependencyKind {
    fn default() -> Self {
        Self::Normal
    }
}

impl fmt::Display for DependencyKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => f.write_str("normal"),
            Self::Development => f.write_str("dev"),
            Self::Build => f.write_str("build"),
        }
    }
}

#[derive(Clone, Debug)]
/// A dependency of the main crate
pub struct Dependency {
    /// Name as given in the `Cargo.toml`
    pub name: String,
    /// The source of dependency
    pub source: Option<String>,
    /// The required version
    pub req: semver::VersionReq,
    /// The kind of dependency this is
    pub kind: DependencyKind,
    /// Whether this dependency is required or optional
    pub optional: bool,
    /// Whether the default features in this dependency are used.
    pub uses_default_features: bool,
    /// The list of features enabled for this dependency.
    pub features: Vec<String>,
    /// The target this dependency is specific to.
    pub target: Option<String>,
    /// If the dependency is renamed, this is the new name for the dependency
    /// as a string.  None if it is not renamed.
    pub rename: Option<String>,
    /// The URL of the index of the registry where this dependency is from.
    ///
    /// If None, the dependency is from crates.io.
    pub registry: Option<String>,
    /// The file system path for a local path dependency.
    ///
    /// Only produced on cargo 1.51+
    pub path: Option<camino::Utf8PathBuf>,
}

#[derive(Clone, Debug)]
/// One or more crates described by a single `Cargo.toml`
///
/// Each [`target`][Package::targets] of a `Package` will be built as a crate.
/// For more information, see <https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html>.
pub struct Package {
    /// The [`name` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-name-field) as given in the `Cargo.toml`
    // (We say "given in" instead of "specified in" since the `name` key cannot be inherited from the workspace.)
    pub name: String,
    /// The [`version` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-version-field) as specified in the `Cargo.toml`
    pub version: Version,
    /// The [`authors` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-authors-field) as specified in the `Cargo.toml`
    pub authors: Vec<String>,
    /// An opaque identifier for a package
    pub id: PackageId,
    /// The source of the package, e.g.
    /// crates.io or `None` for local projects.
    pub source: Option<Source>,
    /// The [`description` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-description-field) as specified in the `Cargo.toml`
    pub description: Option<String>,
    /// List of dependencies of this particular package
    pub dependencies: Vec<Dependency>,
    /// The [`license` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields) as specified in the `Cargo.toml`
    pub license: Option<String>,
    /// The [`license-file` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields) as specified in the `Cargo.toml`.
    /// If the package is using a nonstandard license, this key may be specified instead of
    /// `license`, and must point to a file relative to the manifest.
    pub license_file: Option<PathBuf>,
    /// Targets provided by the crate (lib, bin, example, test, ...)
    pub targets: Vec<Target>,
    /// Features provided by the crate, mapped to the features required by that feature.
    pub features: BTreeMap<String, Vec<String>>,
    /// Path containing the `Cargo.toml`
    pub manifest_path: PathBuf,
    /// The [`categories` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-categories-field) as specified in the `Cargo.toml`
    pub categories: Vec<String>,
    /// The [`keywords` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-keywords-field) as specified in the `Cargo.toml`
    pub keywords: Vec<String>,
    /// The [`readme` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-readme-field) as specified in the `Cargo.toml`
    pub readme: Option<PathBuf>,
    /// The [`repository` URL](https://doc.rust-lang.org/cargo/reference/manifest.html#the-repository-field) as specified in the `Cargo.toml`
    // can't use `url::Url` because that requires a more recent stable compiler
    pub repository: Option<String>,
    /// The [`homepage` URL](https://doc.rust-lang.org/cargo/reference/manifest.html#the-homepage-field) as specified in the `Cargo.toml`.
    ///
    /// On versions of cargo before 1.49, this will always be [`None`].
    pub homepage: Option<String>,
    /// The [`documentation` URL](https://doc.rust-lang.org/cargo/reference/manifest.html#the-documentation-field) as specified in the `Cargo.toml`.
    ///
    /// On versions of cargo before 1.49, this will always be [`None`].
    pub documentation: Option<String>,
    /// The default Rust edition for the package (either what's specified in the [`edition` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-edition-field)
    /// or defaulting to [`Edition::E2015`]).
    ///
    /// Beware that individual targets may specify their own edition in
    /// [`Target::edition`].
    pub edition: Edition,
    /// Contents of the free form [`package.metadata` section](https://doc.rust-lang.org/cargo/reference/manifest.html#the-metadata-table).
    ///
    /// This contents can be serialized to a struct using serde:
    ///
    /// ```rust
    /// use serde::Deserialize;
    /// use serde_json::json;
    ///
    /// #[derive(Debug, Deserialize)]
    /// struct SomePackageMetadata {
    ///     some_value: i32,
    /// }
    ///
    /// let value = json!({
    ///     "some_value": 42,
    /// });
    ///
    /// let package_metadata: SomePackageMetadata = serde_json::from_value(value).unwrap();
    /// assert_eq!(package_metadata.some_value, 42);
    ///
    /// ```
    //#[serde(default, skip_serializing_if = "is_null")]
    pub metadata: serde_json::Value,
    /// The name of a native library the package is linking to.
    pub links: Option<String>,
    /// List of registries to which this package may be published (derived from the [`publish` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-publish-field)).
    ///
    /// Publishing is unrestricted if `None`, and forbidden if the `Vec` is empty.
    ///
    /// This is always `None` if running with a version of Cargo older than 1.39.
    pub publish: Option<Vec<String>>,
    /// The [`default-run` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-default-run-field) as given in the `Cargo.toml`
    // (We say "given in" instead of "specified in" since the `default-run` key cannot be inherited from the workspace.)
    /// The default binary to run by `cargo run`.
    ///
    /// This is always `None` if running with a version of Cargo older than 1.55.
    pub default_run: Option<String>,
    /// The [`rust-version` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field) as specified in the `Cargo.toml`.
    /// The minimum supported Rust version of this package.
    ///
    /// This is always `None` if running with a version of Cargo older than 1.58.
    pub rust_version: Option<Version>,
}

impl Package {
    /// Full path to the license file if one is present in the manifest
    pub fn license_file(&self) -> Option<PathBuf> {
        self.license_file.as_ref().map(|file| {
            self.manifest_path
                .parent()
                .unwrap_or(&self.manifest_path)
                .join(file)
        })
    }

    /// Full path to the readme file if one is present in the manifest
    pub fn readme(&self) -> Option<PathBuf> {
        self.readme.as_ref().map(|file| {
            self.manifest_path
                .parent()
                .unwrap_or(&self.manifest_path)
                .join(file)
        })
    }
}

/// The source of a package such as crates.io.
///
/// It is possible to inspect the `repr` field, if the need arises, but its
/// precise format is an implementation detail and is subject to change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    /// The underlying string representation of a source.
    pub repr: String,
}

impl Source {
    /// Returns true if the source is crates.io.
    pub fn is_crates_io(&self) -> bool {
        self.repr == "registry+https://github.com/rust-lang/crates.io-index"
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.repr, f)
    }
}

#[derive(Clone, Debug)]
/// A single target (lib, bin, example, ...) provided by a crate
pub struct Target {
    /// Name as given in the `Cargo.toml` or generated from the file name
    pub name: String,
    /// Kind of target.
    ///
    /// The possible values are `example`, `test`, `bench`, `custom-build` and
    /// [Cargo crate types](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-crate-type-field):
    /// `bin`, `lib`, `rlib`, `dylib`, `cdylib`, `staticlib`, `proc-macro`.
    ///
    /// Other possible values may be added in the future.
    pub kind: Vec<TargetKind>,
    /// Similar to `kind`, but only reports the
    /// [Cargo crate types](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-crate-type-field):
    /// `bin`, `lib`, `rlib`, `dylib`, `cdylib`, `staticlib`, `proc-macro`.
    /// Everything that's not a proc macro or a library of some kind is reported as "bin".
    ///
    /// Other possible values may be added in the future.
    pub crate_types: Vec<CrateType>,
    /// This target is built only if these features are enabled.
    /// It doesn't apply to `lib` targets.
    pub required_features: Vec<String>,
    /// Path to the main source file of the target
    pub src_path: PathBuf,
    /// Rust edition for this target
    pub edition: Edition,
    /// Whether or not this target has doc tests enabled, and the target is
    /// compatible with doc testing.
    ///
    /// This is always `true` if running with a version of Cargo older than 1.37.
    pub doctest: bool,
    /// Whether or not this target is tested by default by `cargo test`.
    ///
    /// This is always `true` if running with a version of Cargo older than 1.47.
    pub test: bool,
    /// Whether or not this target is documented by `cargo doc`.
    ///
    /// This is always `true` if running with a version of Cargo older than 1.50.
    pub doc: bool,
}

impl Target {
    fn is_kind(&self, name: TargetKind) -> bool {
        self.kind.iter().any(|kind| kind == &name)
    }

    /// Return true if this target is of kind "lib".
    pub fn is_lib(&self) -> bool {
        self.is_kind(TargetKind::Lib)
    }

    /// Return true if this target is of kind "bin".
    pub fn is_bin(&self) -> bool {
        self.is_kind(TargetKind::Bin)
    }

    /// Return true if this target is of kind "example".
    pub fn is_example(&self) -> bool {
        self.is_kind(TargetKind::Example)
    }

    /// Return true if this target is of kind "test".
    pub fn is_test(&self) -> bool {
        self.is_kind(TargetKind::Test)
    }

    /// Return true if this target is of kind "bench".
    pub fn is_bench(&self) -> bool {
        self.is_kind(TargetKind::Bench)
    }

    /// Return true if this target is of kind "custom-build".
    pub fn is_custom_build(&self) -> bool {
        self.is_kind(TargetKind::CustomBuild)
    }

    /// Return true if this target is of kind "proc-macro".
    pub fn is_proc_macro(&self) -> bool {
        self.is_kind(TargetKind::ProcMacro)
    }
}

/// Kind of target.
///
/// The possible values are `example`, `test`, `bench`, `custom-build` and
/// [Cargo crate types](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-crate-type-field):
/// `bin`, `lib`, `rlib`, `dylib`, `cdylib`, `staticlib`, `proc-macro`.
///
/// Other possible values may be added in the future.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetKind {
    /// `cargo bench` target
    Bench,
    /// Binary executable target
    Bin,
    /// Custom build target
    CustomBuild,
    /// Dynamic system library target
    CDyLib,
    /// Dynamic Rust library target
    DyLib,
    /// Example target
    Example,
    /// Rust library
    Lib,
    /// Procedural Macro
    ProcMacro,
    /// Rust library for use as an intermediate artifact
    RLib,
    /// Static system library
    StaticLib,
    /// Test target
    Test,
}

impl FromStr for TargetKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "example" => Self::Example,
            "test" => Self::Test,
            "bench" => Self::Bench,
            "custom-build" => Self::CustomBuild,
            "bin" => Self::Bin,
            "lib" => Self::Lib,
            "rlib" => Self::RLib,
            "dylib" => Self::DyLib,
            "cdylib" => Self::CDyLib,
            "staticlib" => Self::StaticLib,
            "proc-macro" => Self::ProcMacro,
            x => return Err(format!("unknown target kind {x}")),
        })
    }
}

/// Similar to `kind`, but only reports the
/// [Cargo crate types](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-crate-type-field):
/// `bin`, `lib`, `rlib`, `dylib`, `cdylib`, `staticlib`, `proc-macro`.
/// Everything that's not a proc macro or a library of some kind is reported as "bin".
///
/// Other possible values may be added in the future.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CrateType {
    /// Binary executable target
    Bin,
    /// Dynamic system library target
    CDyLib,
    /// Dynamic Rust library target
    DyLib,
    /// Rust library
    Lib,
    /// Procedural Macro
    ProcMacro,
    /// Rust library for use as an intermediate artifact
    RLib,
    /// Static system library
    StaticLib,
}

impl FromStr for CrateType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "bin" => Self::Bin,
            "lib" => Self::Lib,
            "rlib" => Self::RLib,
            "dylib" => Self::DyLib,
            "cdylib" => Self::CDyLib,
            "staticlib" => Self::StaticLib,
            "proc-macro" => Self::ProcMacro,
            x => return Err(format!("unknown crate type {x}")),
        })
    }
}

/// The Rust edition
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Edition {
    /// Edition 2015
    E2015,
    /// Edition 2018
    E2018,
    /// Edition 2021
    E2021,
    /// Edition 2024
    E2024,
}

impl Edition {
    /// Return the string representation of the edition
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::E2015 => "2015",
            Self::E2018 => "2018",
            Self::E2021 => "2021",
            Self::E2024 => "2024",
        }
    }
}

impl FromStr for Edition {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "2015" => Self::E2015,
            "2018" => Self::E2018,
            "2021" => Self::E2021,
            "2024" => Self::E2024,
            x => return Err(format!("unknown edition {x}")),
        })
    }
}

impl fmt::Display for Edition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for Edition {
    fn default() -> Self {
        Self::E2015
    }
}
