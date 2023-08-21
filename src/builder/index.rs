use std::collections::{BTreeMap, BTreeSet};
use tame_index::index::ComboIndexCache;

pub type FeaturesMap = BTreeMap<String, Vec<String>>;

#[derive(Clone)]
pub struct IndexKrateVersion {
    pub version: semver::Version,
    pub features: FeaturesMap,
}

#[derive(Clone)]
pub struct IndexKrate {
    pub versions: Vec<IndexKrateVersion>,
}

pub struct CachingIndex {
    cache: BTreeMap<String, Option<IndexKrate>>,
}

impl CachingIndex {
    /// Creates a caching index around the specified index
    pub fn new(inner: ComboIndexCache, krates: BTreeSet<String>) -> Self {
        let mut cache = BTreeMap::new();
        for name in krates {
            let read = || -> Option<IndexKrate> {
                let name = name.as_str().try_into().ok()?;
                let krate = inner.cached_krate(name).ok()??;
                let versions = krate
                    .versions
                    .into_iter()
                    .filter_map(|kv| {
                        // The index (currently) can have both features, and
                        // features2, the features method gives us an iterator
                        // over both
                        kv.version.parse::<semver::Version>().ok().map(|version| {
                            IndexKrateVersion {
                                version,
                                features: kv
                                    .features()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect(),
                            }
                        })
                    })
                    .collect();

                Some(IndexKrate { versions })
            };

            let krate = read();
            cache.insert(name, krate);
        }

        Self { cache }
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

/// Due to <https://github.com/rust-lang/cargo/issues/11319>, we can't actually
/// trust cargo to give us the correct package metadata, so we instead use the
/// (presumably) correct data from the the index
pub(super) fn fix_features(index: &CachingIndex, krate: &mut cargo_metadata::Package) {
    if krate
        .source
        .as_ref()
        .map_or(true, |src| !src.is_crates_io())
    {
        return;
    }

    let Some(features) = index.index_krate_features(&krate.name, &krate.version) else { return; };

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
}
