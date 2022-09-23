#![allow(dead_code)]

use std::{fmt, path::Path};

pub struct JustId(krates::Kid);

pub type Graph = krates::Krates<JustId>;

impl From<krates::cm::Package> for JustId {
    fn from(pkg: krates::cm::Package) -> Self {
        Self(pkg.id)
    }
}

impl fmt::Display for JustId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.repr)
    }
}

pub struct Grafs {
    pub actual: Graph,
    pub filtered: Vec<krates::Kid>,
    pub simple: SimpleGraph,
}

pub fn build<P: AsRef<Path>>(src: P, kb: krates::Builder) -> Result<Grafs, String> {
    let contents = std::fs::read_to_string(Path::new("tests").join(src))
        .map_err(|e| format!("failed to load metadata file: {}", e))?;

    let md: krates::cm::Metadata = serde_json::from_str(&contents)
        .map_err(|e| format!("failed to deserialize metadata: {}", e))?;

    let resolved = md.resolve.as_ref().cloned().unwrap();

    let simple = SimpleGraph {
        workspace: md.workspace_members.clone(),
        nodes: resolved
            .nodes
            .into_iter()
            .map(|rnode| {
                (
                    rnode.id,
                    rnode
                        .deps
                        .into_iter()
                        .flat_map(|d| {
                            let id = d.pkg;
                            d.dep_kinds.into_iter().map(move |dk| {
                                (
                                    id.clone(),
                                    krates::Edge::Dep {
                                        kind: dk.kind.into(),
                                        cfg: dk.target.map(|f| f.to_string()),
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
            filtered.push(f.id);
        })
        .map_err(|e| format!("failed to build graph: {}", e))?;

    filtered.sort();

    Ok(Grafs {
        actual: graph,
        filtered,
        simple,
    })
}

#[macro_export]
macro_rules! graph {
    { $($id:expr => [$($did:expr; $kind:ident $(@ $cfg:expr)?),* $(,)?]),+ $(,)? } => {{
        let mut _sg = $crate::util::SimpleGraph {
            nodes: Vec::new(),
        };

        $(
            let mut _deps = Vec::new();

            $(
                let mut _cfg = None;

                $(
                    _cfg = Some($cfg.to_owned());
                )?

                _deps.push(($crate::util::make_kid($did), krates::Edge {
                    kind: krates::DepKind::$kind,
                    cfg: _cfg,
                }));
            )*

            _sg.nodes.push(($crate::util::make_kid($id), _deps));
        )+

        _sg
    }};
}

pub fn is_workspace(kid: &krates::Kid) -> bool {
    kid.repr.starts_with("a ") || kid.repr.starts_with("b ") | kid.repr.starts_with("c ")
}

pub fn make_kid(s: &str) -> krates::Kid {
    let mut i = s.splitn(3, ' ');

    let name = i.next().unwrap();
    let version = i.next().unwrap();
    let source = i.next();

    let source = match name {
        which @ "a" | which @ "b" | which @ "c" => {
            format!("(path+file:///home/jake/code/krates/tests/ws/{})", which)
        }
        _ => source
            .unwrap_or("(registry+https://github.com/rust-lang/crates.io-index)")
            .to_owned(),
    };

    krates::Kid {
        repr: format!("{} {} {}", name, version, source,),
    }
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
        let mut edge_map = std::collections::HashMap::new();

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

        let mut node_map = std::collections::HashMap::new();

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
                println!("filtered {}", kid);
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

pub fn cmp<NF: Fn(&krates::Kid) -> bool, EF: Fn(EdgeFilter<'_>) -> bool>(
    grafs: Grafs,
    node_filter: NF,
    edge_filter: EF,
) {
    let expected = grafs.simple.build(node_filter, edge_filter);

    use krates::petgraph::dot::Dot;

    let actual = Dot::new(&grafs.actual.graph()).to_string();

    std::fs::write("graph.dot", &actual).unwrap();

    similar_asserts::assert_str_eq!(
        Dot::new(&expected),
        actual,
        "filtered: {:#?}",
        grafs.filtered
    );
}
