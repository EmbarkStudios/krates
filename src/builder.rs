pub(crate) mod features;

pub mod index;

use crate::{DepKind, Edge, Error, Kid, Krates};
use cargo_metadata as cm;
use features::{Feature, ParsedFeature};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

/// An alternative to [`cargo_metadata::MetadataCommand`] which allows correct
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
    frozen: bool,
    locked: bool,
    offline: bool,
}

#[derive(Copy, Clone)]
pub struct LockOptions {
    /// Requires that the Cargo.lock file is up-to-date. If the lock file is
    /// missing, or it needs to be updated, Cargo will exit with an error.
    /// Prevents Cargo from attempting to access the network to determine if it
    /// is out-of-date.
    pub frozen: bool,
    /// Requires that the Cargo.lock file is up-to-date. If the lock file is
    /// missing, or it needs to be updated, Cargo will exit with an error.
    pub locked: bool,
    /// Prevents Cargo from accessing the network for any reason. Without this
    /// flag, Cargo will stop with an error if it needs to access the network
    /// and the network is not available. With this flag, Cargo will attempt to
    /// proceed without the network if possible.
    ///
    /// Beware that this may result in different dependency resolution than
    /// online mode. Cargo will restrict itself to crates that are downloaded
    /// locally, even if there might be a newer version as indicated in the
    /// local copy of the index. See the [cargo fetch](https://doc.rust-lang.org/cargo/commands/cargo-fetch.html)
    /// command to download dependencies before going offline.
    pub offline: bool,
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

    /// Sets the various [lock options](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html#manifest-options)
    /// for determining if cargo can access the network and if the lockfile must
    /// be present and can be modified
    pub fn lock_opts(&mut self, lopts: LockOptions) -> &mut Self {
        self.frozen = lopts.frozen;
        self.locked = lopts.locked;
        self.offline = lopts.offline;
        self
    }

    /// Arbitrary command line flags to pass to `cargo`.  These will be added to
    /// the end of the command line invocation.
    pub fn other_options(&mut self, options: impl IntoIterator<Item = String>) -> &mut Self {
        self.other_options.extend(options);
        self
    }
}

#[allow(clippy::fallible_impl_from)]
impl From<Cmd> for cm::MetadataCommand {
    fn from(mut cmd: Cmd) -> cm::MetadataCommand {
        let mut mdc = cm::MetadataCommand::new();

        if let Some(cp) = cmd.cargo_path {
            mdc.cargo_path(cp);
        }

        // If the manifest path is set, we force set the current working
        // directory to its parent and use the relative path, this is to fix an
        // edge case where you can run cargo metadata from a directory outside
        // of a workspace which could fail if eg. there is a reference to a
        // registry that is defined in the workspace's .cargo/config
        if let Some(mp) = cmd.manifest_path {
            cmd.current_dir = Some(mp.parent().unwrap().to_owned());
            mdc.manifest_path("Cargo.toml");
        }

        if let Some(cd) = cmd.current_dir {
            if !cd.as_os_str().is_empty() {
                mdc.current_dir(cd);
            } else {
                mdc.current_dir(".");
            }
        }

        // Everything else we specify as additional options, as MetadataCommand
        // does not handle features correctly, eg. you cannot disable default
        // and set specific ones at the same time
        // https://github.com/oli-obk/cargo_metadata/issues/79
        cmd.features.sort();
        cmd.features.dedup();

        let mut opts = Vec::with_capacity(
            cmd.features.len()
                + cmd.other_options.len()
                + usize::from(cmd.no_default_features)
                + usize::from(cmd.all_features),
        );

        if cmd.no_default_features {
            opts.push("--no-default-features".to_owned());
        }

        if cmd.all_features {
            opts.push("--all-features".to_owned());
        }

        if !cmd.features.is_empty() {
            opts.push("--features".to_owned());
            opts.push(cmd.features.join(" "));
        }

        if cmd.frozen {
            opts.push("--frozen".to_owned());
        }

        if cmd.locked {
            opts.push("--locked".to_owned());
        }

        if cmd.offline {
            opts.push("--offline".to_owned());
        }

        opts.append(&mut cmd.other_options);
        mdc.other_options(opts);

        mdc
    }
}

#[derive(Clone)]
pub enum Target {
    Builtin(&'static cfg_expr::targets::TargetInfo),
    #[cfg(feature = "targets")]
    Triple(cfg_expr::target_lexicon::Triple),
    Unknown(String),
}

impl<T> From<T> for Target
where
    T: AsRef<str>,
{
    fn from(triple: T) -> Self {
        let triple = triple.as_ref();
        match cfg_expr::targets::get_builtin_target_by_triple(triple) {
            Some(bi) => Self::Builtin(bi),
            None => {
                #[cfg(feature = "targets")]
                {
                    match triple.parse() {
                        Ok(triple) => Self::Triple(triple),
                        Err(_) => Self::Unknown(triple.to_owned()),
                    }
                }

                #[cfg(not(feature = "targets"))]
                Self::Unknown(triple.to_owned())
            }
        }
    }
}

use std::fmt;

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Builtin(bi) => f.write_str(bi.triple.as_str()),
            #[cfg(feature = "targets")]
            Self::Triple(trip) => write!(f, "{}", trip),
            Self::Unknown(unknown) => f.write_str(unknown),
        }
    }
}

struct TargetFilter {
    inner: Target,
    features: Vec<String>,
}

impl TargetFilter {
    fn eval(&self, predicate: &cfg_expr::Predicate<'_>) -> bool {
        match predicate {
            cfg_expr::expr::Predicate::Target(tp) => match &self.inner {
                Target::Builtin(bi) => tp.matches(*bi),
                #[cfg(feature = "targets")]
                Target::Triple(trip) => tp.matches(trip),
                Target::Unknown(_) => false,
            },
            cfg_expr::expr::Predicate::TargetFeature(feat) => {
                // TODO: target_features are extremely rare in cargo.toml
                // files, it might be a good idea to inform the user of this
                // somehow, if they are unsure why a particular dependency
                // is being filtered
                self.features.iter().any(|f| f == feat)
            }
            // We *could* warn here about an invalid expression, but
            // presumably cargo will be responsible for that, so don't bother
            _ => false,
        }
    }

    fn matches_triple(&self, triple: &str) -> bool {
        match &self.inner {
            Target::Builtin(bi) => bi.triple.as_str() == triple,
            #[cfg(feature = "targets")]
            Target::Triple(trip) => {
                let as_triple = format!("{}", trip);
                as_triple == triple
            }
            Target::Unknown(unknown) => unknown == triple,
        }
    }
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

/// For when you just want to satisfy [`OnFilter`] without doing anything
pub struct NoneFilter;
impl OnFilter for NoneFilter {
    fn filtered(&mut self, _: cm::Package) {}
}

impl<F> OnFilter for F
where
    F: FnMut(cm::Package),
{
    fn filtered(&mut self, krate: cm::Package) {
        self(krate);
    }
}

/// A builder used to create a Krates graph, either by running a cargo metadata
/// command, or using an already deserialized [`cargo_metadata::Metadata`]
#[derive(Default)]
pub struct Builder {
    target_filters: Vec<TargetFilter>,
    workspace_filters: Vec<PathBuf>,
    exclude: Vec<crate::PkgSpec>,
    ignore_kinds: u32,
    workspace: bool,
    crates_io_index: Option<index::BuildIndexCache>,
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
    /// Note that ignoring [`DepKind::Dev`] for [`Scope::NonWorkspace`] is
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

    /// By default, the response from `cargo metadata` determines what the
    /// root(s) of the crate graph will be. If the Cargo.toml path used is a
    /// virtual manifest, then each workspace member will be used as a root. If
    /// the manifest path is for a single crate, or a non-virtual manifest
    /// inside a workspace, then only that single crate will be used as the
    /// root, and in the workspace case, only other workspace members that are
    /// dependencies of that root crate, directly or indirectly, will be
    /// included in the final graph.
    ///
    /// Setting workspace = true will change that default behavior, and instead
    /// include all workspace crates (unless they are filtered via other
    /// methods) even if the manifest path is not a virtual manifest inside
    /// a workspace
    ///
    /// ```
    /// # use krates::Builder;
    /// Builder::new().workspace(true);
    /// ```
    pub fn workspace(&mut self, workspace: bool) -> &mut Self {
        self.workspace = workspace;
        self
    }

    /// Package specification(s) to exclude from the final graph. Unlike with
    /// cargo, each exclusion spec can apply to more than 1 instance of a
    /// package, eg if multiple crates are sourced from the same url, or
    /// multiple versions of the same crate
    ///
    /// ```
    /// # use krates::Builder;
    /// Builder::new().exclude(["a-crate:0.1.0"].iter().map(|spec| spec.parse().unwrap()));
    /// ```
    pub fn exclude<I>(&mut self, exclude: I) -> &mut Self
    where
        I: IntoIterator<Item = crate::PkgSpec>,
    {
        self.exclude.extend(exclude);
        self
    }

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
    /// [`Builder::include_workspace_crates`] was not specified.
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
    /// triple, as well as any [`target_features`](https://doc.rust-lang.org/reference/attributes/codegen.html#the-target_feature-attribute)
    /// that you [promise](https://doc.rust-lang.org/reference/behavior-considered-undefined.html)
    /// are enabled for that target to filter dependencies by. If any of the
    /// specified targets matches a target specific dependency, it will be
    /// included in the graph.
    ///
    /// When specifying a target triple, only builtin targets of rustc can be
    /// used to evaluate `cfg()` expressions. If the triple is not recognized,
    /// it will only be evaluated against `[target.<triple-or-json>.<|build-|dev->dependencies]`.
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
    /// Builder::new().include_targets(targets.iter().map(|triple| {
    ///     if triple.starts_with("wasm32") {
    ///         (*triple, vec!["atomics".to_owned()])
    ///     } else {
    ///         (*triple, vec![])
    ///     }
    /// }));
    /// ```
    pub fn include_targets<S: Into<Target>>(
        &mut self,
        targets: impl IntoIterator<Item = (S, Vec<String>)>,
    ) -> &mut Self {
        self.target_filters
            .extend(targets.into_iter().map(|(triple, features)| TargetFilter {
                inner: triple.into(),
                features,
            }));
        self
    }

    /// Configures the index implementation
    ///
    /// This method allows overriding the location of your `CARGO_HOME`, but note
    /// that no fetching from the remote index is performed by this library, so
    /// it is your responsibility to have called `cargo fetch` or similar to have
    /// an up to date index cache at the location provided
    ///
    /// This method takes into account the local environment to open the correct
    /// crates.io registry, or replacement registry if the user has configured that
    ///
    /// You can force the usage of the git registry (if not overridden in the
    /// local environment) by specifying `Some("1.69.0")` or lower semver as the
    /// sparse registry was not made the default until `1.70.0`
    #[inline]
    pub fn with_crates_io_index(&mut self, index_cache_build: index::BuildIndexCache) -> &mut Self {
        self.crates_io_index = Some(index_cache_build);
        self
    }

    /// Builds a [`Krates`] graph using metadata that be retrieved via the
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

    /// Builds a [`Krates`] graph using the specified metadata. If `on_filter` is
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
        let mut resolved = md.resolve.ok_or(Error::NoResolveGraph)?;

        let mut packages: Vec<_> = md
            .packages
            .into_iter()
            .map(|pkg| (Kid::from(pkg.id.clone()), pkg))
            .collect();
        packages.sort_by(|(a, _), (b, _)| a.cmp(b));

        // Load all the cache entries from disk for all the possible _unique_
        // crates in the graph so that we don't need to access disk again
        let index = self.crates_io_index.map(|index| {
            index::CachingIndex::new(
                index,
                packages.iter().map(|(_id, pkg)| pkg.name.clone()).collect(),
            )
        });

        let mut workspace_members: Vec<_> =
            md.workspace_members.into_iter().map(Kid::from).collect();
        workspace_members.sort();

        let root = resolved.root.take().map(Kid::from);

        let roots = {
            let mut roots = BTreeSet::new();

            // Only include workspaces members the user wants if they have specified
            // any, this is to take into account scenarios where you have a large
            // workspace, but only want to get the crates used by a subset of the
            // workspace
            if self.workspace_filters.is_empty() {
                // If the resolve graph specifies a root, it means the user
                // specified a particular crate in a workspace, so we'll only use
                // that single root for the entire graph rather than a root for each
                // workspace member crate
                if !self.workspace {
                    if let Some(rkid) = &root {
                        roots.insert(rkid);
                    }
                }

                if roots.is_empty() {
                    roots.extend(workspace_members.iter());
                }
            } else {
                // If the filters only contain 1 path, and it is the path to a
                // workspace toml, then we disregard the filters
                if self.workspace_filters.len() == 1
                    && Some(md.workspace_root.as_ref()) == self.workspace_filters[0].parent()
                {
                    roots.extend(workspace_members.iter());
                } else {
                    for wm in &workspace_members {
                        if let Ok(i) = packages.binary_search_by(|(id, _pkg)| id.cmp(wm)) {
                            if self
                                .workspace_filters
                                .iter()
                                .any(|wf| wf == &packages[i].1.manifest_path)
                            {
                                roots.insert(wm);
                            }
                        }
                    }
                }
            }

            roots
        };

        let is_root_crate = |pid: &Kid| -> bool { roots.contains(&pid) };

        let exclude = self.exclude;

        let include_all_targets = self.target_filters.is_empty();
        let ignore_kinds = self.ignore_kinds;
        let targets = self.target_filters;

        #[derive(Debug)]
        struct DepKindInfo {
            kind: DepKind,
            cfg: Option<(String, cargo_platform::Platform)>,
        }

        #[derive(Debug)]
        // We use our resolution nodes because cargo_metadata uses
        // non-exhaustive everywhere :p
        struct NodeDep {
            /// The name of the dependency, which could be a different name than the crate itself
            name: String,
            pkg: Kid,
            dep_kinds: Vec<DepKindInfo>,
        }

        #[derive(Debug)]
        struct Node {
            id: Kid,
            deps: Vec<NodeDep>,
            features: Vec<String>,
            has_default_feature: bool,
        }

        impl Node {
            #[inline]
            fn feature_index(&self, feat: &str) -> usize {
                match self.features.binary_search_by(|f| f.as_str().cmp(feat)) {
                    Ok(i) => i,
                    Err(_i) => {
                        unreachable!(
                            "unable to locate {feat} for crate {} features({:?})",
                            self.id, self.features
                        );
                    }
                }
            }

            #[inline]
            fn feature(&self, index: usize) -> &str {
                self.features[index].as_str()
            }
        }

        let mut nodes: Vec<_> = resolved
            .nodes
            .into_iter()
            .map(|rn| {
                let id = Kid::from(rn.id);
                if let Err(i) = packages.binary_search_by(|(pid, _)| pid.cmp(&id)) {
                    unreachable!(
                        "oh no, it looks we couldn't find '{id}', maybe it is this? {:#?}",
                        &packages[i]
                    );
                }

                let mut deps: Vec<_> = rn
                    .deps
                    .into_iter()
                    .map(|dn| {
                        // This requires 1.41+
                        let dep_kinds = dn
                            .dep_kinds
                            .into_iter()
                            .map(|dk| DepKindInfo {
                                kind: dk.kind.into(),
                                cfg: dk.target.map(|t| (t.to_string(), t)),
                            })
                            .collect();

                        NodeDep {
                            name: dn.name,
                            pkg: Kid::from(dn.pkg),
                            dep_kinds,
                        }
                    })
                    .collect();

                // These _should_ always already be sorted, but again, might be
                // due to implementation details rather than guaranteed
                deps.sort_by(|a, b| a.pkg.cmp(&b.pkg));

                let mut features = rn.features;

                // Note that cargo metadata _currently_ always outputs these in
                // lexicographic order, but I don't know if that is actually
                // guaranteed at all and might just be due to the implementation
                // (eg stored in a Btree), so we just perform our own sort to
                // guarantee that this is indeed always sorted since we rely on
                // that attribute
                features.sort();
                let has_default_feature = features
                    .binary_search_by(|f| f.as_str().cmp("default"))
                    .is_ok();

                Node {
                    id,
                    deps,
                    features,
                    has_default_feature,
                }
            })
            .collect();

        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        // #[inline]
        // fn crate_name_from_pid(pid: &PackageId) -> &str {
        //     if let Some(name_end) = pid.repr.find(' ') {
        //         &pid.repr[..name_end]
        //     } else {
        //         DecomposedRepr::build(pid).name
        //     }
        // }

        let mut dep_edge_map = BTreeMap::new();
        let mut feature_edge_map = BTreeMap::new();

        // The stack of pid + features that are visited until we've reached every
        // unique leaf.
        //
        // This is rooted on the root packages + enabled features as returned by
        // cargo metadata, but we (no longer) use the rest of the output as is,
        // as we can prune edges based on cfg/build kind, or even exclude packages
        // entirely, and thus have to basically re-resolve the crates + features
        // to avoid adding nodes/edges that shouldn't exist based on the caller's
        // configuration (eg. https://github.com/EmbarkStudios/krates/issues/60)
        let mut visit_stack = Vec::with_capacity(roots.len());

        for root in &roots {
            let krate_index = nodes.binary_search_by(|n| n.id.cmp(root)).unwrap();
            let rnode = &nodes[krate_index];

            visit_stack.push((*root, None));

            for feat in 0..rnode.features.len() {
                visit_stack.push((*root, Some(feat)));
            }
        }

        #[derive(Debug)]
        struct DependencyEdge {
            kind: DepKind,
            cfg: Option<String>,
            features: Vec<usize>,
        }

        #[derive(Debug)]
        struct PackageNode<'p> {
            is_root: bool,
            deps: BTreeMap<&'p Kid, KrateDependency>,
        }

        #[derive(Debug)]
        struct KrateDependency {
            edges: Vec<DependencyEdge>,
            features: BTreeSet<usize>,
        }

        #[derive(Debug)]
        enum FeatureEdgeName {
            Feature(String),
            Rename(String),
            Krate,
        }

        #[derive(Debug)]
        struct FeatureEdge<'nodes> {
            kid: &'nodes Kid,
            name: FeatureEdgeName,
        }

        #[derive(Debug)]
        struct KrateFeatures<'p> {
            /// The inner DAG of features, note these are _all_ resolved features,
            /// but might not be actually be present in the final graph if they
            /// reference pruned krates
            graph: Vec<(String, Vec<FeatureEdge<'p>>)>,
            /// The actual set of features enabled by 1 or more parent krates
            actual: BTreeSet<usize>,
            /// Weakly referenced features
            pending_weak: BTreeMap<&'p Kid, BTreeSet<usize>>,
            filled_non_optional: bool,
        }

        let check = |map: &BTreeMap<&Kid, KrateFeatures<'_>>, pid: &Kid, feature: Option<usize>| {
            map.get(pid).map_or(false, |pn| {
                if let Some(feat) = feature {
                    pn.actual.contains(&feat)
                } else {
                    pn.filled_non_optional
                }
            })
        };

        let get_rnode = |pid: &Kid| &nodes[nodes.binary_search_by(|n| n.id.cmp(pid)).unwrap()];

        while let Some((pid, feature)) = visit_stack.pop() {
            if check(&feature_edge_map, pid, feature) {
                continue;
            }

            let is_in_workspace = workspace_members.binary_search(pid).is_ok();

            let krate_index = nodes.binary_search_by(|n| n.id.cmp(pid)).unwrap();

            let rnode = &nodes[krate_index];
            let (_, krate) = &mut packages[krate_index];

            if exclude.iter().any(|exc| exc.matches(krate)) {
                continue;
            }

            let get_dep_id = |dep_name: &str| -> Option<&Kid> {
                rnode.deps.iter().find_map(|ndep| {
                    if dep_names_match(dep_name, &ndep.name) || dep_name == ndep.pkg.name() {
                        Some(&ndep.pkg)
                    } else {
                        None
                    }
                })
            };

            let krate_features = feature_edge_map.entry(pid).or_insert_with(|| {
                if let Some(index) = &index {
                    index::fix_features(index, krate);
                }

                // Cargo puts out a flat list of the enabled features, but we need
                // to use the declared features on the crate itself to figure out
                // the actual chain of features from one crate to another
                // package features:
                // "features": {
                //   "blocking": [
                //     "simple",
                //     "reqwest?/blocking"
                //   ],
                //   "default": [
                //     "simple"
                //   ],
                //   "json": [
                //     "reqwest?/json"
                //   ],
                //   "multipart": [
                //     "reqwest?/multipart"
                //   ],
                //   "reqwest": [
                //     "dep:reqwest"
                //   ],
                //   "rgb": [
                //     "dep:rgb"
                //   ],
                //   "serde": [
                //     "dep:serde",
                //     "rgb?/serde"
                //   ],
                //   "simple": [
                //     "json"
                //   ],
                //   "ssh": [
                //     "git/ssh",
                //     "git/ssh_key_from_memory"
                //   ],
                //   "stream": [
                //     "reqwest/stream"
                //   ],
                //   "zlib": [
                //     "git/zlib-ng-compat",
                //     "reqwest?/deflate"
                //   ]
                // },
                // resolved features:
                // "features": [
                //   "blocking",
                //   "json",
                //   "reqwest",
                //   "simple",
                //   "stream"
                // ]
                let graph: Vec<_> = rnode
                    .features
                    .iter()
                    .filter_map(|feat| {
                        // This _should_ never fail in normal cases, however if we
                        // aren't provided with an index implementation it's possible for
                        // the resolved features to mention features that aren't in
                        // the actual crate manifest
                        let sub_feats: Vec<_> = krate
                            .features
                            .get(feat)?
                            .iter()
                            .filter_map(|sub_feat| {
                                let sf = ParsedFeature::from(sub_feat.as_str());

                                match sf.feat() {
                                    Feature::Krate(krate_name) => {
                                        let kid = get_dep_id(krate_name)?;
                                        let real_name = kid.name();

                                        Some(FeatureEdge {
                                            kid,
                                            name: if real_name != krate_name {
                                                FeatureEdgeName::Rename(krate_name.to_owned())
                                            } else {
                                                FeatureEdgeName::Krate
                                            },
                                        })
                                    }
                                    Feature::Simple(s) => Some(FeatureEdge {
                                        kid: pid,
                                        name: FeatureEdgeName::Feature(s.to_owned()),
                                    }),
                                    Feature::Strong {
                                        krate: krate_name,
                                        feature,
                                    } => Some(FeatureEdge {
                                        kid: get_dep_id(krate_name)?,
                                        name: FeatureEdgeName::Feature(feature.to_owned()),
                                    }),
                                    Feature::Weak {
                                        krate: krate_name,
                                        feature,
                                    } => {
                                        if rnode.features.iter().any(|kn| kn == krate_name) {
                                            Some(FeatureEdge {
                                                kid: get_dep_id(krate_name)?,
                                                name: FeatureEdgeName::Feature(feature.to_owned()),
                                            })
                                        } else {
                                            None
                                        }
                                    }
                                }
                            })
                            .collect();

                        Some((feat.clone(), sub_feats))
                    })
                    .collect();

                KrateFeatures {
                    graph,
                    actual: BTreeSet::new(),
                    pending_weak: BTreeMap::new(),
                    filled_non_optional: false,
                }
            });

            let pn = dep_edge_map.entry(pid).or_insert_with(|| PackageNode {
                is_root: is_root_crate(pid),
                deps: BTreeMap::new(),
            });

            let mut deps = BTreeMap::<&Kid, (&NodeDep, Option<BTreeSet<usize>>)>::new();

            if let Some(feature) = feature {
                if !krate_features.actual.insert(feature) {
                    unreachable!();
                }

                if !krate_features.filled_non_optional {
                    visit_stack.push((pid, None));
                }

                // This _should_ never fail in normal cases, however if the
                // an index implementation is not provided, it's possible for
                // the resolved features to mention features that aren't in
                // the actual crate manifest
                let fs = if let Some(fs) = krate.features.get(rnode.feature(feature)) {
                    fs
                } else {
                    // TODO: Show the user a warning, or just fail?
                    continue;
                };

                for sf in fs {
                    let pf = ParsedFeature::from(sf.as_str());

                    let (krate_name, feature) = match pf.feat() {
                        Feature::Simple(feat) => {
                            visit_stack.push((pid, Some(rnode.feature_index(feat))));
                            continue;
                        }
                        Feature::Krate(krate) => (krate, None),
                        Feature::Strong { krate, feature } | Feature::Weak { krate, feature } => {
                            (krate, Some(feature))
                        }
                    };

                    let Some(ndep) = rnode.deps.iter().find(|rdep| {
                        dep_names_match(krate_name, &rdep.name) || krate_name == rdep.pkg.name()
                    }) else {
                        // We can have a feature that points to a crate that isn't resolved by cargo due to it being
                        // a dev-only dependency
                        continue;
                    };

                    let rdep_node = get_rnode(&ndep.pkg);
                    let is_weak = matches!(pf.feat(), Feature::Weak { .. });

                    if is_weak && !pn.deps.contains_key(&ndep.pkg) {
                        let feature = rdep_node.feature_index(feature.unwrap());
                        krate_features
                            .pending_weak
                            .entry(&ndep.pkg)
                            .or_default()
                            .insert(feature);
                        continue;
                    } else if let Some(mut pending) = krate_features.pending_weak.remove(&ndep.pkg)
                    {
                        deps.entry(&ndep.pkg)
                            .or_insert_with(|| (ndep, Some(BTreeSet::new())))
                            .1
                            .as_mut()
                            .unwrap()
                            .append(&mut pending);
                    }

                    let feats = deps
                        .entry(&ndep.pkg)
                        .or_insert_with(|| (ndep, Some(BTreeSet::new())))
                        .1
                        .as_mut()
                        .unwrap();

                    if let Some(feature) = feature {
                        feats.insert(rdep_node.feature_index(feature));
                    }
                }
            } else {
                krate_features.filled_non_optional = true;
                deps.extend(rnode.deps.iter().map(|ndep| (&ndep.pkg, (ndep, None))));
            }

            let targets = &targets;

            // Though each unique dependency can only be resolved once, it's possible
            // for the crate to list the same dependency multiple times, with different
            // dependency kinds, or different target configurations, so each one gets its
            // own edge
            for (pkg, (rdep, mut features)) in deps {
                let rdep_node = get_rnode(pkg);

                #[derive(Debug)]
                struct Edge<'d> {
                    kind: DepKind,
                    cfg: Option<&'d str>,
                    features: &'d [String],
                    uses_default_features: bool,
                }

                let maybe_real_name = pkg.name();
                let strong = features.is_some();

                // If there are multiple versions of the same package we use the
                // version to disambiguate references to them, unfortunately,
                // though (it should be) extremely rare to do this, we always
                // check that versions match once a crate is found, as nodes can
                // not be resolved due to features, but will still be listed as
                // a dependency
                //
                // Note that cargo _should_ fail to resolve nodes if the same
                // package is referenced with two `^` (compatible) semvers, ie,
                // you can't reference both ">= 0.2.12" and "=0.2.7" of a package
                // even if they could never point to the same package, this _may_
                // mean there could be a situation where a single crate _could_
                // be referenced with 0.0.x versions, but...I'll fix that if
                // someone reports an issue
                let rdep_version: semver::Version =
                    rdep.pkg.version().parse().expect("failed to parse semver");
                let has_prelease = !rdep_version.pre.is_empty();

                let edges = rdep.dep_kinds.iter().filter_map(|dk| {
                        let mask = match dk.kind {
                            DepKind::Normal => 0x1,
                            DepKind::Dev => 0x8,
                            DepKind::Build => 0x40,
                        };

                        let mask = mask | mask << if is_in_workspace { 1 } else { 2 };
                        if mask & ignore_kinds == mask {
                            return None;
                        }

                        let multiple_candidates = krate.dependencies.iter().filter(|dep| {
                            if dk.kind != dep.kind {
                                return false;
                            }

                            // Crates can rename the dependency package themselves
                            let dname = dep.rename.as_deref().unwrap_or(&dep.name);
                            if !dep_names_match(dname, &rdep.name) && maybe_real_name != dname {
                                return false;
                            }

                            // Handle case where a dependency may not have a version requirement, which
                            // typically happens in the case of non-registry dependencies that use a pre-release
                            // semver, if the version _is_ a prelease it will never match the empty
                            // requirement
                            if !((has_prelease && dep.req.comparators.is_empty()) || dep.req.matches(&rdep_version)) {
                                return false;
                            }

                            dk.cfg.as_ref().map(|(_, p)| p) == dep.target.as_ref()
                        }).count() > 1;

                        let dep = krate
                            .dependencies
                            .iter()
                            .find(|dep| {
                                if dk.kind != dep.kind {
                                    return false;
                                }

                                // Crates can rename the dependency package themselves
                                let dname = dep.rename.as_deref().unwrap_or(&dep.name);
                                if !dep_names_match(dname, &rdep.name) && maybe_real_name != dname {
                                    return false;
                                }

                                // Handle case where a dependency may not have a version requirement, which
                                // typically happens in the case of non-registry dependencies that use a pre-release
                                // semver, if the version _is_ a prelease it will never match the empty
                                // requirement
                                if !((has_prelease && dep.req.comparators.is_empty()) || dep.req.matches(&rdep_version)) {
                                    return false;
                                }

                                if dk.cfg.as_ref().map(|(_, p)| p) != dep.target.as_ref() {
                                    return false;
                                }

                                if !multiple_candidates {
                                    return true;
                                }

                                // Finally, even if the name matches and the version matches, the source for the package might
                                // be different if there are multiple git dependencies at different revisions :(
                                // There is also an _extreme_ edge case where a package's lib target can be the same
                                // name as another package. This actually would mean that the code won't compile, but I
                                // encountered it in testing (eg. the `md-5` crate names its lib target `md5`, and you
                                // can have a dependency on the `md5` crate, they both get resolved to the same name, but
                                // then rustc can't compile `md5::compute` because there are two libs that satisfy that name)
                                let source_matches = dep.source.as_deref().map_or(true, |dsrc| {
                                    let psrc = rdep.pkg.source();
                                    if let Some((dgit, pgit)) = dsrc.strip_prefix("git+").zip(psrc.strip_prefix("git+")) {
                                        // Git sources can have the full revision spec at the end, which is not part of
                                        // source declaration
                                        let dgit = dgit.rfind('#').map_or(dgit, |end| &dgit[..end]);
                                        dgit == pgit
                                    } else {
                                        dsrc == psrc
                                    }
                                });

                                source_matches
                            })
                            .unwrap_or_else(|| panic!("cargo metadata resolved a dependency for a dependency not specified by the crate: {rdep:?}"));

                        if dep.optional && !strong {
                            return None;
                        }

                        let cfg = if let Some(cfg) = dk.cfg.as_ref().map(|(c, _)| c.as_str()) {
                            if !include_all_targets {
                                let matched = if cfg.starts_with("cfg(") {
                                    match cfg_expr::Expression::parse(cfg) {
                                        Ok(expr) => {
                                            // We only need to focus on target predicates because they are
                                            // the only type of predicate allowed by cargo at the moment

                                            // While it might be nicer to evaluate all the targets for each predicate
                                            // it would lead to weird situations where an expression could evaluate to true
                                            // (or false) with a combination of platform, that would otherwise be impossible,
                                            // eg cfg(all(windows, target_env = "musl")) could evaluate to true
                                            targets
                                                .iter()
                                                .any(|target| expr.eval(|pred| target.eval(pred)))
                                        }
                                        Err(_pe) => {
                                            // TODO: maybe log a warning if we somehow fail to parse the cfg?
                                            true
                                        }
                                    }
                                } else {
                                    // If it's not a cfg expression, it's just a fully specified target triple,
                                    // so we just do a string comparison
                                    targets.iter().any(|target| target.matches_triple(cfg))
                                };

                                if !matched {
                                    return None;
                                }
                            } else if cfg.starts_with("cfg(") {
                                // This is _basically_ a tortured way to evaluate `cfg(any())`, which is always false but
                                // is used by eg. serde -> serde_derive. If not filtering targets this would mean that
                                // serde_derive and all of its dependencies would be pulled into the graph, even if the
                                // only edge was the cfg(any()).
                                if let Ok(expr) = cfg_expr::Expression::parse(cfg) {
                                    // We can't just do an eval and always return true, as that then would cause any
                                    // not() expressions to evaluate to false
                                    if expr.predicates().count() == 0 && !expr.eval(|_| true) {
                                        return None;
                                    }
                                }
                            }

                            Some(cfg)
                        } else {
                            None
                        };

                        Some(Edge {
                            kind: dk.kind,
                            cfg,
                            features: &dep.features,
                            // Dependencies will default to saying "uses_default_features" on edges,
                            // even if the crate in question doesn't actually have a "default" feature,
                            // so check that it actually does
                            uses_default_features: dep.uses_default_features && rdep_node.has_default_feature,
                        })
                    });

                // Don't add the dependency unless we have at least one edge, otherwise
                // we could include optional dependencies that are only weakly referenced
                let mut edges = edges.peekable();
                if edges.peek().is_none() {
                    continue;
                }

                let dep = pn.deps.entry(pkg).or_insert_with(|| KrateDependency {
                    edges: Vec::new(),
                    features: BTreeSet::new(),
                });

                let mut visit_dep = Some((pkg, None));

                for edge in edges {
                    if let Some(features) = features.take() {
                        visit_stack.extend(visit_dep.take());
                        for feat in features {
                            visit_stack.push((pkg, Some(feat)));
                            dep.features.insert(feat);
                        }
                    }

                    if !dep
                        .edges
                        .iter()
                        .any(|d| d.kind == edge.kind && d.cfg.as_deref() == edge.cfg)
                    {
                        visit_stack.extend(visit_dep.take());

                        let mut features = Vec::new();
                        for feat in edge
                            .features
                            .iter()
                            .map(|f| f.as_str())
                            .chain(edge.uses_default_features.then_some("default"))
                        {
                            let feat_index = rdep_node.feature_index(feat);
                            visit_stack.push((pkg, Some(feat_index)));
                            features.push(feat_index);
                        }

                        let edge = DependencyEdge {
                            kind: edge.kind,
                            cfg: edge.cfg.map(|s| s.into()),
                            features,
                        };

                        dep.edges.push(edge);
                    }
                }
            }
        }

        // Sanity check, it's possible the user could exclude all of the
        // possible workspace root nodes leaving themselves with an empty graph,
        // which isn't much use to anyone
        if dep_edge_map.is_empty() {
            return Err(Error::NoRootKrates);
        }

        let mut graph = petgraph::Graph::<crate::Node<N>, E>::new();
        graph.reserve_nodes(dep_edge_map.len());

        let mut edge_count = 0;

        // Preserve the ordering of the krates when inserting them into the graph
        // so that we can easily binary search for the crates based on their
        // package id with just the graph and no ancillary tables
        for (id, krate) in packages {
            if let Some(pn) = dep_edge_map.get(&id) {
                let rnode = get_rnode(&id);

                // If the crate is a root then the features it has enabled are
                // accurate, however if it is not a root then we need to manually
                // build up the list of enabled features as each edge is added
                let features = if pn.is_root {
                    rnode.features.iter().cloned().collect()
                } else {
                    feature_edge_map[&id]
                        .actual
                        .iter()
                        .map(|k| rnode.feature(*k).to_owned())
                        .collect()
                };

                let krate = crate::Node::Krate {
                    id,
                    krate: N::from(krate),
                    features,
                };

                graph.add_node(krate);
                edge_count += pn.deps.len();
            } else {
                on_filter.filtered(krate);
            }
        }

        let krates_end = graph.node_count();

        // Reserve space for each edge from a crate to another crate's feature(s)
        graph.reserve_edges(edge_count);

        let get = |graph: &petgraph::Graph<crate::Node<N>, E>,
                   kid: &Kid,
                   feature: Option<&str>|
         -> Option<crate::NodeId> {
            if let Some(feat) = feature {
                graph.raw_nodes()[krates_end..]
                    .iter()
                    .enumerate()
                    .find_map(|(i, n)| {
                        if let crate::Node::Feature { krate_index, name } = &n.weight {
                            if let crate::Node::Krate { id, .. } = &graph[*krate_index] {
                                if id == kid && name == feat {
                                    return Some(crate::NodeId::new(i + krates_end));
                                }
                            }
                        }

                        None
                    })
            } else {
                graph.raw_nodes()[..krates_end]
                    .iter()
                    .enumerate()
                    .find_map(|(i, n)| {
                        if let crate::Node::Krate { id, .. } = &n.weight {
                            if id == kid {
                                return Some(crate::NodeId::new(i));
                            }
                        }

                        None
                    })
            }
        };

        // Now that we have all of the actual crate nodes, we can link all of the
        // features exposed by each crate
        let (node_count, edge_count) =
            feature_edge_map
                .iter()
                .fold((0usize, 0usize), |(nc, ec), (pid, feats)| {
                    let nc = nc + feats.actual.len() * 2;
                    let krate_index = nodes.binary_search_by(|n| n.id.cmp(pid)).unwrap();
                    let rnode = &nodes[krate_index];

                    let ec = ec
                        + feats
                            .graph
                            .iter()
                            .filter_map(|(name, sf)| {
                                feats
                                    .actual
                                    .contains(&rnode.feature_index(name))
                                    .then_some(sf.len())
                            })
                            .sum::<usize>();

                    (nc, ec)
                });

        graph.reserve_nodes(node_count);
        graph.reserve_edges(edge_count);

        // Keep edges between crates ordered as well, though we don't depend on this
        for srcind in 0..graph.node_count() {
            let srcid = crate::NodeId::new(srcind);
            let pid = if let crate::Node::Krate { id, .. } = &graph[srcid] {
                id
            } else {
                continue;
            };

            if let Some(pn) = dep_edge_map.remove(pid) {
                // Attach an edge for each crate dependency, note that there might not
                // actually be a target crate for the edge since crates can be pruned
                // due to target configuration
                for (dep_id, dep) in pn.deps {
                    let target_krate = if let Some(tk) = get(&graph, dep_id, None) {
                        tk
                    } else {
                        continue;
                    };

                    let attach = |graph: &mut petgraph::Graph<crate::Node<N>, E>,
                                  feat: String,
                                  edge: Edge| {
                        let feat_node = if let Some(feat_node) = get(graph, dep_id, Some(&feat)) {
                            feat_node
                        } else {
                            graph.add_node(crate::Node::Feature {
                                krate_index: target_krate,
                                name: feat,
                            })
                        };

                        graph.add_edge(srcid, feat_node, edge.into());
                    };

                    let rnode = get_rnode(dep_id);

                    // Add the features that were explicitly enabled by the specific
                    // normal/dev/build dependency
                    for edge in dep.edges {
                        let attach_direct_edge = edge.features.is_empty();

                        for feat in edge.features {
                            let feat = rnode.feature(feat);
                            attach(
                                &mut graph,
                                feat.to_owned(),
                                Edge::DepFeature {
                                    kind: edge.kind,
                                    cfg: edge.cfg.clone(),
                                },
                            );
                        }

                        if attach_direct_edge {
                            graph.add_edge(
                                srcid,
                                target_krate,
                                Edge::Dep {
                                    kind: edge.kind,
                                    cfg: edge.cfg,
                                }
                                .into(),
                            );
                        }
                    }

                    // Add the features that were toggled on via a parent crate feature
                    for feat in dep.features {
                        let feat = rnode.feature(feat);
                        attach(&mut graph, feat.to_owned(), Edge::Feature);
                    }
                }
            }
        }

        // Now attach edges between all of features and their parent crate
        for (pid, features) in feature_edge_map {
            let (kind, mut feature_stack) = if let Some(kind) = get(&graph, pid, None) {
                if let crate::Node::Krate { features, .. } = &graph[kind] {
                    (kind, features.iter().cloned().collect::<Vec<_>>())
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let get_or_insert = |graph: &mut petgraph::Graph<crate::Node<N>, E>,
                                 kid: &Kid,
                                 feature: &str|
             -> crate::NodeId {
                let node_id =
                    graph.raw_nodes()[krates_end..]
                        .iter()
                        .enumerate()
                        .find_map(|(i, n)| {
                            if let crate::Node::Feature { krate_index, name } = &n.weight {
                                if let crate::Node::Krate { id, .. } = &graph[*krate_index] {
                                    if id == kid && name == feature {
                                        return Some(crate::NodeId::new(i + krates_end));
                                    }
                                }
                            }

                            None
                        });

                node_id.unwrap_or_else(|| {
                    let krate_index = get(graph, kid, None).unwrap();

                    graph.add_node(crate::Node::Feature {
                        krate_index,
                        name: feature.to_owned(),
                    })
                })
            };

            let mut enabled = features.graph;

            // Since we can prune crates either by kind, target, or user specification,
            // the actual set of features might not be the same as those that
            // the crate thinks they were, so we use the top level features that
            // have been enabled by the known edges to recursively add all of
            // the sub features
            while let Some(feat) = feature_stack.pop() {
                let (_parent, sub_features) =
                    if let Some(i) = enabled.iter().position(|(k, _)| *k == feat) {
                        enabled.swap_remove(i)
                    } else {
                        continue;
                    };

                let src_id = get_or_insert(&mut graph, pid, &feat);

                // Also add an edge from each feature to the crate node it belongs to
                graph.add_edge(src_id, kind, E::from(crate::Edge::Feature));

                for sub_feat in sub_features {
                    if sub_feat.kid != pid && get(&graph, sub_feat.kid, None).is_none() {
                        continue;
                    }

                    let feat_name = match sub_feat.name {
                        FeatureEdgeName::Feature(feat) => Some(feat),
                        FeatureEdgeName::Rename(kname) => Some(kname),
                        FeatureEdgeName::Krate => None,
                    };

                    let target_id = if let Some(target_id) =
                        get(&graph, sub_feat.kid, feat_name.as_deref())
                    {
                        target_id
                    } else {
                        let feat_name = feat_name.unwrap_or_else(|| sub_feat.kid.name().to_owned());

                        let target_id = graph.add_node(crate::Node::Feature {
                            krate_index: kind,
                            name: feat_name.clone(),
                        });

                        // Ensure that all of the subfeatures enabled by the parent feature are added to the
                        // flat list of enabled features for the crate
                        if let crate::Node::Krate { features, .. } = &mut graph[kind] {
                            if !features.contains(&feat_name) {
                                features.insert(feat_name.clone());
                                feature_stack.push(feat_name);
                            }
                        }

                        target_id
                    };

                    graph.add_edge(src_id, target_id, E::from(crate::Edge::Feature));
                }
            }
        }

        Ok(Krates {
            graph,
            workspace_members,
            workspace_root: md.workspace_root,
            krates_end,
        })
    }
}

/// When renaming dependencies to something with a `-` in a Cargo.toml file,
/// the actual resolved name in the metadata will replace the `-` with a `_` so
/// we need to take that into account when comparing the names as declared in
/// the crate metadata with the dependencies in the resolved graph
#[inline]
fn dep_names_match(krate_dep_name: &str, resolved_name: &str) -> bool {
    if krate_dep_name.len() != resolved_name.len() {
        false
    } else {
        krate_dep_name
            .chars()
            .zip(resolved_name.chars())
            .all(|(kn, rn)| kn == rn || kn == '-' && rn == '_')
    }
}
