use super::validation;
use crate::{
    http::HttpError,
    types::wire::ramp::{validate_order_response_shape, OrderResponse},
    RampClient, RampOrder,
};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum GetRampOrderError {
    #[error("invalid Ramp order input: {0}")]
    InvalidInput(&'static str),

    #[error("Ramp order {order_id} was not found")]
    OrderNotFound { order_id: String },

    #[error("authentication failed")]
    AuthenticationFailed,

    #[error("permission denied")]
    PermissionDenied,

    #[error("service unavailable")]
    ServiceUnavailable,

    #[error("request timed out")]
    RequestTimedOut,

    #[error("request could not be completed")]
    RequestFailed,

    #[error("invalid Ramp order response: {0}")]
    InvalidResponse(&'static str),

    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl RampClient {
    pub async fn get_order(&self, order_id: &str) -> Result<RampOrder, GetRampOrderError> {
        if order_id.trim().is_empty() {
            return Err(GetRampOrderError::InvalidInput("order ID is required"));
        }
        let path = format!(
            "/wallet/api/ramp/orders/{}",
            utf8_percent_encode(order_id, NON_ALPHANUMERIC)
        );
        let bytes = self.http.get(&path).await.map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => GetRampOrderError::RequestFailed,
            HttpError::Timeout => GetRampOrderError::RequestTimedOut,
            HttpError::ResponseTooLarge => {
                GetRampOrderError::InvalidResponse("response exceeds size limit")
            }
            HttpError::Unauthenticated => GetRampOrderError::AuthenticationFailed,
            HttpError::PermissionDenied => GetRampOrderError::PermissionDenied,
            HttpError::InvalidArgument => {
                GetRampOrderError::InvalidInput("service rejected the request")
            }
            HttpError::NotFound => GetRampOrderError::OrderNotFound {
                order_id: order_id.to_owned(),
            },
            HttpError::Unavailable => GetRampOrderError::ServiceUnavailable,
            HttpError::AlreadyExists
            | HttpError::FailedPrecondition
            | HttpError::Aborted
            | HttpError::ResourceExhausted
            | HttpError::UnexpectedStatus(_) => GetRampOrderError::UnexpectedServiceFailure,
        })?;
        let order = normalize_get_order_response(&bytes)?;
        let id = match &order {
            RampOrder::Buy(order) => &order.id,
            RampOrder::Sell(order) => &order.id,
        };
        if id != order_id {
            return Err(GetRampOrderError::InvalidResponse(
                "Ramp order identity mismatch",
            ));
        }
        Ok(order)
    }
}
fn normalize_get_order_response(bytes: &[u8]) -> Result<RampOrder, GetRampOrderError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| GetRampOrderError::InvalidResponse("malformed Ramp order"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    validate_order_response_shape(&payload)
        .map_err(|_| GetRampOrderError::InvalidResponse("malformed Ramp order"))?;
    let response: OrderResponse = serde_json::from_value(payload)
        .map_err(|_| GetRampOrderError::InvalidResponse("malformed Ramp order"))?;
    validation::order(&response.order).map_err(GetRampOrderError::InvalidResponse)?;
    Ok(response.order)
}
