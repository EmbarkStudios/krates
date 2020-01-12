use crate::{DepKind, Edge, Error, Kid, Krates};
use cargo_metadata as cm;
use std::collections::HashMap;

enum TargetFilter {
    Known(&'static cfg_expr::targets::TargetInfo, Vec<String>),
    Unknown(String, Vec<String>),
}

pub enum Unless {
    IsWorkspace,
    IsNotWorkspace,
}

impl Into<bool> for Unless {
    fn into(self) -> bool {
        match self {
            Unless::IsWorkspace => true,
            Unless::IsNotWorkspace => false,
        }

enum TargetFilter {
    Known(&'static cfg_expr::targets::TargetInfo, Vec<String>),
    Unknown(String, Vec<String>),
}

pub enum Scope {
    Workspace,
    NonWorkspace,
    All,
}

pub trait OnFilter {
    fn filtered(&mut self, krate: cm::Package);
}

impl<F> OnFilter for F
where
    F: FnMut(cm::Package),
{
    fn filtered(&mut self, krate: cm::Package) {
        self(krate)
    }
}

#[derive(Default)]
pub struct Builder {
    target_filters: Vec<TargetFilter>,
    ignore_kinds: u32,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ignore_kind(&mut self, kind: DepKind, scope: Scope) -> &mut Self {
        let kind_flag = match kind {
            DepKind::Normal => 0x1,
            DepKind::Dev => 0x8,
            DepKind::Build => 0x40,
        };

        self.ignore_kinds |= kind_flag;

        self.ignore_kinds |= match scope {
            Scope::Workspace => kind_flag << 1,
            Scope::NonWorkspace => kind_flag << 2,
            Scope::All => kind_flag << 1 | kind_flag << 2,
        };

        self
    }

        self
    }

    pub fn include_target(&mut self, triple: String, features: Vec<String>) -> &mut Self {
        let tf = match cfg_expr::targets::get_target_by_triple(&triple) {
            Some(ti) => TargetFilter::Known(ti, features),
            None => TargetFilter::Unknown(triple, features),
        };

        self.target_filters.push(tf);
        self
    }

    pub fn build<N, E, F>(
        self,
        mut cmd: cargo_metadata::MetadataCommand,
        on_filter: Option<F>,
    ) -> Result<Krates<N, E>, Error>
    where
        N: From<cargo_metadata::Package>,
        E: From<Edge>,
        F: OnFilter,
    {
        let metadata = cmd.exec()?;
        self.build_with_metadata(metadata, on_filter)
    }

    pub fn build_with_metadata<N, E, F>(
        self,
        md: cargo_metadata::Metadata,
        mut on_filter: Option<F>,
    ) -> Result<Krates<N, E>, Error>
    where
        N: From<cargo_metadata::Package>,
        E: From<Edge>,
        F: OnFilter,
    {
        let resolved = md.resolve.ok_or_else(|| Error::NoResolveGraph)?;

        let mut packages = md.packages;
        packages.sort_by(|a, b| a.id.cmp(&b.id));

        let mut workspace_members = md.workspace_members;
        workspace_members.sort();

        let mut edge_map = HashMap::new();
        let mut pid_stack = Vec::with_capacity(workspace_members.len());
        pid_stack.extend(workspace_members.iter());

        let include_all_targets = self.target_filters.is_empty();
        let ignore_kinds = self.ignore_kinds;
        let targets = self.target_filters;

        struct DepKindInfo {
            kind: DepKind,
            cfg: Option<String>,
        }

        // We use our resolution nodes because cargo_metadata uses non-exhaustive everywhere :p
        struct NodeDep {
            //name: String,
            pkg: Kid,
            dep_kinds: Vec<DepKindInfo>,
        }

        struct Node {
            id: Kid,
            deps: Vec<NodeDep>,
            // We don't use this for now, but maybe we should expose it on each crate?
            // features: Vec<String>,
        }

        let mut nodes: Vec<_> = resolved
            .nodes
            .into_iter()
            .map(|rn| {
                let krate = &packages[packages.binary_search_by(|k| k.id.cmp(&rn.id)).unwrap()];

                let deps = rn
                    .deps
                    .into_iter()
                    .map(|dn| {
                        // We can't rely on the user using cargo from 1.41+ at least for a little bit,
                        // so use a fallback for now. Maybe eventually can do a breaking change to require
                        // 1.41 so this is nicer
                        let dep_kinds = if dn.dep_kinds.is_empty() {
                            let name = &dn.pkg.repr[..dn.pkg.repr.find(' ').unwrap()];

                            krate
                                .dependencies
                                .iter()
                                .filter_map(|dep| {
                                    if name == dep.name {
                                        Some(DepKindInfo {
                                            kind: dep.kind.into(),
                                            cfg: dep.target.as_ref().map(|t| format!("{}", t)),
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        } else {
                            dn.dep_kinds
                                .into_iter()
                                .map(|dk| DepKindInfo {
                                    kind: dk.kind.into(),
                                    cfg: dk.target.map(|t| format!("{}", t)),
                                })
                                .collect()
                        };

                        NodeDep {
                            //name: dn.name,
                            pkg: dn.pkg,
                            dep_kinds,
                        }
                    })
                    .collect();

                Node { id: rn.id, deps }
            })
            .collect();

        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        while let Some(pid) = pid_stack.pop() {
            let is_in_workspace = workspace_members.binary_search(&pid).is_ok();

            let krate_index = nodes.binary_search_by(|n| n.id.cmp(&pid)).unwrap();

            let rnode = &nodes[krate_index];
            let krate = &packages[krate_index];

            debug_assert!(rnode.id == krate.id);

            // Though each unique dependency can only be resolved once, it's possible
            // for the crate to list the same dependency multiple times, with different
            // dependency kinds, or different target configurations, so each one gets its
            // own edge
            let edges: Vec<_> = rnode
                .deps
                .iter()
                .flat_map(|rdep| {
                    let targets = &targets;
                    rdep.dep_kinds.iter().filter_map(move |dk| {
                        let mask = match dk.kind {
                            DepKind::Normal => 0x1,
                            DepKind::Dev => 0x8,
                            DepKind::Build => 0x40,
                        };

                        let mask = mask | mask << if is_in_workspace { 1 } else { 2 };
                        if mask & ignore_kinds == mask {
                            return None;
                        }

                        match &dk.cfg {
                            None => Some((
                                Edge {
                                    kind: dk.kind,
                                    cfg: None,
                                },
                                &rdep.pkg,
                            )),
                            Some(cfg) => {
                                if include_all_targets {
                                    return Some((
                                        Edge {
                                            kind: dk.kind,
                                            cfg: Some(cfg.to_owned()),
                                        },
                                        &rdep.pkg,
                                    ));
                                }

                                let matched = if cfg.starts_with("cfg(") {
                                    match cfg_expr::Expression::parse(&cfg) {
                                        Ok(expr) => {
                                            // We only need to focus on target predicates because they are
                                            // the only type of predicate allowed by cargo at the moment

                                            // While it might be nicer to evaluate all the targets for each predicate
                                            // it would lead to weird situations where an expression could evaluate to true
                                            // (or false) with a combination of platform, that would otherwise by impossible,
                                            // eg cfg(all(windows, target_env = "musl")) could evaluate to true
                                            targets.iter().any(|target| {
                                                expr.eval(|pred| match pred {
                                                    cfg_expr::expr::Predicate::Target(tp) => {
                                                        if let TargetFilter::Known(ti, _) = target {
                                                            tp.matches(ti)
                                                        } else {
                                                            false
                                                        }
                                                    }
                                                    cfg_expr::expr::Predicate::TargetFeature(
                                                        feat,
                                                    ) => {
                                                        let features = match target {
                                                            TargetFilter::Known(_, f) => f,
                                                            TargetFilter::Unknown(_, f) => f,
                                                        };

                                                        // TODO: target_features are extremely rare in cargo.toml
                                                        // files, it might be a good idea to inform the user of this
                                                        // somehow, filteredare unsure why a particular dependency
                                                        // is being filtered
                                                        features.iter().any(|f| f == feat)
                                                    }
                                                    // We *could* warn here about an invalid expression, but
                                                    // presumably cargo will be responsible for that so don't bother
                                                    _ => false,
                                                })
                                            })
                                        }
                                        Err(_pe) => {
                                            // TODO: maybe log a warning if we somehow fail to parse the cfg?
                                            true
                                        }
                                    }
                                } else {
                                    targets.iter().any(|target| match target {
                                        TargetFilter::Known(ti, _) => ti.triple == cfg,
                                        TargetFilter::Unknown(t, _) => t.as_str() == cfg,
                                    })
                                };

                                if matched {
                                    Some((
                                        Edge {
                                            kind: dk.kind,
                                            cfg: Some(cfg.to_owned()),
                                        },
                                        &rdep.pkg,
                                    ))
                                } else {
                                    None
                                }
                            }
                        }
                    })
                })
                .collect();

            for pid in edges.iter().map(|(_, pid)| pid) {
                if !edge_map.contains_key(pid) {
                    pid_stack.push(pid);
                }
            }

            edge_map.insert(pid, edges);
        }

        let mut graph = petgraph::Graph::<crate::Node<N>, E>::new();
        graph.reserve_nodes(packages.len());

        let mut edge_count = 0;

        // Preserve the ordering of the krates when inserting them into the graph
        if let Some(ref mut on_filter) = on_filter {
            for krate in packages {
                if let Some(edges) = edge_map.get(&krate.id) {
                    let id = krate.id.clone();
                    let krate = crate::Node {
                        id,
                        krate: N::from(krate),
                    };

                    graph.add_node(krate);
                    edge_count += edges.len();
                } else {
                    on_filter.filtered(krate);
                }
            }
        } else {
            for krate in packages {
                if let Some(edges) = edge_map.get(&krate.id) {
                    let id = krate.id.clone();
                    let krate = crate::Node {
                        id,
                        krate: N::from(krate),
                    };

                    graph.add_node(krate);
                    edge_count += edges.len();
                }
            }
        }

        graph.reserve_edges(edge_count);

        let get = |graph: &petgraph::Graph<crate::Node<N>, E>, id: &Kid| -> crate::NodeId {
            crate::NodeId::new(
                graph
                    .raw_nodes()
                    .binary_search_by(|n| n.weight.id.cmp(id))
                    .unwrap(),
            )
        };

        // Keep edges ordered as well
        for srcind in 0..graph.node_count() {
            let srcid = crate::NodeId::new(srcind);
            if let Some(edges) = edge_map.remove(&graph[srcid].id) {
                for (dep, tid) in edges {
                    let target = get(&graph, &tid);
                    graph.add_edge(srcid, target, E::from(dep));
                }
            }
        }

        Ok(Krates {
            graph,
            workspace_members,
            lock_file: md.workspace_root.join("Cargo.lock"),
        })
    }
}
