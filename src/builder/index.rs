use std::collections::{BTreeMap, BTreeSet};

pub type FeaturesMap = BTreeMap<String, Vec<String>>;

#[derive(Copy, Clone)]
pub enum IndexKind {
    Sparse,
    Git,
}

#[derive(Clone)]
pub struct IndexKrateVersion {
    pub version: semver::Version,
    pub features: FeaturesMap,
}

#[derive(Clone)]
pub struct IndexKrate {
    pub versions: Vec<IndexKrateVersion>,
}

pub trait CratesIoIndex {
    fn index_krate(&self, _name: &str) -> Option<IndexKrate> {
        None
    }

    fn index_krate_features(
        &self,
        _name: &str,
        _version: &semver::Version,
        _on_features: &mut dyn FnMut(Option<&FeaturesMap>),
    ) {
    }

    /// Allows an implementation to read local cache entries for crates to avoid
    /// individual lookups
    fn prepare_cache_entries(&self, _krate_set: BTreeSet<String>) {}
}

pub struct CachingIndex<TIndex: CratesIoIndex> {
    cache: std::sync::RwLock<BTreeMap<String, Option<IndexKrate>>>,
    inner: TIndex,
}

impl<TIndex: CratesIoIndex> CachingIndex<TIndex> {
    /// Creates a caching index around the specified index
    pub fn new(inner: TIndex) -> Self {
        Self {
            cache: std::sync::RwLock::new(BTreeMap::new()),
            inner,
        }
    }

    /// Clears the in-memory cache
    #[inline]
    pub fn clear(&self) {
        self.cache.write().unwrap().clear();
    }
}

impl<TIndex: CratesIoIndex> CratesIoIndex for CachingIndex<TIndex> {
    fn index_krate_features(
        &self,
        name: &str,
        version: &semver::Version,
        on_features: &mut dyn FnMut(Option<&FeaturesMap>),
    ) {
        loop {
            if let Some(index_krate) = self.cache.read().unwrap().get(name) {
                let fm = index_krate.as_ref().and_then(|ik| {
                    ik.versions
                        .iter()
                        .find_map(|ikv| (&ikv.version == version).then_some(&ikv.features))
                });
                on_features(fm);
                return;
            } else {
                self.cache
                    .write()
                    .unwrap()
                    .insert(name.to_owned(), self.inner.index_krate(name));
            }
        }
    }

    fn prepare_cache_entries(&self, krate_set: BTreeSet<String>) {
        let mut cache = self.cache.write().unwrap();

        for krate_name in krate_set {
            let krate = self.inner.index_krate(&krate_name);
            cache.insert(krate_name, krate);
        }
    }
}

#[cfg(feature = "with-index-impl")]
mod external {
    use super::*;
    use tame_index as ti;

    pub fn sparse(cargo_home: Option<&Path>) -> Result<CachingIndex<Index>, crate::Error> {
        let sparse = if let Some(cargo_home) = cargo_home {
            ci::SparseIndex::with_path(cargo_home, ci::CRATES_IO_HTTP_INDEX)?
        } else {
            ci::SparseIndex::new_cargo_default()?
        };

        Ok(CachingIndex::new(Index::Sparse(sparse)))
    }

    /// Converts from a [`crates_index::Crate`] to an [`IndexKrate`].
    ///
    /// All versions _should_ be parsed since crates.io verifies semver on upload,
    /// but just in case, we just skip versions that we can't parse as we still
    /// have the local information already
    fn convert(krate: ci::Crate) -> IndexKrate {
        let versions = krate
            .versions()
            .iter()
            .filter_map(|kv| {
                let version = kv.version().parse().ok()?;
                let features = kv
                    .features()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                Some(IndexKrateVersion { version, features })
            })
            .collect();

        IndexKrate { versions }
    }

    pub fn git(cargo_home: Option<&Path>) -> Result<CachingIndex<Index>, crate::Error> {
        // We would like to use the get_index_details here, but that is broken
        // currently https://github.com/frewsxcv/rust-crates-index
        //
        // Luckily we can just cheat since the we only use crates.io here and
        // can just manually specify the path
        //let (path, url) = ci::get_index_details(ci::INDEX_GIT_URL, cargo_home)?;
        let git = if let Some(cargo_home) = cargo_home {
            ci::Index::with_path(
                cargo_home.join("registry/index/github.com-1ecc6299db9ec823"),
                ci::INDEX_GIT_URL,
            )
        } else {
            ci::Index::new_cargo_default()
        };

        Ok(CachingIndex::new(Index::Git(git?)))
    }

    pub enum Index {
        Git(ci::Index),
        Sparse(ci::SparseIndex),
    }

    impl CratesIoIndex for Index {
        fn index_krate(&self, name: &str) -> Option<IndexKrate> {
            let krate = match self {
                Self::Sparse(sparse) => sparse.crate_from_cache(name).ok()?,
                Self::Git(git) => git.crate_(name)?,
            };

            Some(convert(krate))
        }
    }
}

#[cfg(feature = "with-index-impl")]
pub use external::*;

/// Due to <https://github.com/rust-lang/cargo/issues/11319>, we can't actually
/// trust cargo to give us the correct package metadata, so we instead use the
/// (presumably) correct data from the the index
pub(super) fn fix_features(index: &dyn CratesIoIndex, krate: &mut cargo_metadata::Package) {
    if krate
        .source
        .as_ref()
        .map_or(true, |src| !src.is_crates_io())
    {
        return;
    }

    index.index_krate_features(&krate.name, &krate.version, &mut |features| {
        let Some(features) = features else { return; };

        for (ikey, ivalue) in features {
            if !krate.features.contains_key(ikey) {
                krate.features.insert(ikey.clone(), ivalue.clone());
            }
        }

        // The index entry features might not have the `dep:<crate>`
        // used with weak features if the crate version was
        // published with cargo <1.60.0 version, so we need to
        // manually fix that up since we depend on that format
        let missing_deps: Vec<_> = krate
            .features
            .iter()
            .flat_map(|(_, sf)| sf.iter())
            .filter_map(|sf| {
                let pf = crate::ParsedFeature::from(sf.as_str());

                if let super::features::Feature::Simple(simple) = pf.feat() {
                    if krate.features.contains_key(simple) {
                        None
                    } else {
                        Some(simple.to_owned())
                    }
                } else {
                    None
                }
            })
            .collect();

        for missing in missing_deps {
            let dep_feature = format!("dep:{missing}");
            krate.features.insert(missing, vec![dep_feature]);
        }
    });
}
