use crate::Error;
use semver::Version;

/// A package specification. See
/// [cargo pkgid](https://doc.rust-lang.org/cargo/commands/cargo-pkgid.html)
/// for more information on this.
#[derive(Debug, Clone)]
pub struct PkgSpec {
    pub name: String,
    pub version: Option<Version>,
    pub url: Option<String>,
}

impl PkgSpec {
    pub fn matches(&self, krate: &crate::cm::Package) -> bool {
        if self.name != krate.name {
            return false;
        }

        if let Some(ref vers) = self.version {
            if vers != &krate.version {
                return false;
            }
        }

        let Some((url, src)) = self
            .url
            .as_ref()
            .zip(krate.source.as_ref().map(|s| s.repr.as_str()))
        else {
            return true;
        };

        let begin = src.find('+').map_or(0, |i| i + 1);
        let end = src.find('?').or_else(|| src.find('#')).unwrap_or(src.len());

        url == &src[begin..end]
    }
}

impl std::str::FromStr for PkgSpec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name_and_or_version = |nv: Option<&str>,
                                   url: Option<&str>|
         -> Result<(String, Option<Version>), Self::Err> {
            let validate_name = |n: &str| -> Result<String, Self::Err> {
                if n.find(|c: char| c != '-' && c != '_' && !c.is_ascii_alphanumeric())
                    .is_some()
                {
                    Err(Error::InvalidPkgSpec(
                        "found an invalid character for the package name",
                    ))
                } else {
                    Ok(n.to_owned())
                }
            };

            // We validate the url portion regardless, unlike cargo
            let path_name = match url {
                Some(url) => {
                    // Cargo is actually more lenient than this and will allow an end slash,
                    // even though that means it will never actually match a package due to
                    // how it retrieves a name from the url if the name isn't explicitly provided
                    if url.ends_with('/') {
                        return Err(Error::InvalidPkgSpec("url ends with /"));
                    }

                    match url.rfind("://") {
                        Some(ind) => match url.rfind('/') {
                            Some(pind) => {
                                if pind == ind + 2 {
                                    return Err(Error::InvalidPkgSpec("path required for urls"));
                                }

                                let path = &url[pind + 1..];
                                Some(path)
                            }
                            None => return Err(Error::InvalidPkgSpec("path required for urls")),
                        },
                        None => return Err(Error::InvalidPkgSpec("missing url scheme")),
                    }
                }
                None => None,
            };

            match nv {
                Some(nv) => {
                    match nv.find(':') {
                        Some(ind) => {
                            if ind == nv.len() - 1 {
                                return Err(Error::InvalidPkgSpec(
                                    "package spec cannot end with ':'",
                                ));
                            }

                            Ok((
                                validate_name(&nv[..ind])?,
                                Some(Version::parse(&nv[ind + 1..]).map_err(|_e| {
                                    Error::InvalidPkgSpec("failed to parse version")
                                })?),
                            ))
                        }
                        None => {
                            // If we have a url, this could be either a name and/or a version
                            match path_name {
                                Some(name) => {
                                    // This is the same way that cargo itself parses
                                    if nv.chars().next().unwrap().is_alphabetic() {
                                        Ok((validate_name(nv)?, None))
                                    } else {
                                        let version = Version::parse(nv).map_err(|_e| {
                                            Error::InvalidPkgSpec("failed to parse version")
                                        })?;

                                        Ok((validate_name(name)?, Some(version)))
                                    }
                                }
                                None => Ok((validate_name(nv)?, None)),
                            }
                        }
                    }
                }
                None => Ok((validate_name(path_name.unwrap())?, None)),
            }
        };

        if s.contains('/') {
            let url = if s.contains("://") {
                std::borrow::Cow::Borrowed(s)
            } else {
                std::borrow::Cow::Owned(format!("cargo://{}", s))
            };

            if let Some(ind) = url.find('#') {
                if ind == url.len() - 1 {
                    return Err(Error::InvalidPkgSpec("package spec cannot end with '#'"));
                }

                let url_no_frag = url[..ind].to_owned();

                let (name, version) =
                    name_and_or_version(Some(&url[ind + 1..]), Some(&url_no_frag))?;

                Ok(Self {
                    url: Some(url_no_frag),
                    name,
                    version,
                })
            } else {
                let (name, version) = name_and_or_version(None, Some(&url))?;

                Ok(Self {
                    url: Some(url.into_owned()),
                    name,
                    version,
                })
            }
        } else {
            let (name, version) = name_and_or_version(Some(s), None)?;

            Ok(Self {
                url: None,
                name,
                version,
            })
        }
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
        let spec: PkgSpec = "bitflags:1.0.4".parse().unwrap();

        assert_eq!("bitflags", spec.name);
        assert_eq!(Version::parse("1.0.4").unwrap(), spec.version.unwrap());
        assert!(spec.url.is_none());
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
        let spec: PkgSpec = "https://github.com/rust-lang/cargo#crates-io:0.21.0"
            .parse()
            .unwrap();

        assert_eq!("crates-io", spec.name);
        assert_eq!(Version::parse("0.21.0").unwrap(), spec.version.unwrap());
        assert_eq!("https://github.com/rust-lang/cargo", spec.url.unwrap());
    }

    #[test]
    fn no_proto() {
        let spec: PkgSpec = "crates.io/foo".parse().unwrap();

        assert_eq!("foo", spec.name);
        assert!(spec.version.is_none());
        assert_eq!("cargo://crates.io/foo", spec.url.unwrap());
    }

    #[test]
    fn no_proto_and_version() {
        let spec: PkgSpec = "crates.io/foo#1.2.3".parse().unwrap();

        assert_eq!("foo", spec.name);
        assert_eq!(Version::parse("1.2.3").unwrap(), spec.version.unwrap());
        assert_eq!("cargo://crates.io/foo", spec.url.unwrap());
    }

    #[test]
    fn no_proto_and_name_and_version() {
        let spec: PkgSpec = "crates.io/foo#1.2.3".parse().unwrap();

        assert_eq!("foo", spec.name);
        assert_eq!(Version::parse("1.2.3").unwrap(), spec.version.unwrap());
        assert_eq!("cargo://crates.io/foo", spec.url.unwrap());
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
