//! Transforms the output of [`cargo metadata`] into a graph, [`Krates`], where
//! crates are nodes and dependency links are edges.
//!
//! ```no_run
//! use krates::{Builder, Cmd, Krates, cm, petgraph};
//!
//! fn main() -> Result<(), krates::Error> {
//!     let mut cmd = Cmd::new();
//!     cmd.manifest_path("path/to/a/Cargo.toml");
//!     // Enable all features, works for either an entire workspace or a single crate
//!     cmd.all_features();
//!
//!     let mut builder = Builder::new();
//!     // Let's filter out any crates that aren't used by x86_64 windows
//!     builder.include_targets(std::iter::once(("x86_64-pc-windows-msvc", vec![])));
//!
//!     let krates: Krates = builder.build(cmd, |pkg: cm::Package| {
//!         println!("Crate {} was filtered out", pkg.id);
//!     })?;
//!
//!     // Print a dot graph of the entire crate graph
//!     println!("{:?}", petgraph::dot::Dot::new(krates.graph()));
//!
//!     Ok(())
//! }
//! ```

#[cfg(feature = "metadata")]
pub use cargo_metadata as cm;
#[cfg(not(feature = "metadata"))]
pub mod cm;

pub use cfg_expr;

#[cfg(feature = "targets")]
pub use cfg_expr::target_lexicon;

use cm::DependencyKind as DK;
pub use cm::{Metadata, Package, PackageId};

pub use petgraph;
pub use semver;

pub use camino::{self, Utf8Path, Utf8PathBuf};
use petgraph::{Direction, graph::EdgeIndex, graph::NodeIndex, visit::EdgeRef};

mod builder;
mod errors;
mod pkgspec;

pub use builder::{
    Builder, Cmd, LockOptions, NoneFilter, OnFilter, Scope, Target,
    features::{Feature, ParsedFeature},
    index,
};
pub use errors::Error;
pub use pkgspec::PkgSpec;
use std::fmt;

/// A crate's unique identifier
#[derive(Clone, Default)]
pub struct Kid {
    /// The full package id string as supplied by cargo
    pub repr: String,
    /// The subslices for each component in name -> version -> source order
    components: [(usize, usize); 3],
}

impl Kid {
    /// Gets the name of the package
    #[inline]
    pub fn name(&self) -> &str {
        let (s, e) = self.components[0];
        &self.repr[s..e]
    }

    /// Gets the semver of the package
    #[inline]
    pub fn version(&self) -> &str {
        let (s, e) = self.components[1];
        &self.repr[s..e]
    }

    /// Gets the source url of the package
    #[inline]
    pub fn source(&self) -> &str {
        let (s, e) = self.components[2];
        &self.repr[s..e]
    }
}

#[allow(clippy::fallible_impl_from)]
impl From<PackageId> for Kid {
    fn from(pid: PackageId) -> Self {
        let mut repr = pid.repr;

        let mut parse = || {
            let components = if repr.contains(' ') {
                let name = (0, repr.find(' ')?);
                let version = (name.1 + 1, repr[name.1 + 1..].find(' ')? + name.1 + 1);
                // Note we skip the open and close parentheses as they are superfluous
                // as every source has them, as well as not being present in the new
                // stabilized format
                //
                // Note that we also chop off the commit id, it is not present in
                // the stabilized format and is not used for package identification anyways
                let source = (version.1 + 2, repr.rfind('#').unwrap_or(repr.len() - 1));

                [name, version, source]
            } else {
                let mut vmn = repr.rfind('#')?;
                let (name, version) = if let Some(split) = repr[vmn..].find('@') {
                    ((vmn + 1, vmn + split), (vmn + split + 1, repr.len()))
                } else {
                    let begin = repr.rfind('/')? + 1;
                    let end = if repr.starts_with("git+") {
                        // Unfortunately the stable format percent encodes the source url in the metadata, and since
                        // git branches/tags can contain various special characters, notably '/', we need to decode them
                        // to be able to match against the non-encoded url used...everywhere else
                        let end = repr[begin..].rfind('?').map_or(vmn, |q| q + begin);

                        if repr[end..].contains('%') {
                            let mut decoded = String::new();
                            let mut encoded = &repr[end..];
                            let before = encoded.len();

                            loop {
                                let Some(pi) = encoded.find('%') else {
                                    decoded.push_str(encoded);
                                    break;
                                };

                                decoded.push_str(&encoded[..pi]);

                                let Some(encoding) = encoded.get(pi + 1..pi + 3) else {
                                    // This _should_ never happen, but just in case
                                    panic!(
                                        "invalid percent encoding in '{}', '{}' should be exactly 3 digits long",
                                        &repr[end..],
                                        &encoded[pi..]
                                    );
                                };

                                // https://en.wikipedia.org/wiki/Percent-encoding
                                // Reserved characters after percent-encoding
                                //
                                // ␣   !   "   #   $   %   &   '   (   )   *   +   ,   /   :   ;   =   ?   @   [   ]
                                // %20 %21 %22 %23 %24 %25 %26 %27 %28 %29 %2A %2B %2C %2F %3A %3B %3D %3F %40 %5B %5D
                                //
                                // Common characters after percent-encoding (ASCII or UTF-8 based)
                                //
                                // -   .   <   >   \   ^   _   `   {   |   }   ~
                                // %2D %2E %3C %3E %5C %5E %5F %60 %7B %7C %7D %7E
                                //
                                // Note that `£` and `€` can also be percent encoded, but are _completely_ different and
                                // I don't feel like supporting it until someone actually complains

                                let c = match encoding {
                                    // By far the most likely one
                                    "2F" | "2f" => '/',
                                    "21" => '!',
                                    "22" => '"',
                                    "23" => '#',
                                    "24" => '$',
                                    "25" => '%',
                                    "26" => '&',
                                    "27" => '\'',
                                    "28" => '(',
                                    "29" => ')',
                                    "2A" | "2a" => '*',
                                    "2B" | "2b" => '+',
                                    "2C" | "2c" => ',',
                                    "2D" | "2d" => '-',
                                    "2E" | "2e" => '.',
                                    "3B" | "3b" => ';',
                                    "3C" | "3c" => '<',
                                    "3D" | "3d" => '=',
                                    "3E" | "3e" => '>',
                                    "40" => '@',
                                    "5D" | "5d" => ']',
                                    "5F" | "5f" => '_',
                                    "60" => '`',
                                    "7B" | "7b" => '{',
                                    "7C" | "7c" => '|',
                                    "7D" | "7d" => '}',
                                    // These are invalid in branches/tags, but meh
                                    // https://git-scm.com/docs/git-check-ref-format
                                    "20" => ' ',
                                    "3A" | "3a" => ':',
                                    "3F" | "3f" => '?',
                                    "5B" | "5b" => '[',
                                    "5C" | "5c" => '\\',
                                    "5E" | "5e" => '^',
                                    "7E" | "7e" => '~',
                                    _ => panic!(
                                        "unknown percent encoding '%{encoding}' in '{}'",
                                        &repr[end..]
                                    ),
                                };

                                decoded.push(c);
                                encoded = &encoded[pi + 3..];
                            }

                            repr.truncate(end);
                            repr.push_str(&decoded);

                            // move the version string back to account for the now shorter decoded repr
                            vmn -= before - decoded.len();
                        }

                        end
                    } else {
                        vmn
                    };

                    ((begin, end), (vmn + 1, repr.len()))
                };

                [name, version, (0, vmn)]
            };

            Some(components)
        };

        if let Some(components) = parse() {
            Self { repr, components }
        } else {
            panic!("unable to parse package id '{repr}'");
        }
    }
}

impl fmt::Debug for Kid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("Kid");

        ds.field("name", &self.name())
            .field("version", &self.version());

        let src = self.source();
        if src != "registry+https://github.com/rust-lang/crates.io-index" {
            ds.field("source", &src);
        }

        ds.finish()
    }
}

impl fmt::Display for Kid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.repr)
    }
}

impl std::hash::Hash for Kid {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(self.repr.as_bytes());
    }
}

impl Ord for Kid {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        let a = &self.repr;
        let b = &o.repr;

        for (ar, br) in self.components.into_iter().zip(o.components.into_iter()) {
            let ord = a[ar.0..ar.1].cmp(&b[br.0..br.1]);
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }

        std::cmp::Ordering::Equal
    }
}

impl PartialOrd for Kid {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Kid {}

impl PartialEq for Kid {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

/// The set of features that have been enabled on a crate
pub type EnabledFeatures = std::collections::BTreeSet<String>;

/// The dependency kind. A crate can depend on the same crate multiple times
/// with different dependency kinds
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DepKind {
    Normal,
    Build,
    Dev,
}

impl From<DK> for DepKind {
    fn from(dk: DK) -> Self {
        match dk {
            DK::Normal => Self::Normal,
            DK::Build => Self::Build,
            DK::Development => Self::Dev,
            #[cfg(feature = "metadata")]
            DK::Unknown => unreachable!(),
        }
    }
}

impl PartialEq<DK> for DepKind {
    fn eq(&self, other: &DK) -> bool {
        matches!(
            (self, *other),
            (Self::Normal, DK::Normal) | (Self::Build, DK::Build) | (Self::Dev, DK::Development)
        )
    }
}

impl fmt::Display for DepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => Ok(()),
            Self::Build => f.write_str("build"),
            Self::Dev => f.write_str("dev"),
        }
    }
}

/// A node identifier.
pub type NodeId = NodeIndex<u32>;
pub type EdgeId = EdgeIndex<u32>;

/// A node in the crate graph.
pub enum Node<N> {
    Krate {
        /// The unique identifier for this node.
        id: Kid,
        /// Associated user data with the node. Must be `From<cargo_metadata::Package>`
        krate: N,
        /// List of features enabled on the crate
        features: EnabledFeatures,
        /// Mapping to the exact node that a dependency is resolved to.
        ///
        /// This can be used manually but the helper `[Krates::resolved_dependency`]
        /// is easier to use
        dep_mapping: Vec<Option<NodeId>>,
    },
    Feature {
        /// The node index for the crate this feature is for
        krate_index: NodeId,
        name: String,
    },
}

impl<N> fmt::Display for Node<N>
where
    N: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Krate { krate, .. } => {
                write!(f, "crate {krate}")
            }
            Self::Feature { name, .. } => {
                write!(f, "feature {name}")
            }
        }
    }
}

impl<N> fmt::Debug for Node<N>
where
    N: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Krate { id, krate, .. } => {
                write!(f, "crate {} {krate:?}", id.repr)
            }
            Self::Feature { name, .. } => {
                write!(f, "feature {name}")
            }
        }
    }
}

/// The default type used for edges in the crate graph.
#[derive(Debug, Clone)]
pub enum Edge {
    Dep {
        /// The dependency kind for the edge link
        kind: DepKind,
        /// A possible `cfg()` or <target-triple> applied to this dependency
        cfg: Option<String>,
    },
    /// An edge from one feature to another
    Feature,
    DepFeature {
        /// The dependency kind for the edge link
        kind: DepKind,
        /// A possible `cfg()` or <target-triple> applied to this dependency
        cfg: Option<String>,
    },
}

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::DepFeature { kind, cfg } | Self::Dep { kind, cfg } = self {
            match kind {
                DepKind::Normal => {}
                DepKind::Build => f.write_str("(build)")?,
                DepKind::Dev => f.write_str("(dev)")?,
            };

            if let Some(cfg) = cfg {
                write!(f, " '{cfg}'")?;
            }
        }

        Ok(())
    }
}

/// A crate graph. Each unique crate is a node, and each unique dependency
/// between 2 crates is an edge.
pub struct Krates<N = cm::Package, E = Edge> {
    graph: petgraph::Graph<Node<N>, E, petgraph::Directed, u32>,
    workspace_members: Vec<Kid>,
    workspace_root: Utf8PathBuf,
    /// We split the graph between crate and feature nodes, but keep the crates
    /// grouped together in the front since most queries are against them
    krates_end: usize,
}

#[allow(clippy::len_without_is_empty)]
impl<N, E> Krates<N, E> {
    /// The number of unique crates in the graph
    #[inline]
    pub fn len(&self) -> usize {
        self.krates_end
    }

    /// Path to the root of the workspace where the graph metadata was acquired from
    #[inline]
    pub fn workspace_root(&self) -> &Utf8Path {
        &self.workspace_root
    }

    /// Get access to the raw petgraph
    #[inline]
    pub fn graph(&self) -> &petgraph::Graph<Node<N>, E> {
        &self.graph
    }

    /// Get an iterator over the crate nodes in the graph. The crates are always
    /// ordered lexicographically by their identfier.
    ///
    /// ```no_run
    /// use krates::Krates;
    ///
    /// fn print_krates(krates: &Krates) {
    ///     for (name, version) in krates.krates().map(|krate| (&krate.name, &krate.version)) {
    ///         println!("Crate {} @ {}", name, version);
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn krates(&self) -> impl Iterator<Item = &N> {
        self.graph.raw_nodes()[..self.krates_end]
            .iter()
            .filter_map(move |node| {
                if let Node::Krate { krate, .. } = &node.weight {
                    Some(krate)
                } else {
                    None
                }
            })
    }

    /// Get an iterator over each dependency of the specified crate. The same
    /// dependency can be returned multiple times if the crate depends on it
    /// with more than 1 dependency kind.
    ///
    /// ```no_run
    /// use krates::{Krates, Kid, DepKind};
    ///
    /// fn count_build_deps(krates: &Krates, pkg: &Kid) -> usize {
    ///     krates.get_deps(krates.nid_for_kid(pkg).unwrap())
    ///         .filter(|(_, edge)| matches!(
    ///             edge,
    ///             krates::Edge::Dep { kind: DepKind::Build, .. } |
    ///             krates::Edge::DepFeature { kind: DepKind::Build, .. }
    ///         ))
    ///         .count()
    /// }
    /// ```
    #[inline]
    pub fn get_deps(&self, id: NodeId) -> impl Iterator<Item = (&Node<N>, &E)> {
        self.graph
            .edges_directed(id, Direction::Outgoing)
            .map(move |edge| {
                let krate = &self.graph[edge.target()];
                (krate, edge.weight())
            })
    }

    /// Gets crates directly depended upon by the specified node
    #[inline]
    pub fn direct_dependencies(&self, nid: NodeId) -> Vec<DirectDependency<'_, N>> {
        let graph = self.graph();
        let mut direct_dependencies = Vec::new();
        let mut stack = vec![nid];
        let mut visited = std::collections::BTreeSet::new();

        while let Some(nid) = stack.pop() {
            for edge in graph.edges_directed(nid, Direction::Outgoing) {
                match &graph[edge.target()] {
                    Node::Krate { krate, .. } => {
                        if visited.insert(edge.target()) {
                            direct_dependencies.push(DirectDependency {
                                krate,
                                node_id: edge.target(),
                                edge_id: edge.id(),
                            });
                        }
                    }
                    Node::Feature { .. } => {
                        if visited.insert(edge.target()) {
                            stack.push(edge.target());
                        }
                    }
                }
            }
        }

        direct_dependencies
    }

    /// Gets the crates that have a direct dependency on the specified node
    #[inline]
    pub fn direct_dependents(&self, nid: NodeId) -> Vec<DirectDependent<'_, N>> {
        let graph = self.graph();
        let mut direct_dependents = Vec::new();
        let mut stack = vec![nid];
        let mut visited = std::collections::BTreeSet::new();

        while let Some(nid) = stack.pop() {
            for edge in graph.edges_directed(nid, Direction::Incoming) {
                match &graph[edge.source()] {
                    Node::Krate { krate, .. } => {
                        if visited.insert(edge.source()) {
                            direct_dependents.push(DirectDependent {
                                krate,
                                node_id: edge.source(),
                                edge_id: edge.id(),
                            });
                        }
                    }
                    Node::Feature { krate_index, .. } => {
                        if *krate_index == nid && visited.insert(edge.source()) {
                            stack.push(edge.source());
                        }
                    }
                }
            }
        }

        direct_dependents
    }

    /// Retrieves the krate that was resolved for the specified crate dependency.
    ///
    /// This will return `None` if the krate doesn't exist, the dependency doesn't
    /// exist, or dependency was pruned.
    ///
    /// Note that `dep_index` is the index of [`cargo_metadata::Package::dependencies`]
    #[inline]
    pub fn resolved_dependency(&self, nid: NodeId, dep_index: usize) -> Option<&N> {
        if nid.index() >= self.krates_end {
            return None;
        }

        let Node::Krate { dep_mapping, .. } = &self.graph[nid] else {
            return None;
        };
        dep_mapping
            .get(dep_index)
            .copied()
            .flatten()
            .and_then(|nid| {
                let Node::Krate { krate, .. } = &self.graph[nid] else {
                    return None;
                };
                Some(krate)
            })
    }

    /// Get the node identifier for the specified crate identifier
    #[inline]
    pub fn nid_for_kid(&self, kid: &Kid) -> Option<NodeId> {
        self.graph.raw_nodes()[..self.krates_end]
            .binary_search_by(|rn| {
                if let Node::Krate { id, .. } = &rn.weight {
                    id.cmp(kid)
                } else {
                    unreachable!();
                }
            })
            .ok()
            .map(NodeId::new)
    }

    /// Get the node for the specified crate identifier
    #[inline]
    pub fn node_for_kid(&self, kid: &Kid) -> Option<&Node<N>> {
        self.nid_for_kid(kid).map(|nid| &self.graph[nid])
    }

    #[inline]
    pub fn get_node(&self, kid: &Kid, feature: Option<&str>) -> Option<(NodeId, &Node<N>)> {
        self.nid_for_kid(kid).and_then(|nid| {
            if let Some(feat) = feature {
                self.graph
                    .edges_directed(nid, Direction::Incoming)
                    .find_map(|edge| {
                        if let Node::Feature { krate_index, name } = &self.graph[edge.source()] {
                            if *krate_index == nid && name == feat {
                                return Some((edge.source(), &self.graph[edge.source()]));
                            }
                        }

                        None
                    })
            } else {
                Some((nid, &self.graph[nid]))
            }
        })
    }

    /// Gets the features enabled for the specified crate
    #[inline]
    pub fn get_enabled_features(&self, kid: &Kid) -> Option<&EnabledFeatures> {
        self.node_for_kid(kid).map(|node| {
            if let Node::Krate { features, .. } = node {
                features
            } else {
                unreachable!()
            }
        })
    }

    /// Get an iterator over the nodes for the members of the workspace
    #[inline]
    pub fn workspace_members(&self) -> impl Iterator<Item = &Node<N>> {
        self.workspace_members
            .iter()
            .filter_map(move |pid| self.nid_for_kid(pid).map(|ind| &self.graph[ind]))
    }
}

/// A direct dependency of a crate
pub struct DirectDependency<'krates, N> {
    /// The crate in the node
    pub krate: &'krates N,
    /// The crate's node id
    pub node_id: NodeId,
    /// The edge that links the crate with the crate that depends on it
    pub edge_id: EdgeId,
}

/// A crate that has a direct dependency on another crate
pub struct DirectDependent<'krates, N> {
    /// The crate in the node
    pub krate: &'krates N,
    /// The crate's node id
    pub node_id: NodeId,
    /// The edge that links the crate with the dependency
    pub edge_id: EdgeId,
}

/// A trait that can be applied to the type stored in the graph nodes to give
/// additional features on `Krates`.
pub trait KrateDetails {
    /// The name of the crate
    fn name(&self) -> &str;
    /// The version of the crate
    fn version(&self) -> &semver::Version;
}

impl KrateDetails for cm::Package {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &semver::Version {
        &self.version
    }
}

impl<N> Krates<N, Edge>
where
    N: std::fmt::Debug,
{
    /// Removes all of the crates that are only referenced via the specified
    /// dependency kind.
    ///
    /// This gives the same output as if the graph had been built by using
    /// [`ignore_kind`](crate::Builder::ignore_kind) with [`Scope::all`](crate::Scope::All)
    pub fn krates_filtered(&self, filter: DepKind) -> Vec<&N> {
        let graph = self.graph();
        let mut filtered: std::collections::BTreeMap<_, _> = self
            .workspace_members()
            .filter_map(|n| {
                let Node::Krate { id, krate, .. } = n else {
                    return None;
                };
                Some((id, krate))
            })
            .collect();
        let mut stack: Vec<_> = self
            .workspace_members
            .iter()
            .filter_map(|pid| self.nid_for_kid(pid))
            .collect();
        let mut visited = std::collections::BTreeSet::new();

        while let Some(nid) = stack.pop() {
            for edge in graph.edges_directed(nid, Direction::Outgoing) {
                match edge.weight() {
                    Edge::Dep { kind, .. } | Edge::DepFeature { kind, .. } => {
                        if *kind == filter {
                            continue;
                        }
                    }
                    Edge::Feature => {}
                };

                match &graph[edge.target()] {
                    Node::Krate { id, krate, .. } => {
                        if visited.insert(edge.target()) {
                            stack.push(edge.target());
                            filtered.insert(id, krate);
                        }
                    }
                    Node::Feature { .. } => {
                        if visited.insert(edge.target()) {
                            stack.push(edge.target());
                        }
                    }
                }
            }

            visited.insert(nid);
        }

        filtered.into_values().collect()
    }
}

/// If the node type N supports [`KrateDetails`], we can also iterator over krates
/// of a given name and or version
impl<N, E> Krates<N, E>
where
    N: KrateDetails,
{
    /// Get an iterator over the crates that match the specified name, as well
    /// as satisfy the specified semver requirement.
    ///
    /// ```no_run
    /// use krates::{Krates, semver::VersionReq};
    ///
    /// fn print(krates: &Krates, name: &str) {
    ///     let req = VersionReq::parse("=0.2").unwrap();
    ///     for vs in krates.search_matches(name, req.clone()).map(|km| &km.krate.version) {
    ///         println!("found version {vs} matching {req}!");
    ///     }
    /// }
    /// ```
    pub fn search_matches(
        &self,
        name: impl Into<String>,
        req: semver::VersionReq,
    ) -> impl Iterator<Item = KrateMatch<'_, N>> {
        let raw_nodes = &self.graph.raw_nodes()[0..self.krates_end];

        let name = name.into();

        raw_nodes
            .iter()
            .enumerate()
            .filter_map(move |(index, node)| {
                if let Node::Krate { krate, id, .. } = &node.weight {
                    if krate.name() == name && req.matches(krate.version()) {
                        return Some(KrateMatch {
                            node_id: NodeId::new(index),
                            krate,
                            kid: id,
                        });
                    }
                }

                None
            })
    }

    /// Get an iterator over all of the crates in the graph with the given name,
    /// in the case there are multiple versions, or sources, of the crate.
    ///
    /// ```
    /// use krates::Krates;
    ///
    /// fn print_all_versions(krates: &Krates, name: &str) {
    ///     for vs in krates.krates_by_name(name).map(|km| &km.krate.version) {
    ///         println!("found version {vs}");
    ///     }
    /// }
    /// ```
    pub fn krates_by_name(
        &self,
        name: impl Into<String>,
    ) -> impl Iterator<Item = KrateMatch<'_, N>> {
        let raw_nodes = &self.graph.raw_nodes()[0..self.krates_end];

        let name = name.into();

        raw_nodes
            .iter()
            .enumerate()
            .filter_map(move |(index, node)| {
                if let Node::Krate { krate, id, .. } = &node.weight {
                    if krate.name() == name {
                        return Some(KrateMatch {
                            node_id: NodeId::new(index),
                            krate,
                            kid: id,
                        });
                    }
                }

                None
            })
    }
}

pub struct KrateMatch<'graph, N> {
    pub krate: &'graph N,
    pub kid: &'graph Kid,
    pub node_id: NodeId,
}

impl<N, E> std::ops::Index<NodeId> for Krates<N, E> {
    type Output = N;

    #[inline]
    fn index(&self, id: NodeId) -> &Self::Output {
        match &self.graph[id] {
            Node::Krate { krate, .. } => krate,
            Node::Feature { .. } => panic!("indexed outside of crate graph"),
        }
    }
}

impl<N, E> std::ops::Index<usize> for Krates<N, E> {
    type Output = N;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        match &self.graph.raw_nodes()[idx].weight {
            Node::Krate { krate, .. } => krate,
            Node::Feature { .. } => panic!("indexed outside of crate graph"),
        }
    }
}

#[derive(Debug)]
struct MdTarget {
    inner: String,
    cfg: Option<cfg_expr::Expression>,
    #[cfg(feature = "metadata")]
    platform: cargo_platform::Platform,
}

#[cfg(feature = "metadata")]
impl From<cargo_platform::Platform> for MdTarget {
    fn from(platform: cargo_platform::Platform) -> Self {
        let inner = platform.to_string();
        let cfg = inner
            .starts_with("cfg(")
            .then(|| cfg_expr::Expression::parse(&inner).ok())
            .flatten();
        Self {
            inner,
            cfg,
            platform,
        }
    }
}

#[cfg(not(feature = "metadata"))]
impl From<String> for MdTarget {
    fn from(inner: String) -> Self {
        let cfg = inner
            .starts_with("cfg(")
            .then(|| cfg_expr::Expression::parse(&inner).ok())
            .flatten();
        Self { inner, cfg }
    }
}

#[cfg(feature = "metadata")]
fn targets_eq(target: &Option<MdTarget>, other: &Option<cargo_platform::Platform>) -> bool {
    match (target, other) {
        (None, None) => true,
        (Some(a), Some(b)) => a.platform.eq(b),
        _ => false,
    }
}

#[cfg(not(feature = "metadata"))]
fn targets_eq(target: &Option<MdTarget>, other: &Option<String>) -> bool {
    match (target, other) {
        (None, None) => true,
        (Some(a), Some(b)) => a.inner.eq(b),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn converts_package_ids() {
        let ids = [
            // STABLE
            // A typical registry url, source, name, and version are always distinct
            (
                "registry+https://github.com/rust-lang/crates.io-index#ab_glyph@0.2.22",
                "ab_glyph",
                "0.2.22",
                "registry+https://github.com/rust-lang/crates.io-index",
            ),
            // A git url, with a `rev` specifier. For git urls, if the name of the package is the same as the last path component of the source, the name is not repeated after the #, only the version
            (
                "git+https://github.com/EmbarkStudios/egui-stylist?rev=3900e8aedc5801e42c1bb747cfd025615bf3b832#0.2.0",
                "egui-stylist",
                "0.2.0",
                "git+https://github.com/EmbarkStudios/egui-stylist?rev=3900e8aedc5801e42c1bb747cfd025615bf3b832",
            ),
            // The same as with git urls, the name is only after the # if it is different from the last path component
            (
                "path+file:///home/jake/code/ark/components/allocator#ark-allocator@0.1.0",
                "ark-allocator",
                "0.1.0",
                "path+file:///home/jake/code/ark/components/allocator",
            ),
            // A git url with a `branch` specifier
            (
                "git+https://github.com/EmbarkStudios/ash?branch=nv-low-latency2#0.38.0+1.3.269",
                "ash",
                "0.38.0+1.3.269",
                "git+https://github.com/EmbarkStudios/ash?branch=nv-low-latency2",
            ),
            // A git url with a `branch` specifier and a different name from the repo
            (
                "git+https://github.com/EmbarkStudios/fsr-rs?branch=nv-low-latency2#fsr@0.1.7",
                "fsr",
                "0.1.7",
                "git+https://github.com/EmbarkStudios/fsr-rs?branch=nv-low-latency2",
            ),
            // A git url that doesn't specify a branch, tag, or revision, defaulting to HEAD
            (
                "git+https://github.com/ComunidadAylas/glsl-lang#0.5.2",
                "glsl-lang",
                "0.5.2",
                "git+https://github.com/ComunidadAylas/glsl-lang",
            ),
            // A git url that uses a `tag` specifier
            (
                "git+https://github.com/vtavernier/glsl-lang?tag=v0.5.2#0.5.2",
                "glsl-lang",
                "0.5.2",
                "git+https://github.com/vtavernier/glsl-lang?tag=v0.5.2",
            ),
            // OPAQUE
            (
                "fuser 0.4.1 (git+https://github.com/cberner/fuser?branch=master#b2e7622942e52a28ffa85cdaf48e28e982bb6923)",
                "fuser",
                "0.4.1",
                "git+https://github.com/cberner/fuser?branch=master",
            ),
            (
                "fuser 0.4.1 (git+https://github.com/cberner/fuser?rev=b2e7622#b2e7622942e52a28ffa85cdaf48e28e982bb6923)",
                "fuser",
                "0.4.1",
                "git+https://github.com/cberner/fuser?rev=b2e7622",
            ),
            (
                "a 0.1.0 (path+file:///home/jake/code/krates/tests/ws/a)",
                "a",
                "0.1.0",
                "path+file:///home/jake/code/krates/tests/ws/a",
            ),
            (
                "bindgen 0.59.2 (registry+https://github.com/rust-lang/crates.io-index)",
                "bindgen",
                "0.59.2",
                "registry+https://github.com/rust-lang/crates.io-index",
            ),
        ];

        for (repr, name, version, source) in ids {
            let kid = super::Kid::from(super::PackageId {
                repr: repr.to_owned(),
            });

            assert_eq!(kid.name(), name);
            assert_eq!(kid.version(), version);
            assert_eq!(kid.source(), source);
        }
    }
}
