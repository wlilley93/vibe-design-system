//! The one error type, and the exit-code contract.
//!
//! Exit codes are a contract a caller reads; the text above them is what a human
//! reads. Both matter, so neither is left to chance: every refusal carries a
//! sentence saying what was expected and, where it can, what to run instead.

use thiserror::Error;

/// Passed. The check ran and found nothing.
pub const EXIT_PASSED: i32 = 0;
/// A violation. The check ran and found something.
pub const EXIT_VIOLATION: i32 = 1;
/// A precondition failed. The check did NOT run and proves nothing.
pub const EXIT_PRECONDITION: i32 = 2;
/// Vacuous. The check ran over zero enforceable rows (VDS S-7(2)(4)).
pub const EXIT_VACUOUS: i32 = 3;

/// Every failure VDS can produce.
///
/// There is deliberately no `Warning` variant. VDS S-7(2) makes a check's failing
/// direction load-bearing, and a warning is a failure that was allowed to pass.
/// Where a finding genuinely should not block, it is a `Violation` carrying
/// [`crate::Severity::Warning`] on a proof record, which is counted and printed,
/// not an error that was swallowed.
#[derive(Debug, Error)]
pub enum VdsError {
    #[error("{0}")]
    Precondition(String),

    #[error("{path}: {message}")]
    Artefact { path: String, message: String },

    #[error("{path} does not validate as a {kind}:\n  {}", .errors.join("\n  "))]
    Validation {
        path: String,
        kind: &'static str,
        errors: Vec<String>,
    },

    #[error(
        "no .vds/config.toml found at or above {0}.\n  \
         Run: vds init --root <project>"
    )]
    NoProject(String),

    #[error("{0}")]
    Identifier(String),

    #[error(
        "{path}: {kind} schema_version {found} exceeds what this build understands \
         ({understood}). Refusing rather than skipping what it cannot parse (VDS S-11(2))."
    )]
    SchemaVersionAhead {
        path: String,
        kind: &'static str,
        found: u32,
        understood: u32,
    },

    #[error("io error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("could not parse {path} as {format}: {message}")]
    Parse {
        path: String,
        format: &'static str,
        message: String,
    },

    #[error("could not serialise {what}: {message}")]
    Serialize { what: String, message: String },
}

impl VdsError {
    pub fn precondition(message: impl Into<String>) -> Self {
        Self::Precondition(message.into())
    }

    pub fn io(path: impl std::fmt::Display, source: std::io::Error) -> Self {
        Self::Io {
            path: path.to_string(),
            source,
        }
    }

    pub fn parse(
        path: impl std::fmt::Display,
        format: &'static str,
        message: impl std::fmt::Display,
    ) -> Self {
        Self::Parse {
            path: path.to_string(),
            format,
            message: message.to_string(),
        }
    }

    /// Every `VdsError` is a precondition failure from a caller's point of view:
    /// the command did not do the thing, and nothing partial was left behind.
    pub fn exit_code(&self) -> i32 {
        EXIT_PRECONDITION
    }
}

pub type Result<T> = std::result::Result<T, VdsError>;
