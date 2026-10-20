use super::validation;
use crate::{
    http::HttpError,
    types::wire::ramp::{
        validate_order_response_shape, CreateOrderRequest, OrderContextRequest, OrderResponse,
    },
    RampClient, RampEnvironment, RampLocation, RampOrder, RampOrderRequest, RampRoute,
};
use serde_json::Value;
use solana_pubkey::Pubkey;
use thiserror::Error;
#[derive(Clone)]
pub struct RampOrderContext {
    pub customer_id: String,
    pub swig_config_address: Pubkey,
    pub location: RampLocation,
}

pub struct CreateRampOrderArgs {
    pub request_id: String,
    pub context: RampOrderContext,
    pub route: RampRoute,
    pub order: RampOrderRequest,
}

#[derive(Debug, Error)]
pub enum CreateRampOrderError {
    #[error("Ramp configuration is required for this operation")]
    MissingConfiguration,
    #[error("invalid Ramp order input: {0}")]
    InvalidInput(&'static str),

    #[error("request ID {request_id} is already bound to another order payload")]
    IdempotencyConflict { request_id: String },

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
    pub async fn create_order(
        &self,
        args: CreateRampOrderArgs,
    ) -> Result<RampOrder, CreateRampOrderError> {
        let config = self
            .ramp_config
            .as_ref()
            .ok_or(CreateRampOrderError::MissingConfiguration)?;
        if args.request_id.trim().is_empty()
            || args.context.customer_id.trim().is_empty()
            || args.route.provider.trim().is_empty()
            || args.route.payment_method.trim().is_empty()
        {
            return Err(CreateRampOrderError::InvalidInput(
                "request ID, customer, and route are required",
            ));
        }
        validation::location(&args.context.location).map_err(CreateRampOrderError::InvalidInput)?;
        validation::order_request(&args.order).map_err(CreateRampOrderError::InvalidInput)?;
        let expected_sell = matches!(&args.order, RampOrderRequest::Sell { .. });
        let request = CreateOrderRequest {
            request_id: args.request_id.clone(),
            configuration_id: config.configuration_id.clone(),
            environment: match config.environment {
                RampEnvironment::Sandbox => "RAMP_ENVIRONMENT_SANDBOX",
                RampEnvironment::Production => "RAMP_ENVIRONMENT_PRODUCTION",
            },
            context: OrderContextRequest {
                customer_id: args.context.customer_id,
                swig_config_address: args.context.swig_config_address.to_string(),
                network: self.network.into(),
                location: args.context.location,
            },
            route: args.route,
            order: args.order,
        };
        let body = serde_json::to_vec(&request).map_err(|_| {
            CreateRampOrderError::InvalidInput("could not encode Ramp order request")
        })?;
        let bytes = self
            .http
            .post_with_retry("/wallet/api/ramp/orders", body)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    CreateRampOrderError::RequestFailed
                }
                HttpError::Timeout => CreateRampOrderError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    CreateRampOrderError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => CreateRampOrderError::AuthenticationFailed,
                HttpError::PermissionDenied => CreateRampOrderError::PermissionDenied,
                HttpError::InvalidArgument => {
                    CreateRampOrderError::InvalidInput("service rejected the request")
                }
                HttpError::AlreadyExists => CreateRampOrderError::IdempotencyConflict {
                    request_id: args.request_id.clone(),
                },
                HttpError::Unavailable => CreateRampOrderError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => CreateRampOrderError::UnexpectedServiceFailure,
            })?;
        let order = normalize_create_order_response(&bytes)?;
        if expected_sell != matches!(&order, RampOrder::Sell(_)) {
            return Err(CreateRampOrderError::InvalidResponse(
                "Ramp order direction mismatch",
            ));
        }
        Ok(order)
    }
}
fn normalize_create_order_response(bytes: &[u8]) -> Result<RampOrder, CreateRampOrderError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| CreateRampOrderError::InvalidResponse("malformed Ramp order"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    validate_order_response_shape(&payload)
        .map_err(|_| CreateRampOrderError::InvalidResponse("malformed Ramp order"))?;
    let response: OrderResponse = serde_json::from_value(payload)
        .map_err(|_| CreateRampOrderError::InvalidResponse("malformed Ramp order"))?;
    validation::order(&response.order).map_err(CreateRampOrderError::InvalidResponse)?;
    Ok(response.order)
}
