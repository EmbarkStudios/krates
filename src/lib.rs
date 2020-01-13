//! Transforms the output of `cargo metadata` into a graph, `Krates`,
//! where crates are nodes and dependency links are edges.
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

#![warn(clippy::all)]
#![warn(rust_2018_idioms)]

pub use cargo_metadata as cm;
pub use cfg_expr;
pub use petgraph;

use petgraph::{graph::NodeIndex, Direction};

mod builder;
mod errors;

pub use builder::{Builder, Cmd, Scope};
pub use errors::Error;

/// A crate's unique identifier
pub type Kid = cargo_metadata::PackageId;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DepKind {
    Normal,
    Build,
    Dev,
}

impl From<cm::DependencyKind> for DepKind {
    fn from(dk: cm::DependencyKind) -> Self {
        match dk {
            cm::DependencyKind::Normal => Self::Normal,
            cm::DependencyKind::Build => Self::Build,
            cm::DependencyKind::Development => Self::Dev,
            _ => unreachable!(),
        }
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

pub type NodeId = NodeIndex<u32>;
pub struct Node<N> {
    pub id: Kid,
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

#[derive(Debug, Clone)]
pub struct Edge {
    pub kind: DepKind,
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

pub struct Krates<N = cm::Package, E = Edge> {
    graph: petgraph::Graph<Node<N>, E>,
    workspace_members: Vec<Kid>,
    lock_file: std::path::PathBuf,
}

#[allow(clippy::len_without_is_empty)]
impl<N, E> Krates<N, E> {
    /// The number of unique crates in the graph
    #[inline]
    pub fn len(&self) -> usize {
        self.graph.node_count()
    }

    #[inline]
    pub fn lock_path(&self) -> &std::path::PathBuf {
        &self.lock_file
    }

    #[inline]
    pub fn graph(&self) -> &petgraph::Graph<Node<N>, E> {
        &self.graph
    }

    pub fn krates(&self) -> impl Iterator<Item = &Node<N>> {
        self.graph.node_indices().map(move |nid| &self.graph[nid])
    }

    pub fn get_deps(&self, id: NodeId) -> impl Iterator<Item = (&Node<N>, &E)> {
        use petgraph::visit::EdgeRef;

        self.graph
            .edges_directed(id, Direction::Outgoing)
            .map(move |edge| {
                let krate = &self.graph[edge.target()];
                (krate, edge.weight())
            })
    }

    pub fn nid_for_kid(&self, kid: &Kid) -> Option<NodeId> {
        self.graph
            .raw_nodes()
            .binary_search_by(|rn| rn.weight.id.cmp(kid))
            .ok()
            .map(NodeId::new)
    }

    pub fn node_for_kid(&self, kid: &Kid) -> Option<&Node<N>> {
        self.nid_for_kid(kid).map(|nid| &self.graph[nid])
    }

    pub fn workspace_members(&self) -> impl Iterator<Item = &Node<N>> {
        self.workspace_members
            .iter()
            .filter_map(move |pid| self.nid_for_kid(pid).map(|ind| &self.graph[ind]))
    }
}

pub trait KrateDetails {
    fn name(&self) -> &str;
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

impl<N, E> Krates<N, E>
where
    N: KrateDetails,
{
    pub fn search_matches<'a: 'b, 'b>(
        &'b self,
        name: &'a str,
        req: &'a semver::VersionReq,
    ) -> impl Iterator<Item = (NodeId, &Node<N>)> {
        self.krates_by_name(name)
            .filter(move |(_, n)| req.matches(n.krate.version()))
    }

    pub fn krates_by_name(&self, name: &str) -> impl Iterator<Item = (NodeId, &Node<N>)> {
        let lowest = semver::Version::new(0, 0, 0);

        let raw_nodes = self.graph.raw_nodes();

        let range =
            match raw_nodes.binary_search_by(|node| match node.weight.krate.name().cmp(&name) {
                std::cmp::Ordering::Equal => node.weight.krate.version().cmp(&lowest),
                o => o,
            }) {
                Ok(i) | Err(i) => {
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
