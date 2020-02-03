use crate::{DepKind, Edge, Error, Kid, Krates};
use cargo_metadata as cm;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// An alternative to cargo_metadata::MetadataCommand which allows correct
/// feature usage, as well as ensuring that the command can run successfully
/// regardless of where it is executed and on what.
#[derive(Default, Debug)]
pub struct Cmd {
    cargo_path: Option<PathBuf>,
    manifest_path: Option<PathBuf>,
    current_dir: Option<PathBuf>,
    features: Vec<String>,
    other_options: Vec<String>,
    all_features: bool,
    no_default_features: bool,
}

impl Cmd {
    pub fn new() -> Self {
        Self::default()
    }

    /// Path to `cargo` executable.  If not set, this will use the the `$CARGO`
    /// environment variable, and if that is not set, will simply be `cargo`.
    pub fn cargo_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.cargo_path = Some(path.into());
        self
    }

    /// Path to a `Cargo.toml` manifest
    pub fn manifest_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.manifest_path = Some(path.into());
        self
    }

    /// Current directory of the `cargo metadata` process.
    pub fn current_dir(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.current_dir = Some(path.into());
        self
    }

    /// Disables default features.
    ///
    /// **NOTE**: This has **no effect** if used on a workspace. You must
    /// specify a working directory or manifest path to a specific crate if used
    /// on a crate inside a workspace.
    pub fn no_default_features(&mut self) -> &mut Self {
        self.no_default_features = true;
        self
    }

    /// Enables all features for all workspace crates. Usable on both individual
    /// crates and on an entire workspace.
    pub fn all_features(&mut self) -> &mut Self {
        self.all_features = true;
        self
    }

    /// Enables specific features. See the **NOTE** for `no_default_features`
    pub fn features(&mut self, feats: impl IntoIterator<Item = String>) -> &mut Self {
        self.features.extend(feats);
        self
    }

    /// Arbitrary command line flags to pass to `cargo`.  These will be added to
    /// the end of the command line invocation.
    pub fn other_options(&mut self, options: impl IntoIterator<Item = String>) -> &mut Self {
        self.other_options.extend(options);
        self
    }
}

impl Into<cm::MetadataCommand> for Cmd {
    fn into(mut self) -> cm::MetadataCommand {
        let mut mdc = cm::MetadataCommand::new();

        if let Some(cp) = self.cargo_path {
            mdc.cargo_path(cp);
        }

        // If the manifest path is set, we force set the current working
        // directory to its parent and use the relative path, this is to fix an
        // edge case where you can run cargo metadata from a directory outside
        // of a workspace which could fail if eg. there is a reference to a
        // registry that is defined in the workspace's .cargo/config
        if let Some(mp) = self.manifest_path {
            self.current_dir = Some(mp.parent().unwrap().to_owned());
            mdc.manifest_path("Cargo.toml");
        }

        if let Some(cd) = self.current_dir {
            mdc.current_dir(cd);
        }

        // Everything else we specify as additional options, as MetadataCommand
        // does not handle features correctly, eg. you cannot disable default
        // and set specific ones at the same time
        // https://github.com/oli-obk/cargo_metadata/issues/79
        self.features.sort();
        self.features.dedup();

        let mut opts = Vec::with_capacity(
            self.features.len()
                + self.other_options.len()
                + if self.no_default_features { 1 } else { 0 }
                + if self.all_features { 1 } else { 0 },
        );

        if self.no_default_features {
            opts.push("--no-default-features".to_owned());
        }

        if self.all_features {
            opts.push("--all-features".to_owned());
        }

        if !self.features.is_empty() {
            opts.push("--features".to_owned());
            opts.append(&mut self.features);
        }

        opts.append(&mut self.other_options);
        mdc.other_options(opts);

        mdc
    }
}

enum TargetFilter {
    Known(&'static cfg_expr::targets::TargetInfo, Vec<String>),
    Unknown(String, Vec<String>),
}

/// The scope for which a dependency kind will be ignored
#[derive(Clone, Copy)]
pub enum Scope {
    /// Will match a dependency from a crate in the workspace
    Workspace,
    /// Will match a dependency from any crate not in the workspace
    NonWorkspace,
    /// Will ignore a dependency from any crate
    All,
}

/// Trait used to report back any crates that are completely ignored in the
/// final crate graph that is built. This occurs when the crate has no
/// dependents any longer due to the applied filters.
pub trait OnFilter {
    fn filtered(&mut self, krate: cm::Package);
}

/// For when you just want to satisfy OnFilter without doing anything
pub struct NoneFilter;
impl OnFilter for NoneFilter {
    fn filtered(&mut self, _: cm::Package) {}
}

impl<F> OnFilter for F
where
    F: FnMut(cm::Package),
{
    fn filtered(&mut self, krate: cm::Package) {
        self(krate)
    }
}

/// A builder used to create a Krates graph, either by running a cargo metadata
/// command, or using an already deserialized `cargo_metadata::Metadata`
#[derive(Default)]
pub struct Builder {
    target_filters: Vec<TargetFilter>,
    workspace_filters: Vec<PathBuf>,
    ignore_kinds: u32,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ignores a specific dependency kind in the given scope.
    ///
    /// ```
    /// # use krates::{Builder, DepKind, Scope};
    /// Builder::new().ignore_kind(DepKind::Build, Scope::NonWorkspace);
    /// ```
    ///
    /// In the above example, let's say we depended on `zstd`. zstd depends on
    /// the `cc` crate (`zstd -> zstd-safe -> zstd-sys -> cc`) for building
    /// C code. By ignoring the `build` kind for non-workspace crates, the link
    /// from `zstd-sys` -> `cc` will be filtered out. If the same `cc` is not
    /// depended on by a crate in the workspace, `cc` will not end up in the
    /// final `Krates` graph.
    ///
    /// Note that ignoring `DepKind::Dev` for `Scope::NonWorkspace` is
    /// meaningless as dev dependencies are not resolved by cargo for transitive
    /// dependencies.
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

    /// By default, every workspace crate is treated as a root node and implicitly
    /// added to the graph if the graph is built from a workspace context and not
    /// a specific crate in the workspace.
    ///
    /// By default, every workspace crate is treated as a root node and
    /// implicitly added to the graph if the graph is built from a workspace
    /// context and not a specific crate in the workspace.
    ///
    /// By using this method, only the workspace crates whose Cargo.toml path
    /// matches one of the specified crates will be added as root nodes, meaning
    /// that any workspace crate not in the list that doesn't have any
    /// dependendents on a workspace crate that does, will no longer appear in
    /// the graph.
    ///
    /// If you specify only a single path, and that path is actually to a
    /// a workspace's virtual manifest, the graph will be the same as if
    /// invlude_workspace_crates was not specified.
    ///
    /// ```
    /// # use krates::{Builder, DepKind, Scope};
    /// Builder::new().include_workspace_crates(&["path/to/some/crate"]);
    /// ```
    pub fn include_workspace_crates<P, I>(&mut self, crates: I) -> &mut Self
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        self.workspace_filters.extend(crates.into_iter().map(|p| {
            // It's the users's responsibility to give proper paths here, as
            // we can't rely on eg canonicalization since we might not be on
            // the same filesystem any more. We do fixup the ensure they point
            // at a Cargo.toml however
            let p = p.as_ref();

            // Attempt to canonicalize the path, which might not work if the
            // user is attempting to add a path to filter from another
            // filesystem or similar
            let p = p.canonicalize().unwrap_or_else(|_| p.to_owned());

            if !p.ends_with("Cargo.toml") {
                p.join("Cargo.toml")
            } else {
                p
            }
        }));
        self
    }

    /// By default, cargo resolves all target specific dependencies. Optionally,
    /// you can use the `--filter-platform` option on `cargo metadata` to
    /// resolve only dependencies that match the specified target, but it can
    /// only do this for one platlform.
    ///
    /// By using this method, you can specify one or more targets by their
    /// triple, as well as any
    /// [`target_features`](https://doc.rust-lang.org/reference/attributes/codegen.html#the-target_feature-attribute)
    /// that you
    /// [promise](https://doc.rust-lang.org/reference/behavior-considered-undefined.html)
    /// are enabled for that target to filter dependencies by. If any of the
    /// specified targets matches a target specific dependency, it will be
    /// included in the graph.
    ///
    /// When specifying a target triple, only builtin targets of rustc
    /// (as of 1.40) can be used to evaluate `cfg()` expressions. If the triple
    /// is not recognized, it will only be evaluated against
    /// `[target.<triple-or-json>.<|build-|dev->dependencies]`.
    ///
    /// ```
    /// # use krates::{Builder, DepKind, Scope};
    /// let targets = [
    ///     // the big 3
    ///     "x86_64-unknown-linux-gnu",
    ///     "x86_64-pc-windows-msvc",
    ///     "x86_64-apple-darwin",
    ///     // and musl!
    ///     "x86_64-unknown-linux-musl",
    ///     // and wasm (with the fancy atomics feature!)
    ///     "wasm32-unknown-unknown",
    /// ];
    ///
    /// Builder::new().include_targets(targets.into_iter().map(|triple| {
    ///     if triple.starts_with("wasm32") {
    ///         (*triple, vec!["atomics".to_owned()])
    ///     } else {
    ///         (*triple, vec![])
    ///     }
    /// }));
    /// ```
    pub fn include_targets<S: AsRef<str>>(
        &mut self,
        targets: impl IntoIterator<Item = (S, Vec<String>)>,
    ) -> &mut Self {
        self.target_filters
            .extend(targets.into_iter().map(|(triple, features)| {
                match cfg_expr::targets::get_target_by_triple(triple.as_ref()) {
                    Some(ti) => TargetFilter::Known(ti, features),
                    None => TargetFilter::Unknown(triple.as_ref().to_owned(), features),
                }
            }));
        self
    }

    /// Builds a `Krates` graph using metadata that be retrieved via the
    /// specified metadata command. If `on_filter` is specified, it will be
    /// called with each package that was filtered from the graph, if any.
    ///
    /// This method will fail if the metadata command fails for some reason, or
    /// if the command specifies `--no-deps` which means there will be no
    /// resolution graph to build our graph from.
    ///
    /// ```no_run
    /// # use krates::cm::Package;
    /// let mut mdc = krates::Cmd::new();
    /// mdc.manifest_path("path/to/Cargo.toml");
    ///
    /// if /*no_default_features*/ true {
    ///     mdc.no_default_features();
    /// }
    ///
    /// if /*cfg.all_features*/ false {
    ///     mdc.all_features();
    /// }
    ///
    /// mdc.features(
    ///     ["cool-feature", "cooler-feature", "coolest-feature"]
    ///         .iter()
    ///         .map(|s| s.to_string()),
    /// );
    ///
    /// let mut builder = krates::Builder::new();
    ///
    /// if /*cfg.ignore_build_dependencies*/ false {
    ///     builder.ignore_kind(krates::DepKind::Build, krates::Scope::All);
    /// }
    ///
    /// if /*cfg.ignore_dev_dependencies*/ true {
    ///     builder.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
    /// }
    ///
    /// let graph: krates::Krates = builder.build(
    ///     mdc,
    ///     |filtered: Package| match filtered.source {
    ///         Some(src) => {
    ///             if src.is_crates_io() {
    ///                 println!("filtered {} {}", filtered.name, filtered.version);
    ///             } else {
    ///                 println!("filtered {} {} {}", filtered.name, filtered.version, src);
    ///             }
    ///         }
    ///         None => println!("filtered crate {} {}", filtered.name, filtered.version),
    ///     },
    /// ).unwrap();
    /// ```
    pub fn build<N, E, F>(
        self,
        cmd: impl Into<cm::MetadataCommand>,
        on_filter: F,
    ) -> Result<Krates<N, E>, Error>
    where
        N: From<cargo_metadata::Package>,
        E: From<Edge>,
        F: OnFilter,
    {
        let metadata = cmd.into().exec()?;
        self.build_with_metadata(metadata, on_filter)
    }

    /// Builds a `Krates` graph using the specified metadata. If `on_filter` is
    /// specified, it will be called with each package that was filtered from
    /// the graph, if any.
    ///
    /// The metadata **must** have resolved dependencies for the graph to be
    /// built, so not having it is the only way this method will fail.
    ///
    /// ```no_run
    /// # use krates::{Krates, Builder, DepKind, Scope, cm::Package};
    /// let contents = std::fs::read_to_string("metadata.json")
    ///     .map_err(|e| format!("failed to load metadata file: {}", e)).unwrap();
    ///
    /// let md: krates::cm::Metadata = serde_json::from_str(&contents)
    ///     .map_err(|e| format!("failed to deserialize metadata: {}", e)).unwrap();
    ///
    /// let krates: Krates = Builder::new().build_with_metadata(
    ///     md,
    ///     |pkg: Package| println!("filtered {}", pkg.id)
    /// ).unwrap();
    ///
    /// println!("found {} unique crates", krates.len());
    /// ```
    pub fn build_with_metadata<N, E, F>(
        self,
        md: cargo_metadata::Metadata,
        mut on_filter: F,
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

        // Only include workspaces members the user wants if they have
        // specified any, this is to take into account scenarios where
        // you have a large workspace, but only want to get the crates
        // used by a subset of the workspace
        if self.workspace_filters.is_empty() {
            // If the resolve graph specifies a root, it means the user specified
            // a particular crate in a workspace, so we'll only use that single
            // root for the entire graph rather than a root for each workspace
            // member crate
            match &resolved.root {
                Some(root) => pid_stack.push(root),
                None => pid_stack.extend(workspace_members.iter()),
            }
        } else {
            // If the filters only contain 1 path, and it is the path to a
            // workspace toml, then we disregard the filters
            if self.workspace_filters.len() == 1
                && Some(md.workspace_root.as_ref()) == self.workspace_filters[0].parent()
            {
                pid_stack.extend(workspace_members.iter());
            } else {
                for wm in &workspace_members {
                    if let Ok(i) = packages.binary_search_by(|pkg| pkg.id.cmp(wm)) {
                        if self
                            .workspace_filters
                            .iter()
                            .any(|wf| wf == &packages[i].manifest_path)
                        {
                            pid_stack.push(wm);
                        }
                    }
                }
            }
        }

        let include_all_targets = self.target_filters.is_empty();
        let ignore_kinds = self.ignore_kinds;
        let targets = self.target_filters;

        #[derive(Debug)]
        struct DepKindInfo {
            kind: DepKind,
            cfg: Option<String>,
        }

        #[derive(Debug)]
        // We use our resolution nodes because cargo_metadata uses
        // non-exhaustive everywhere :p
        struct NodeDep {
            //name: String,
            pkg: Kid,
            dep_kinds: Vec<DepKindInfo>,
        }

        #[derive(Debug)]
        struct Node {
            id: Kid,
            deps: Vec<NodeDep>,
            // We don't use this for now, but maybe we should expose it on each
            // crate?
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
