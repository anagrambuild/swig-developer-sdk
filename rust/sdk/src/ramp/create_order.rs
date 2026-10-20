use super::validation;
use crate::{
    api::{
        self,
        types::{
            ramp::Environment,
            ramp_create_order::{CreateOrderRequest, OrderContext},
        },
    },
    http::HttpError,
    RampClient, RampEnvironment, RampLocation, RampOrder, RampOrderRequest, RampRoute,
};
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
        mut args: CreateRampOrderArgs,
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
        validation::normalize_order_request(&mut args.order)
            .map_err(CreateRampOrderError::InvalidInput)?;
        let request = CreateOrderRequest {
            request_id: args.request_id.clone(),
            configuration_id: config.configuration_id.clone(),
            environment: match config.environment {
                RampEnvironment::Sandbox => Environment::Sandbox,
                RampEnvironment::Production => Environment::Production,
            },
            context: OrderContext {
                customer_id: args.context.customer_id,
                swig_config_address: args.context.swig_config_address,
                network: self.network.into(),
                location: args.context.location.into(),
            },
            route: args.route.into(),
            order: args.order.clone().into(),
        };
        let response = api::create_ramp_order::create_ramp_order(&self.http, request)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    CreateRampOrderError::RequestFailed
                }
                HttpError::Timeout => CreateRampOrderError::RequestTimedOut,
                HttpError::InvalidResponse(message) => {
                    CreateRampOrderError::InvalidResponse(message)
                }
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
        let order = RampOrder::try_from(response.order)
            .map_err(|_| CreateRampOrderError::InvalidResponse("malformed Ramp order"))?;
        validation::created_order(&order, &args.order)
            .map_err(CreateRampOrderError::InvalidResponse)?;
        Ok(order)
    }
}
