#![allow(dead_code)]

use krates::Kid;
use std::{fmt, path::Path};

#[derive(Debug, PartialEq)]
pub struct JustId(pub Kid);

pub type Graph = krates::Krates<JustId>;

impl From<krates::cm::Package> for JustId {
    fn from(pkg: krates::cm::Package) -> Self {
        Self(pkg.id.into())
    }
}

impl fmt::Display for JustId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.0.name();
        let version = self.0.version();
        let source = self.0.source();
        const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

        write!(f, "{name} {version}")?;
        if source != CRATES_IO {
            const PATH_PREFIX: &str = "path+file://";
            if let Some(path) = source.strip_prefix(PATH_PREFIX) {
                let path = std::path::Path::new(path);

                fn push(f: &mut fmt::Formatter<'_>, path: &std::path::Path) {
                    let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
                        return;
                    };
                    if file_name != "krates" {
                        let Some(parent) = path.parent() else {
                            return;
                        };
                        push(f, parent);
                    }

                    f.write_str("/").unwrap();
                    f.write_str(file_name).unwrap();
                }

                f.write_str(" ")?;
                f.write_str(PATH_PREFIX)?;
                push(f, path);
            } else {
                write!(f, " {source}")?;
            }
        }

        Ok(())
    }
}

pub struct Grafs {
    pub actual: Graph,
    pub filtered: Vec<Kid>,
    pub simple: SimpleGraph,
}

impl Grafs {
    #[inline]
    pub fn dotgraph(&self) -> String {
        krates::petgraph::dot::Dot::new(self.actual.graph()).to_string()
    }
}

pub fn build<P: AsRef<Path>>(src: P, kb: krates::Builder) -> Result<Grafs, String> {
    let contents = std::fs::read_to_string(Path::new("tests").join(src))
        .map_err(|e| format!("failed to load metadata file: {e}"))?;

    let md: krates::cm::Metadata = serde_json::from_str(&contents)
        .map_err(|e| format!("failed to deserialize metadata: {e}"))?;

    let resolved = md.resolve.as_ref().cloned().unwrap();

    let simple = SimpleGraph {
        workspace: md
            .workspace_members
            .iter()
            .map(|wm| Kid::from(wm.clone()))
            .collect(),
        nodes: resolved
            .nodes
            .into_iter()
            .map(|rnode| {
                (
                    rnode.id.into(),
                    rnode
                        .deps
                        .into_iter()
                        .flat_map(|d| {
                            let id = d.pkg;
                            d.dep_kinds.into_iter().map(move |dk| {
                                (
                                    id.clone().into(),
                                    krates::Edge::Dep {
                                        kind: dk.kind.into(),
                                        #[cfg(not(feature = "metadata"))]
                                        cfg: dk.target.clone(),
                                        #[cfg(feature = "metadata")]
                                        cfg: dk.target.map(|s| s.to_string()),
                                    },
                                )
                            })
                        })
                        .collect(),
                )
            })
            .collect(),
    };

    let mut filtered = Vec::new();

    let graph = kb
        .build_with_metadata(md, |f: krates::cm::Package| {
            filtered.push(f.id.into());
        })
        .map_err(|e| format!("failed to build graph: {e}"))?;

    filtered.sort();

    Ok(Grafs {
        actual: graph,
        filtered,
        simple,
    })
}

pub fn is_workspace(kid: &krates::Kid) -> bool {
    kid.repr.starts_with("a ") || kid.repr.starts_with("b ") | kid.repr.starts_with("c ")
}

pub struct SimpleGraph {
    pub nodes: Vec<(krates::Kid, Vec<(krates::Kid, krates::Edge)>)>,
    workspace: Vec<krates::Kid>,
}

impl SimpleGraph {
    fn build<NF: Fn(&krates::Kid) -> bool, EF: Fn(EdgeFilter<'_>) -> bool>(
        mut self,
        nf: NF,
        ef: EF,
    ) -> krates::petgraph::Graph<JustId, krates::Edge> {
        self.nodes.sort_by(|a, b| a.0.cmp(&b.0));

        let mut graph = krates::petgraph::Graph::new();
        let mut edge_map = std::collections::BTreeMap::new();

        let mut pkg_stack = Vec::new();

        for kid in &self.workspace {
            pkg_stack.push(kid);
        }

        while let Some(kid) = pkg_stack.pop() {
            if nf(kid) {
                continue;
            }

            let pkg = &self.nodes[self.nodes.binary_search_by(|(id, _)| id.cmp(kid)).unwrap()];

            let mut edges: Vec<_> = pkg
                .1
                .iter()
                .filter_map(|(pid, edge)| {
                    if ef(EdgeFilter {
                        source: kid,
                        target: pid,
                        dep: if let krates::Edge::Dep { kind, cfg } = edge {
                            Some(EdgeDepFilter {
                                kind: *kind,
                                cfg: cfg.as_deref(),
                            })
                        } else {
                            None
                        },
                    }) {
                        None
                    } else {
                        Some((pid, edge))
                    }
                })
                .collect();

            edges.sort_by(|a, b| a.0.cmp(b.0));

            for kid in edges.iter().map(|(kid, _)| kid) {
                if !edge_map.contains_key(kid) {
                    pkg_stack.push(kid);
                }
            }

            edge_map.insert(kid, edges);
        }

        let mut node_map = std::collections::BTreeMap::new();

        for kid in self.nodes.iter().map(|(id, _)| id) {
            if edge_map.contains_key(kid) {
                node_map.insert(kid, graph.add_node(JustId(kid.clone())));
            }
        }

        for kid in self.nodes.iter().map(|(id, _)| id) {
            if let Some(source) = node_map.get(kid) {
                let edges = edge_map.remove(kid).unwrap();
                for (edge, target) in edges
                    .into_iter()
                    .filter_map(|edge| node_map.get(edge.0).map(|target| (edge, target)))
                {
                    graph.add_edge(*source, *target, edge.1.clone());
                }
            } else {
                println!("filtered {kid}");
            }
        }

        graph
    }
}

pub struct EdgeDepFilter<'a> {
    pub kind: krates::DepKind,
    pub cfg: Option<&'a str>,
}

pub struct EdgeFilter<'a> {
    pub source: &'a krates::Kid,
    pub target: &'a krates::Kid,
    pub dep: Option<EdgeDepFilter<'a>>,
}
