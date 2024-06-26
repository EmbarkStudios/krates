//! Internal version of [`cargo_metadata`](https://github.com/oli-obk/cargo_metadata)

pub use camino::Utf8PathBuf as PathBuf;
use semver::Version;
use serde::Deserialize;
use std::{collections::BTreeMap, fmt};

mod cmd;
mod dependency;
mod errors;

pub use cmd::MetadataCommand;
pub use dependency::{Dependency, DependencyKind};
pub use errors::Error;

/// An "opaque" identifier for a package.
///
/// It is possible to inspect the `repr` field, if the need arises, but its
/// precise format is an implementation detail and is subject to change.
///
/// `Metadata` can be indexed by `PackageId`.
#[derive(Clone, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct PackageId {
    /// The underlying string representation of id.
    pub repr: String,
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.repr, f)
    }
}

#[derive(Clone, Deserialize, Debug)]
/// Starting point for metadata returned by `cargo metadata`
pub struct Metadata {
    /// A list of all crates referenced by this crate (and the crate itself)
    pub packages: Vec<Package>,
    /// A list of all workspace members
    pub workspace_members: Vec<PackageId>,
    /// The list of default workspace members
    ///
    /// This not available if running with a version of Cargo older than 1.71.
    #[serde(skip_serializing_if = "workspace_default_members_is_missing")]
    pub workspace_default_members: WorkspaceDefaultMembers,
    /// Dependencies graph
    pub resolve: Option<Resolve>,
    /// Workspace root
    pub workspace_root: PathBuf,
    /// Build directory
    pub target_directory: PathBuf,
    /// The workspace-level metadata object. Null if non-existent.
    #[serde(rename = "metadata", default)]
    pub workspace_metadata: serde_json::Value,
    /// The metadata format version
    pub version: usize,
}

impl Metadata {
    /// Get the workspace's root package of this metadata instance.
    pub fn root_package(&self) -> Option<&Package> {
        match &self.resolve {
            Some(resolve) => {
                // if dependencies are resolved, use Cargo's answer
                let root = resolve.root.as_ref()?;
                self.packages.iter().find(|pkg| &pkg.id == root)
            }
            None => {
                // if dependencies aren't resolved, check for a root package manually
                let root_manifest_path = self.workspace_root.join("Cargo.toml");
                self.packages
                    .iter()
                    .find(|pkg| pkg.manifest_path == root_manifest_path)
            }
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

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
/// A list of default workspace members.
///
/// See [`Metadata::workspace_default_members`].
///
/// It is only available if running a version of Cargo of 1.71 or newer.
///
/// # Panics
///
/// Dereferencing when running an older version of Cargo will panic.
pub struct WorkspaceDefaultMembers(Option<Vec<PackageId>>);

impl core::ops::Deref for WorkspaceDefaultMembers {
    type Target = [PackageId];

    fn deref(&self) -> &Self::Target {
        self.0
            .as_ref()
            .expect("WorkspaceDefaultMembers should only be dereferenced on Cargo versions >= 1.71")
    }
}

/// Return true if a valid value for [`WorkspaceDefaultMembers`] is missing, and
/// dereferencing it would panic.
///
/// Internal helper for `skip_serializing_if` and test code. Might be removed in
/// the future.
#[doc(hidden)]
pub fn workspace_default_members_is_missing(
    workspace_default_members: &WorkspaceDefaultMembers,
) -> bool {
    workspace_default_members.0.is_none()
}

#[derive(Clone, Deserialize, Debug)]
/// A dependency graph
pub struct Resolve {
    /// Nodes in a dependencies graph
    pub nodes: Vec<Node>,

    /// The crate for which the metadata was read.
    pub root: Option<PackageId>,
}

impl<'a> std::ops::Index<&'a PackageId> for Resolve {
    type Output = Node;

    fn index(&self, idx: &'a PackageId) -> &Self::Output {
        self.nodes
            .iter()
            .find(|p| p.id == *idx)
            .unwrap_or_else(|| panic!("no Node with this id: {:?}", idx))
    }
}

#[derive(Clone, Deserialize, Debug)]
/// A node in a dependencies graph
pub struct Node {
    /// An opaque identifier for a package
    pub id: PackageId,
    /// Dependencies in a structured format.
    ///
    /// `deps` handles renamed dependencies whereas `dependencies` does not.
    #[serde(default)]
    pub deps: Vec<NodeDep>,

    /// List of opaque identifiers for this node's dependencies.
    /// It doesn't support renamed dependencies. See `deps`.
    pub dependencies: Vec<PackageId>,

    /// Features enabled on the crate
    #[serde(default)]
    pub features: Vec<String>,
}

#[derive(Clone, Deserialize, Debug)]
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
    #[serde(default)]
    pub dep_kinds: Vec<DepKindInfo>,
}

#[derive(Clone, Deserialize, Debug)]
/// Information about a dependency kind.
pub struct DepKindInfo {
    /// The kind of dependency.
    #[serde(deserialize_with = "dependency::parse_dependency_kind")]
    pub kind: DependencyKind,
    /// The target platform for the dependency.
    ///
    /// This is `None` if it is not a target dependency.
    pub target: Option<String>,
}

#[derive(Clone, Deserialize, Debug)]
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
    #[serde(default)]
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
    #[serde(default)]
    pub categories: Vec<String>,
    /// The [`keywords` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-keywords-field) as specified in the `Cargo.toml`
    #[serde(default)]
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
    #[serde(default)]
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
    #[serde(default, skip_serializing_if = "is_null")]
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
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_rust_version")]
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
#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
#[serde(transparent)]
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

#[derive(Clone, Deserialize, Debug)]
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
    #[serde(default)]
    #[cfg_attr(feature = "builder", builder(default))]
    pub crate_types: Vec<CrateType>,

    #[serde(default)]
    #[cfg_attr(feature = "builder", builder(default))]
    #[serde(rename = "required-features")]
    /// This target is built only if these features are enabled.
    /// It doesn't apply to `lib` targets.
    pub required_features: Vec<String>,
    /// Path to the main source file of the target
    pub src_path: PathBuf,
    /// Rust edition for this target
    #[serde(default)]
    #[cfg_attr(feature = "builder", builder(default))]
    pub edition: Edition,
    /// Whether or not this target has doc tests enabled, and the target is
    /// compatible with doc testing.
    ///
    /// This is always `true` if running with a version of Cargo older than 1.37.
    #[serde(default = "default_true")]
    #[cfg_attr(feature = "builder", builder(default = "true"))]
    pub doctest: bool,
    /// Whether or not this target is tested by default by `cargo test`.
    ///
    /// This is always `true` if running with a version of Cargo older than 1.47.
    #[serde(default = "default_true")]
    #[cfg_attr(feature = "builder", builder(default = "true"))]
    pub test: bool,
    /// Whether or not this target is documented by `cargo doc`.
    ///
    /// This is always `true` if running with a version of Cargo older than 1.50.
    #[serde(default = "default_true")]
    #[cfg_attr(feature = "builder", builder(default = "true"))]
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
#[derive(Clone, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetKind {
    /// `cargo bench` target
    #[serde(rename = "bench")]
    Bench,
    /// Binary executable target
    #[serde(rename = "bin")]
    Bin,
    /// Custom build target
    #[serde(rename = "custom-build")]
    CustomBuild,
    /// Dynamic system library target
    #[serde(rename = "cdylib")]
    CDyLib,
    /// Dynamic Rust library target
    #[serde(rename = "dylib")]
    DyLib,
    /// Example target
    #[serde(rename = "example")]
    Example,
    /// Rust library
    #[serde(rename = "lib")]
    Lib,
    /// Procedural Macro
    #[serde(rename = "proc-macro")]
    ProcMacro,
    /// Rust library for use as an intermediate artifact
    #[serde(rename = "rlib")]
    RLib,
    /// Static system library
    #[serde(rename = "staticlib")]
    StaticLib,
    /// Test target
    #[serde(rename = "test")]
    Test,
}

#[allow(clippy::fallible_impl_from)]
impl From<&str> for TargetKind {
    fn from(value: &str) -> Self {
        match value {
            "example" => TargetKind::Example,
            "test" => TargetKind::Test,
            "bench" => TargetKind::Bench,
            "custom-build" => TargetKind::CustomBuild,
            "bin" => TargetKind::Bin,
            "lib" => TargetKind::Lib,
            "rlib" => TargetKind::RLib,
            "dylib" => TargetKind::DyLib,
            "cdylib" => TargetKind::CDyLib,
            "staticlib" => TargetKind::StaticLib,
            "proc-macro" => TargetKind::ProcMacro,
            x => panic!("unknown target kind {x}"),
        }
    }
}

/// Similar to `kind`, but only reports the
/// [Cargo crate types](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-crate-type-field):
/// `bin`, `lib`, `rlib`, `dylib`, `cdylib`, `staticlib`, `proc-macro`.
/// Everything that's not a proc macro or a library of some kind is reported as "bin".
///
/// Other possible values may be added in the future.
#[derive(Clone, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CrateType {
    /// Binary executable target
    #[serde(rename = "bin")]
    Bin,
    /// Dynamic system library target
    #[serde(rename = "cdylib")]
    CDyLib,
    /// Dynamic Rust library target
    #[serde(rename = "dylib")]
    DyLib,
    /// Rust library
    #[serde(rename = "lib")]
    Lib,
    /// Procedural Macro
    #[serde(rename = "proc-macro")]
    ProcMacro,
    /// Rust library for use as an intermediate artifact
    #[serde(rename = "rlib")]
    RLib,
    /// Static system library
    #[serde(rename = "staticlib")]
    StaticLib,
}

#[allow(clippy::fallible_impl_from)]
impl From<&str> for CrateType {
    fn from(value: &str) -> Self {
        match value {
            "bin" => CrateType::Bin,
            "lib" => CrateType::Lib,
            "rlib" => CrateType::RLib,
            "dylib" => CrateType::DyLib,
            "cdylib" => CrateType::CDyLib,
            "staticlib" => CrateType::StaticLib,
            "proc-macro" => CrateType::ProcMacro,
            x => panic!("unknown crate type {x}"),
        }
    }
}

/// The Rust edition
///
/// As of writing this comment rust editions 2024, 2027 and 2030 are not actually a thing yet but are parsed nonetheless for future proofing.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Edition {
    /// Edition 2015
    #[serde(rename = "2015")]
    E2015,
    /// Edition 2018
    #[serde(rename = "2018")]
    E2018,
    /// Edition 2021
    #[serde(rename = "2021")]
    E2021,
}

impl Edition {
    /// Return the string representation of the edition
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::E2015 => "2015",
            Self::E2018 => "2018",
            Self::E2021 => "2021",
        }
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

fn default_true() -> bool {
    true
}

/// As per the Cargo Book the [`rust-version` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field) must:
///
/// > be a bare version number with two or three components;
/// > it cannot include semver operators or pre-release identifiers.
///
/// [`semver::Version`] however requires three components. This function takes
/// care of appending `.0` if the provided version number only has two components
/// and ensuring that it does not contain a pre-release version or build metadata.
fn deserialize_rust_version<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Version>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let mut buf = match Option::<String>::deserialize(deserializer)? {
        None => return Ok(None),
        Some(buf) => buf,
    };

    for char in buf.chars() {
        if char == '-' {
            return Err(serde::de::Error::custom(
                "pre-release identifiers are not supported in rust-version",
            ));
        } else if char == '+' {
            return Err(serde::de::Error::custom(
                "build metadata is not supported in rust-version",
            ));
        }
    }

    if buf.matches('.').count() == 1 {
        // e.g. 1.0 -> 1.0.0
        buf.push_str(".0");
    }

    Ok(Some(
        Version::parse(&buf).map_err(serde::de::Error::custom)?,
    ))
}

#[cfg(test)]
mod test {
    use semver::Version;

    #[derive(Debug, serde::Deserialize)]
    struct BareVersion(
        #[serde(deserialize_with = "super::deserialize_rust_version")] Option<semver::Version>,
    );

    fn bare_version(str: &str) -> Version {
        serde_json::from_str::<BareVersion>(&format!(r#""{}""#, str))
            .unwrap()
            .0
            .unwrap()
    }

    fn bare_version_err(str: &str) -> String {
        serde_json::from_str::<BareVersion>(&format!(r#""{}""#, str))
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn test_deserialize_rust_version() {
        assert_eq!(bare_version("1.2"), Version::new(1, 2, 0));
        assert_eq!(bare_version("1.2.0"), Version::new(1, 2, 0));
        assert_eq!(
            bare_version_err("1.2.0-alpha"),
            "pre-release identifiers are not supported in rust-version"
        );
        assert_eq!(
            bare_version_err("1.2.0+123"),
            "build metadata is not supported in rust-version"
        );
    }
}
