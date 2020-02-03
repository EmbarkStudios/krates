use cargo_metadata::Error as CMErr;
use std::fmt;

/// Errors that can occur when acquiring metadata to create a graph from
#[derive(Debug)]
pub enum Error {
    /// --no-deps was specified when acquiring metadata
    NoResolveGraph,
    Metadata(CMErr),
    InvalidPkgSpec(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoResolveGraph => f.write_str("no resolution graph was provided"),
            Self::Metadata(err) => write!(f, "{}", err),
            Self::InvalidPkgSpec(err) => write!(f, "package spec was invalid: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoResolveGraph | Self::InvalidPkgSpec(_) => None,
            Self::Metadata(err) => Some(err),
        }
    }
}

impl From<CMErr> for Error {
    fn from(e: CMErr) -> Self {
        Error::Metadata(e)
    }
}
