use thiserror::Error;

/// Internal conversion failures carry SDK-authored context, never input data.
#[derive(Debug, Error, Eq, PartialEq)]
pub(crate) enum WireError {
    #[error("{0}")]
    Invalid(&'static str),
}
