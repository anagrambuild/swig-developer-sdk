use super::validation;
use crate::{
    http::HttpError, RampBuyQuote, RampClient, RampEnvironment, RampLocation, RampOrderRequest,
    RampRoute, RampSellQuote,
};
use thiserror::Error;
pub struct GetRampQuotesArgs {
    pub location: RampLocation,
    pub order: RampOrderRequest,
}

#[derive(Clone)]
pub enum RampQuoteDetails {
    Buy(RampBuyQuote),
    Sell(RampSellQuote),
}

#[derive(Clone)]
pub struct RampQuote {
    pub route: RampRoute,
    pub details: RampQuoteDetails,
}

#[derive(Debug, Error)]
pub enum GetRampQuotesError {
    #[error("Ramp configuration is required for this operation")]
    MissingConfiguration,
    #[error("invalid Ramp quote input: {0}")]
    InvalidInput(&'static str),

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

    #[error("invalid Ramp quotes response: {0}")]
    InvalidResponse(&'static str),

    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl RampClient {
    pub async fn get_quotes(
        &self,
        mut args: GetRampQuotesArgs,
    ) -> Result<Vec<RampQuote>, GetRampQuotesError> {
        let config = self
            .ramp_config
            .as_ref()
            .ok_or(GetRampQuotesError::MissingConfiguration)?;
        validation::location(&args.location).map_err(GetRampQuotesError::InvalidInput)?;
        validation::normalize_order_request(&mut args.order)
            .map_err(GetRampQuotesError::InvalidInput)?;
        let request = crate::api::types::ramp_quote::GetQuotesRequest {
            configuration_id: config.configuration_id.clone(),
            environment: match config.environment {
                RampEnvironment::Sandbox => crate::api::types::ramp::Environment::Sandbox,
                RampEnvironment::Production => crate::api::types::ramp::Environment::Production,
            },
            location: args.location.into(),
            order: args.order.clone().into(),
        };
        let response = crate::api::get_ramp_quotes::get_ramp_quotes(&self.http, request)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    GetRampQuotesError::RequestFailed
                }
                HttpError::Timeout => GetRampQuotesError::RequestTimedOut,
                HttpError::InvalidResponse(message) => GetRampQuotesError::InvalidResponse(message),
                HttpError::ResponseTooLarge => {
                    GetRampQuotesError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => GetRampQuotesError::AuthenticationFailed,
                HttpError::PermissionDenied => GetRampQuotesError::PermissionDenied,
                HttpError::InvalidArgument => {
                    GetRampQuotesError::InvalidInput("service rejected the request")
                }
                HttpError::Unavailable => GetRampQuotesError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => GetRampQuotesError::UnexpectedServiceFailure,
            })?;
        let quotes = response
            .quotes
            .into_iter()
            .map(RampQuote::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| GetRampQuotesError::InvalidResponse("malformed Ramp quotes"))?;
        for quote in &quotes {
            validation::quote(quote, &args.order).map_err(GetRampQuotesError::InvalidResponse)?;
        }
        Ok(quotes)
    }
}

impl TryFrom<crate::api::types::ramp_quote::Quote> for RampQuote {
    type Error = crate::api::error::ApiConversionError;

    fn try_from(value: crate::api::types::ramp_quote::Quote) -> Result<Self, Self::Error> {
        let details = match (value.buy, value.sell) {
            (Some(buy), None) => RampQuoteDetails::Buy(buy.into()),
            (None, Some(sell)) => RampQuoteDetails::Sell(sell.into()),
            _ => {
                return Err(Self::Error::Invalid(
                    "Ramp quote requires exactly one direction",
                ))
            }
        };
        Ok(Self {
            route: value.route.into(),
            details,
        })
    }
}
