use std::{fmt, io, str::Utf8Error, string::FromUtf8Error};

/// Error returned when executing/parsing `cargo metadata` fails.
#[derive(Debug)]
pub enum Error {
    /// Error during execution of `cargo metadata`
    CargoMetadata {
        /// stderr returned by the `cargo metadata` command
        stderr: String,
    },

    /// IO Error during execution of `cargo metadata`
    Io(io::Error),

    /// Output of `cargo metadata` was not valid utf8
    Utf8(Utf8Error),

    /// Error output of `cargo metadata` was not valid utf8
    ErrUtf8(FromUtf8Error),

    /// Deserialization error (structure of json did not match expected structure)
    Json(serde_json::Error),

    /// The output did not contain any json
    NoJson,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(io) => Some(io),
            Self::Utf8(err) => Some(err),
            Self::ErrUtf8(err) => Some(err),
            Self::Json(err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CargoMetadata { stderr } => {
                write!(f, "`cargo metadata` exited with an error: {stderr}")
            }
            Self::Io(io) => {
                write!(f, "failed to start `cargo metadata`: {io}")
            }
            Self::Utf8(err) => {
                write!(f, "cannot convert the stdout of `cargo metadata`: {err}")
            }
            Self::ErrUtf8(err) => {
                write!(f, "cannot convert the stderr of `cargo metadata`: {err}")
            }
            Self::Json(err) => {
                write!(f, "failed to interpret `cargo metadata`'s json: {err}")
            }
            Self::NoJson => {
                f.write_str("could not find any json in the output of `cargo metadata`")
            }
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<Utf8Error> for Error {
    fn from(value: Utf8Error) -> Self {
        Self::Utf8(value)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(value: FromUtf8Error) -> Self {
        Self::ErrUtf8(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
