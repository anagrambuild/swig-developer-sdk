use std::error::Error;

use crate::{api::error::ApiConversionError, http::HttpError};

#[test]
fn conversion_errors_become_safe_http_invalid_responses() {
    let error = ApiConversionError::Invalid("invalid policy key source");
    assert_eq!(error.to_string(), "invalid policy key source");
    assert!(error.source().is_none());

    let error = HttpError::from(error);
    assert!(matches!(
        error,
        HttpError::InvalidResponse("invalid policy key source")
    ));
    assert_eq!(
        error.to_string(),
        "invalid response: invalid policy key source"
    );
    assert!(error.source().is_none());
}
