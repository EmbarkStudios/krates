pub(super) struct ComboIndex {
    git: Option<crates_index::Index>,
    http: Option<crates_index::SparseIndex>,
}

impl ComboIndex {
    #[inline]
    pub(super) fn open(allow_git: bool) -> Self {
        Self {
            git: if allow_git {
                crates_index::Index::new_cargo_default().ok()
            } else {
                None
            },
            http: crates_index::SparseIndex::from_url("sparse+https://index.crates.io/").ok(),
        }
    }

    #[inline]
    fn krate(&self, name: &str) -> Option<crates_index::Crate> {
        // Attempt http first, as this will be the default in future cargo versions
        // and using it when it is not the defaul indicates the user has opted in
        self.http
            .as_ref()
            .and_then(|h| h.crate_from_cache(name).ok())
            .or_else(|| self.git.as_ref().and_then(|g| g.crate_(name)))
    }
}

/// Due to <https://github.com/rust-lang/cargo/issues/11319>, we can't actually
/// trust cargo to give us the correct package metadata, so we instead use the
/// (presumably) correct data from the the index
pub(super) fn fix_features(index: &ComboIndex, krate: &mut cargo_metadata::Package) {
    if krate
        .source
        .as_ref()
        .map_or(true, |src| !src.is_crates_io())
    {
        return;
    }

    if let Some(entry) = index.krate(&krate.name) {
        let features = entry.versions().iter().find_map(|v| {
            if let Ok(iv) = v.version().parse::<semver::Version>() {
                if iv == krate.version {
                    Some(v.features())
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(features) = features {
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
    }
}
