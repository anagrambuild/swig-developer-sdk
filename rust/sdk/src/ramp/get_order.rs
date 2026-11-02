use super::validation;
use crate::{
    api::{self, types::ramp_order::GetOrderRequest},
    http::HttpError,
    RampClient, RampOrder,
};
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
        let response = api::get_ramp_order::get_ramp_order(
            &self.http,
            GetOrderRequest {
                order_id: order_id.to_owned(),
            },
        )
        .await
        .map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => GetRampOrderError::RequestFailed,
            HttpError::Timeout => GetRampOrderError::RequestTimedOut,
            HttpError::InvalidResponse(message) => GetRampOrderError::InvalidResponse(message),
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

        let order = RampOrder::try_from(response.order)
            .map_err(|_| GetRampOrderError::InvalidResponse("malformed Ramp order"))?;
        validation::order(&order).map_err(GetRampOrderError::InvalidResponse)?;
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
