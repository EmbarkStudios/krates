//! Utilities for fixing features in metadata with information from an index

use std::collections::{BTreeMap, BTreeSet};

/// Callback to transform a set package names to a list of their versions and features for that version
pub type BuildIndexCache =
    Box<dyn FnOnce(BTreeSet<String>) -> BTreeMap<String, Option<IndexKrate>>>;

/// The map of features for a crate version
pub type FeaturesMap = BTreeMap<String, Vec<String>>;

/// A specific version of a crate
#[derive(Clone)]
pub struct IndexKrateVersion {
    /// The unique version
    pub version: semver::Version,
    /// The feature map for that version
    pub features: FeaturesMap,
}

/// A single crate
#[derive(Clone)]
pub struct IndexKrate {
    /// All the published versions for that crate
    pub versions: Vec<IndexKrateVersion>,
}

/// A cache of index information that can be used to fix features retrieved from metadata
pub struct CachingIndex {
    cache: BTreeMap<String, Option<IndexKrate>>,
}

impl CachingIndex {
    /// Creates a caching index with information provided by the user
    #[inline]
    pub fn new(build: BuildIndexCache, krates: BTreeSet<String>) -> Self {
        Self {
            cache: build(krates),
        }
    }

    fn index_krate_features(&self, name: &str, version: &semver::Version) -> Option<&FeaturesMap> {
        self.cache.get(name).and_then(|ik| {
            ik.as_ref().and_then(|ik| {
                ik.versions
                    .iter()
                    .find_map(|ikv| (&ikv.version == version).then_some(&ikv.features))
            })
        })
    }
}

/// Correct features with index information
///
/// Due to <https://github.com/rust-lang/cargo/issues/11319>, we can't actually
/// trust cargo to give us the correct package metadata, so we instead use the
/// (presumably) correct data from the the index
pub(super) fn fix_features(index: &CachingIndex, krate: &mut crate::Package) {
    if krate.source.as_ref().is_none_or(|src| !src.is_crates_io()) {
        return;
    }

    let Some(features) = index.index_krate_features(&krate.name, &krate.version) else {
        return;
    };

    for (ikey, ivalue) in features {
        if !krate.features.contains_key(ikey) {
            krate.features.insert(ikey.clone(), ivalue.clone());
        }
    }

    // The index entry features might not have the `dep:<crate>` used with weak
    // features if the crate version was published with cargo <1.60.0 version,
    // so we need to manually fix that up since we depend on that format
    let missing_deps: Vec<_> = krate
        .features
        .values()
        .flat_map(|sf| sf.iter())
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
}
