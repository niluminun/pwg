use std::fmt;

/// Single error type for the whole app. One-line messages to stderr, no hints.
#[derive(Debug, PartialEq)]
pub enum AppError {
    InvalidArg(String),
    AmbiguousAction,
    NoProfile { site: String },
    AmbiguousAccount { site: String },
    DuplicateProfile { site: String, account: String },
    CannotOpen { path: String, reason: String },
    CannotWrite { path: String, reason: String },
    BadProfile { path: String, reason: String },
    KeyRead(String),
    KeyMismatch,
    Kdf(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::InvalidArg(msg) => write!(f, "error: {msg}"),
            AppError::AmbiguousAction => write!(f, "error: ambiguous, which action?"),
            AppError::NoProfile { site } => write!(f, "error: no profile for '{site}'"),
            AppError::AmbiguousAccount { site } => {
                write!(f, "error: multiple accounts for '{site}'")
            }
            AppError::DuplicateProfile { site, account } => {
                write!(f, "error: profile for '{site}' ({account}) already exists")
            }
            AppError::CannotOpen { path, reason } => {
                write!(f, "error: cannot open '{path}': {reason}")
            }
            AppError::CannotWrite { path, reason } => {
                write!(f, "error: cannot write '{path}': {reason}")
            }
            AppError::BadProfile { path, reason } => {
                write!(f, "error: invalid profile '{path}': {reason}")
            }
            AppError::KeyRead(reason) => write!(f, "error: cannot read key: {reason}"),
            AppError::KeyMismatch => write!(f, "error: keys do not match"),
            AppError::Kdf(reason) => write!(f, "error: key derivation failed: {reason}"),
        }
    }
}

impl std::error::Error for AppError {}
