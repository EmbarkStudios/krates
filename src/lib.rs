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

// BEGIN - Embark standard lints v5 for Rust 1.55+
// do not change or add/remove here, but one can add exceptions after this section
// for more info see: <https://github.com/EmbarkStudios/rust-ecosystem/issues/59>
#![deny(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::disallowed_method,
    clippy::disallowed_type,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_enforced_import_renames,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::rc_mutex,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms
)]
// END - Embark standard lints v0.5 for Rust 1.55+

pub use cargo_metadata as cm;
pub use cfg_expr;

#[cfg(feature = "targets")]
pub use cfg_expr::target_lexicon;

use cm::DependencyKind as DK;
pub use petgraph;
pub use semver;

pub use cm::camino::{self, Utf8Path, Utf8PathBuf};
use petgraph::{graph::NodeIndex, Direction};

mod builder;
mod errors;
mod pkgspec;

pub use builder::{Builder, Cmd, LockOptions, NoneFilter, OnFilter, Scope, Target};
pub use errors::Error;
pub use pkgspec::PkgSpec;

/// A crate's unique identifier
pub type Kid = cargo_metadata::PackageId;

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

use std::fmt;

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

/// A node in the crate graph.
pub struct Node<N> {
    /// The unique identifier for this node.
    pub id: Kid,
    /// Associated user data with the node. Must be From<cargo_metadata::Package>
    pub krate: N,
}

impl<N> fmt::Display for Node<N>
where
    N: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.krate)
    }
}

impl<N> fmt::Debug for Node<N>
where
    N: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {:?}", self.id.repr, self.krate)
    }
}

/// The default type used for edges in the crate graph.
#[derive(Debug, Clone)]
pub struct Edge {
    /// The dependency kind for the edge link
    pub kind: DepKind,
    /// A possible cfg() or <target-triple> applied to this dependency
    pub cfg: Option<String>,
}

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            DepKind::Normal => {}
            DepKind::Build => f.write_str("(build)")?,
            DepKind::Dev => f.write_str("(dev)")?,
        };

        if let Some(cfg) = &self.cfg {
            write!(f, " '{}'", cfg)?;
        }

        Ok(())
    }
}

/// A crate graph. Each unique crate is a node, and each unique dependency
/// between 2 crates is an edge.
pub struct Krates<N = cm::Package, E = Edge> {
    graph: petgraph::Graph<Node<N>, E>,
    workspace_members: Vec<Kid>,
    lock_file: Utf8PathBuf,
}

#[allow(clippy::len_without_is_empty)]
impl<N, E> Krates<N, E> {
    /// The number of unique crates in the graph
    #[inline]
    pub fn len(&self) -> usize {
        self.graph.node_count()
    }

    /// Path to the Cargo.lock file for the crate or workspace where the graph
    /// metadata was acquired from
    #[inline]
    pub fn lock_path(&self) -> &Utf8Path {
        &self.lock_file
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
    ///     for (name, version) in krates.krates().map(|kn| (&kn.krate.name, &kn.krate.version)) {
    ///         println!("Crate {} @ {}", name, version);
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn krates(&self) -> impl Iterator<Item = &Node<N>> {
        self.graph.node_indices().map(move |nid| &self.graph[nid])
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
    ///         .filter(|(_, edge)| edge.kind == DepKind::Build)
    ///         .count()
    /// }
    /// ```
    #[inline]
    pub fn get_deps(&self, id: NodeId) -> impl Iterator<Item = (&Node<N>, &E)> {
        use petgraph::visit::EdgeRef;

        self.graph
            .edges_directed(id, Direction::Outgoing)
            .map(move |edge| {
                let krate = &self.graph[edge.target()];
                (krate, edge.weight())
            })
    }

    /// Get the node identifier for the specified crate identifier
    #[inline]
    pub fn nid_for_kid(&self, kid: &Kid) -> Option<NodeId> {
        self.graph
            .raw_nodes()
            .binary_search_by(|rn| rn.weight.id.cmp(kid))
            .ok()
            .map(NodeId::new)
    }

    /// Get the node for the specified crate identifier
    #[inline]
    pub fn node_for_kid(&self, kid: &Kid) -> Option<&Node<N>> {
        self.nid_for_kid(kid).map(|nid| &self.graph[nid])
    }

    /// Get an iterator over the nodes for the members of the workspace
    #[inline]
    pub fn workspace_members(&self) -> impl Iterator<Item = &Node<N>> {
        self.workspace_members
            .iter()
            .filter_map(move |pid| self.nid_for_kid(pid).map(|ind| &self.graph[ind]))
    }
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
    ///     for vs in krates.search_matches(name, req.clone()).map(|(_, kn)| &kn.krate.version) {
    ///         println!("found version {} matching {}!", vs, req);
    ///     }
    /// }
    /// ```
    pub fn search_matches(
        &self,
        name: &str,
        req: semver::VersionReq,
    ) -> impl Iterator<Item = (NodeId, &Node<N>)> {
        self.krates_by_name(name)
            .filter(move |(_, n)| req.matches(n.krate.version()))
    }

    /// Get an iterator over all of the crates in the graph with the given name,
    /// in the case there are multiple versions, or sources, of the crate.
    ///
    /// ```
    /// use krates::Krates;
    ///
    /// fn print_all_versions(krates: &Krates, name: &str) {
    ///     for vs in krates.krates_by_name(name).map(|(_, kn)| &kn.krate.version) {
    ///         println!("found version {}", vs);
    ///     }
    /// }
    /// ```
    pub fn krates_by_name(&self, name: &str) -> impl Iterator<Item = (NodeId, &Node<N>)> {
        let lowest = semver::Version::new(0, 0, 0);

        let raw_nodes = self.graph.raw_nodes();

        let range =
            match raw_nodes.binary_search_by(|node| match node.weight.krate.name().cmp(name) {
                std::cmp::Ordering::Equal => node.weight.krate.version().cmp(&lowest),
                o => o,
            }) {
                Ok(i) | Err(i) => {
                    #[allow(clippy::reversed_empty_ranges)]
                    if i >= raw_nodes.len() || raw_nodes[i].weight.krate.name() != name {
                        0..0
                    } else {
                        // Backtrack until if the crate name matches, as, for instance, 0.0.0-pre
                        // versions will be sorted before a 0.0.0 version
                        let mut begin = i;
                        while begin > 0 && raw_nodes[begin - 1].weight.krate.name() == name {
                            begin -= 1;
                        }

                        let end = raw_nodes[begin..]
                            .iter()
                            .take_while(|kd| kd.weight.krate.name() == name)
                            .count()
                            + begin;

                        begin..end
                    }
                }
            };

        let begin = range.start;
        raw_nodes[range]
            .iter()
            .enumerate()
            .map(move |(i, n)| (NodeId::new(begin + i), &n.weight))
    }
}

impl<N, E> std::ops::Index<NodeId> for Krates<N, E> {
    type Output = N;

    #[inline]
    fn index(&self, id: NodeId) -> &Self::Output {
        &self.graph[id].krate
    }
}

impl<N, E> std::ops::Index<usize> for Krates<N, E> {
    type Output = N;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        &self.graph.raw_nodes()[idx].weight.krate
    }
}
