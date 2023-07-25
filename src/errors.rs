use cargo_metadata::Error as CMErr;
use std::fmt;

/// Errors that can occur when acquiring metadata to create a graph from
#[derive(Debug)]
pub enum Error {
    /// --no-deps was specified when acquiring metadata
    NoResolveGraph,
    /// A cargo_metadata error occurred
    Metadata(CMErr),
    /// A package specification was invalid
    InvalidPkgSpec(&'static str),
    /// Due to how the graph was built, all possible root nodes were actually
    /// filtered out, leaving an empty graph
    NoRootKrates,
    /// An error occurred trying to open or read the crates.io index
    #[cfg(feature = "prefer-index")]
    Index(tame_index::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoResolveGraph => f.write_str("no resolution graph was provided"),
            Self::Metadata(err) => write!(f, "{err}"),
            Self::InvalidPkgSpec(err) => write!(f, "package spec was invalid: {err}"),
            Self::NoRootKrates => f.write_str("no root crates available"),
            #[cfg(feature = "prefer-index")]
            Self::Index(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Metadata(err) => Some(err),
            #[cfg(feature = "with-crates-index")]
            Self::CratesIndex(err) => Some(err),
            _ => None,
        }
    }
}

impl From<CMErr> for Error {
    fn from(e: CMErr) -> Self {
        Error::Metadata(e)
    }
}

#[cfg(feature = "prefer-index")]
impl From<tame_index::Error> for Error {
    fn from(e: tame_index::Error) -> Self {
        Error::Index(e)
    }
}
