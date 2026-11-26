use crate::Error;
use semver::Version;

/// A package specification.
///
/// See [cargo pkgid](https://doc.rust-lang.org/cargo/commands/cargo-pkgid.html) for more information on this.
#[derive(Debug, Clone)]
pub struct PkgSpec {
    /// The name of the crate
    pub name: String,
    /// The semver of the crate
    pub version: Option<Version>,
    /// The source of the crate
    pub url: Option<String>,
}

pub struct PkgSpecComponents<'s> {
    pub name: &'s str,
    pub version: Option<&'s str>,
    pub url: Option<&'s str>,
}

impl PkgSpec {
    /// Returns true if this package spec matches the specified crate
    pub fn matches(&self, krate: &crate::cm::Package) -> bool {
        if self.name != krate.name {
            return false;
        }

        if let Some(vers) = &self.version {
            if vers != &krate.version {
                return false;
            }
        }

        match (
            self.url.as_deref(),
            krate.source.as_ref().map(|s| s.repr.as_str()),
        ) {
            (Some(url), Some(src)) => {
                let begin = src.find('+').map_or(0, |i| i + 1);
                let end = src
                    .rfind('?')
                    .or_else(|| src.rfind('#'))
                    .unwrap_or(src.len());

                url == &src[begin..end]
            }
            (None, None) => true,
            (Some(url), None) => {
                // It should never be the case that a package has a null source and is not a path
                if !krate.id.repr.contains("path+file://") {
                    // technically unreachable barring a bug in the metadata generation
                    return false;
                };

                let pkid = crate::ParsedKid::parse(&krate.id.repr);

                // We _could_ recheck the name and version but them differing would indicate a serious bug in the metadata
                // generation that would realistically never? happen
                let src = match pkid {
                    crate::ParsedKid::Opaque(comps) => comps[2],
                    crate::ParsedKid::Stable { src, .. } => (0, src),
                };

                url == &krate.id.repr[src.0..src.1]
            }
            // The crates.io is used as the source and pkgid source even when using source replacement, the only way
            // to actually tell if a crate is sourced from a non-crates.io registry is to look at the source paths or
            // parse the source replacement from the cargo configuration/environment
            (None, Some(url)) => url == "registry+https://github.com/rust-lang/crates.io-index",
        }
    }

    /// Attempts to parse a string as the components of a [`Self`]
    pub fn parse_components(s: &str) -> Result<PkgSpecComponents<'_>, Error> {
        fn name_and_or_version<'s>(
            nv: Option<&'s str>,
            url: Option<&'s str>,
        ) -> Result<(&'s str, Option<&'s str>), Error> {
            fn validate_name(n: &str) -> Result<&str, Error> {
                if n.find(|c: char| c != '-' && c != '_' && !c.is_ascii_alphanumeric())
                    .is_some()
                {
                    Err(Error::InvalidPkgSpec(
                        "found an invalid character for the package name",
                    ))
                } else {
                    Ok(n)
                }
            }

            // We validate the url portion regardless, unlike cargo
            let path_name = match url {
                Some(url) => {
                    // Cargo is actually more lenient than this and will allow an end slash,
                    // even though that means it will never actually match a package due to
                    // how it retrieves a name from the url if the name isn't explicitly provided
                    if url.ends_with('/') {
                        return Err(Error::InvalidPkgSpec("url ends with /"));
                    }

                    let Some(ind) = url.find("://") else {
                        return Err(Error::InvalidPkgSpec("missing url scheme"));
                    };

                    let Some(pind) = url.rfind('/') else {
                        return Err(Error::InvalidPkgSpec("path required for urls"));
                    };

                    if pind == ind + 2 {
                        return Err(Error::InvalidPkgSpec("path required for urls"));
                    }

                    let path = &url[pind + 1..];
                    Some(path)
                }
                None => None,
            };

            let Some(nv) = nv else {
                return Ok((validate_name(path_name.unwrap())?, None));
            };

            if let Some(ind) = nv.find([':', '@']) {
                if ind == nv.len() - 1 {
                    return Err(Error::InvalidPkgSpec(
                        "package spec cannot end with ':' or '@'",
                    ));
                }

                Ok((validate_name(&nv[..ind])?, Some(&nv[ind + 1..])))
            } else {
                // If we have a url, this could be either a name and/or a version
                let Some(name) = path_name else {
                    return Ok((validate_name(nv)?, None));
                };

                let Some(first) = nv.chars().next() else {
                    return Err(Error::InvalidPkgSpec("fragment was empty"));
                };

                let (name, version) = if first.is_alphabetic() {
                    (validate_name(nv)?, None)
                } else {
                    (validate_name(name)?, Some(nv))
                };

                Ok((name, version))
            }
        }

        let (url, name, version) = if s.contains('/') {
            let url = s;

            if let Some((url, frag)) = url.rsplit_once('#') {
                if frag.is_empty() {
                    return Err(Error::InvalidPkgSpec("package spec cannot end with '#'"));
                }

                let (name, version) = name_and_or_version(Some(frag), Some(url))?;
                (Some(url), name, version)
            } else {
                let (name, version) = name_and_or_version(None, Some(url))?;
                (Some(url), name, version)
            }
        } else {
            let (name, version) = name_and_or_version(Some(s), None)?;
            (None, name, version)
        };

        Ok(PkgSpecComponents { url, name, version })
    }
}

impl std::str::FromStr for PkgSpec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_components(s).and_then(|comps| {
            let version = if let Some(vs) = comps.version {
                Some(
                    Version::parse(vs)
                        .map_err(|_e| Error::InvalidPkgSpec("failed to parse version"))?,
                )
            } else {
                None
            };

            Ok(Self {
                name: comps.name.to_owned(),
                version,
                url: comps.url.map(|s| s.to_owned()),
            })
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn name() {
        let spec: PkgSpec = "bitflags".parse().unwrap();

        assert_eq!("bitflags", spec.name);
        assert!(spec.version.is_none());
        assert!(spec.url.is_none());
    }

    #[test]
    fn name_and_version() {
        for spec in ["bitflags:1.0.4", "bitflags@1.0.4"] {
            let spec: PkgSpec = spec.parse().unwrap();

            assert_eq!("bitflags", spec.name);
            assert_eq!(Version::parse("1.0.4").unwrap(), spec.version.unwrap());
            assert!(spec.url.is_none());
        }
    }

    #[test]
    fn url() {
        let spec: PkgSpec = "https://github.com/rust-lang/cargo".parse().unwrap();

        assert_eq!("cargo", spec.name);
        assert!(spec.version.is_none());
        assert_eq!("https://github.com/rust-lang/cargo", spec.url.unwrap());
    }

    #[test]
    fn url_and_version() {
        let spec: PkgSpec = "https://github.com/rust-lang/cargo#0.33.0".parse().unwrap();

        assert_eq!("cargo", spec.name);
        assert_eq!(Version::parse("0.33.0").unwrap(), spec.version.unwrap());
        assert_eq!("https://github.com/rust-lang/cargo", spec.url.unwrap());
    }

    #[test]
    fn url_and_name() {
        let spec: PkgSpec = "https://github.com/rust-lang/crates.io-index#bitflags"
            .parse()
            .unwrap();

        assert_eq!("bitflags", spec.name);
        assert!(spec.version.is_none());
        assert_eq!(
            "https://github.com/rust-lang/crates.io-index",
            spec.url.unwrap()
        );
    }

    #[test]
    fn url_and_name_and_version() {
        for spec in [
            "https://github.com/rust-lang/cargo#crates-io:0.21.0",
            "https://github.com/rust-lang/cargo#crates-io@0.21.0",
        ] {
            let spec: PkgSpec = spec.parse().unwrap();

            assert_eq!("crates-io", spec.name);
            assert_eq!(Version::parse("0.21.0").unwrap(), spec.version.unwrap());
            assert_eq!("https://github.com/rust-lang/cargo", spec.url.unwrap());
        }
    }

    #[test]
    fn disallow_no_path() {
        let nopes = [
            "https://crates.io#1.2.3",
            "https://crates.io",
            "https://crates.io#foo",
        ];

        for nope in &nopes {
            match nope.parse::<PkgSpec>().unwrap_err() {
                Error::InvalidPkgSpec(err) => assert_eq!(err, "path required for urls"),
                nope => panic!("didn't expect {:?}", nope),
            }
        }

        for nope in &["https://crates.io/", "crates.io/#1.2.3"] {
            match nope.parse::<PkgSpec>().unwrap_err() {
                Error::InvalidPkgSpec(err) => assert_eq!(err, "url ends with /"),
                nope => panic!("didn't expect {:?}", nope),
            }
        }

        for nope in &["crates.io#foo", "crates.io#1.2.3"] {
            match nope.parse::<PkgSpec>().unwrap_err() {
                Error::InvalidPkgSpec(err) => {
                    assert_eq!(err, "found an invalid character for the package name");
                }
                nope => panic!("didn't expect {:?}", nope),
            }
        }
    }
}
