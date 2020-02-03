use crate::Error;
use semver::Version;

/// A package specification. See
/// [cargo pkgid](https://doc.rust-lang.org/cargo/commands/cargo-pkgid.html)
/// for more information on this.
pub struct PkgSpec {
    pub name: String,
    pub version: Option<Version>,
    pub url: Option<String>,
}

impl std::str::FromStr for PkgSpec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name_and_or_version =
            |nv: Option<&str>, url: Option<&str>| -> Result<(String, Option<Version>), Self::Err> {
                let validate_name = |n: &str| -> Result<String, Self::Err> {
                    if let Some(_) =
                        n.find(|c: char| c != '-' && c != '_' && !c.is_ascii_alphanumeric())
                    {
                        Err(Error::InvalidPkgSpec(
                            "found an invalid character for the package name",
                        ))
                    } else {
                        Ok(n.to_owned())
                    }
                };

                let extract_name = |url: &str| -> Result<String, Self::Err> {
                    // Cargo is actually more lenient than this and will allow an end slash,
                    // even though that means it will never actually match a package due to
                    // how it retrieves a name from the url if the name isn't explicitly provided
                    if url.ends_with('/') {
                        return Err(Error::InvalidPkgSpec(
                            "url ends with /, which won't ever work",
                        ));
                    }

                    match url.rfind('/') {
                        Some(ind) => {
                            let path = &url[ind + 1..];

                            validate_name(path)
                        }
                        None => Err(Error::InvalidPkgSpec("path required for urls")),
                    }
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
                                    Some(Version::parse(&nv[ind + 1..]).map_err(|_| {
                                        Error::InvalidPkgSpec("failed to parse version")
                                    })?),
                                ))
                            }
                            None => {
                                // If we have a url, this could be either a name and/or a version
                                match url {
                                    Some(url) => {
                                        // This is the same way that cargo itself parses
                                        if nv.chars().next().unwrap().is_alphabetic() {
                                            Ok((validate_name(nv)?, None))
                                        } else {
                                            let version = Version::parse(&nv).map_err(|_| {
                                                Error::InvalidPkgSpec("failed to parse version")
                                            })?;

                                            Ok((extract_name(url)?, Some(version)))
                                        }
                                    }
                                    None => Ok((validate_name(nv)?, None)),
                                }
                            }
                        }
                    }
                    None => Ok((extract_name(url.unwrap())?, None)),
                }
            };

        if s.contains('/') {
            match s.find('#') {
                Some(ind) => {
                    if ind == s.len() - 1 {
                        return Err(Error::InvalidPkgSpec("package spec cannot end with '#'"));
                    }

                    let url = (&s[..ind]).to_owned();

                    let (name, version) = name_and_or_version(Some(&s[ind + 1..]), Some(&url))?;

                    Ok(Self {
                        url: Some(url),
                        name,
                        version,
                    })
                }
                None => {
                    let url = s.to_owned();

                    let (name, version) = name_and_or_version(None, Some(&url))?;

                    Ok(Self {
                        url: Some(url),
                        name,
                        version,
                    })
                }
            }
        } else {
            let (name, version) = name_and_or_version(Some(&s[..]), None)?;

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
}
