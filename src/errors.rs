use cargo_metadata::Error as CMErr;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    NoResolveGraph,
    Metadata(CMErr),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoResolveGraph => f.write_str("no resolution graph was provided"),
            Self::Metadata(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoResolveGraph => None,
            Self::Metadata(err) => Some(err),
        }
    }
}

impl From<CMErr> for Error {
    fn from(e: CMErr) -> Self {
        Error::Metadata(e)
    }
}
