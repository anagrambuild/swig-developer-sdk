use thiserror::Error;

use crate::http::HttpError;

/// Conversion failures carry SDK-authored context, never input data.
#[derive(Debug, Error, Eq, PartialEq)]
pub(crate) enum ApiConversionError {
    #[error("{0}")]
    Invalid(&'static str),
}

impl From<ApiConversionError> for HttpError {
    fn from(error: ApiConversionError) -> Self {
        match error {
            ApiConversionError::Invalid(message) => Self::InvalidResponse(message),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/api/error.rs"]
mod tests;
