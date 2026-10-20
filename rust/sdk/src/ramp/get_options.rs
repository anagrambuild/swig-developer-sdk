use crate::{http::HttpError, CryptoAsset, RampClient, RampEnvironment};
use thiserror::Error;
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RampDirection {
    Buy,
    Sell,
}

pub struct GetRampOptionsArgs {
    pub direction: RampDirection,
    pub country_code: Option<String>,
    pub fiat_currency_code: Option<String>,
}

#[derive(Clone)]
pub struct RampSubdivision {
    pub code: String,
    pub name: String,
}

#[derive(Clone)]
pub struct RampCountry {
    pub code: String,
    pub name: String,
    pub subdivisions: Vec<RampSubdivision>,
}

#[derive(Clone)]
pub struct RampFiatCurrencyOption {
    pub currency_code: String,
    pub exponent: u32,
}

#[derive(Clone)]
pub struct RampAssetOption {
    pub asset: CryptoAsset,
    pub name: String,
    pub icon_url: Option<String>,
    pub decimals: u32,
}

#[derive(Clone)]
pub struct RampOptions {
    pub countries: Vec<RampCountry>,
    pub fiat_currencies: Vec<RampFiatCurrencyOption>,
    pub payment_methods: Vec<String>,
    pub assets: Vec<RampAssetOption>,
}

#[derive(Debug, Error)]
pub enum GetRampOptionsError {
    #[error("Ramp configuration is required for this operation")]
    MissingConfiguration,

    #[error("invalid Ramp options input: {0}")]
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

    #[error("invalid Ramp options response: {0}")]
    InvalidResponse(&'static str),

    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl RampClient {
    pub async fn get_options(
        &self,
        args: GetRampOptionsArgs,
    ) -> Result<RampOptions, GetRampOptionsError> {
        let config = self
            .ramp_config
            .as_ref()
            .ok_or(GetRampOptionsError::MissingConfiguration)?;
        if args
            .country_code
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
            || args
                .fiat_currency_code
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(GetRampOptionsError::InvalidInput(
                "country and currency must not be empty",
            ));
        }
        let request = crate::api::types::ramp_options::GetOptionsRequest {
            configuration_id: config.configuration_id.clone(),
            environment: match config.environment {
                RampEnvironment::Sandbox => crate::api::types::ramp::Environment::Sandbox,
                RampEnvironment::Production => crate::api::types::ramp::Environment::Production,
            },
            direction: match args.direction {
                RampDirection::Buy => crate::api::types::ramp::Direction::Buy,
                RampDirection::Sell => crate::api::types::ramp::Direction::Sell,
            },
            country_code: args.country_code.clone(),
            fiat_currency_code: args.fiat_currency_code,
        };
        let response = crate::api::get_ramp_options::get_ramp_options(&self.http, request)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    GetRampOptionsError::RequestFailed
                }
                HttpError::Timeout => GetRampOptionsError::RequestTimedOut,
                HttpError::InvalidResponse(message) => {
                    GetRampOptionsError::InvalidResponse(message)
                }
                HttpError::ResponseTooLarge => {
                    GetRampOptionsError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => GetRampOptionsError::AuthenticationFailed,
                HttpError::PermissionDenied => GetRampOptionsError::PermissionDenied,
                HttpError::InvalidArgument => {
                    GetRampOptionsError::InvalidInput("service rejected the request")
                }
                HttpError::Unavailable => GetRampOptionsError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => GetRampOptionsError::UnexpectedServiceFailure,
            })?;
        let mut options = RampOptions {
            countries: response
                .countries
                .into_iter()
                .map(|country| RampCountry {
                    code: country.code,
                    name: country.name,
                    subdivisions: country
                        .subdivisions
                        .into_iter()
                        .map(|subdivision| RampSubdivision {
                            code: subdivision.code,
                            name: subdivision.name,
                        })
                        .collect(),
                })
                .collect(),
            fiat_currencies: response
                .fiat_currencies
                .into_iter()
                .map(|currency| RampFiatCurrencyOption {
                    currency_code: currency.currency_code,
                    exponent: currency.exponent,
                })
                .collect(),
            payment_methods: response.payment_methods,
            assets: response
                .assets
                .into_iter()
                .map(|asset| RampAssetOption {
                    asset: asset.asset.into(),
                    name: asset.name,
                    icon_url: asset.icon_url,
                    decimals: asset.decimals,
                })
                .collect(),
        };
        validate_options(&mut options).map_err(GetRampOptionsError::InvalidResponse)?;
        if args.country_code.is_none() {
            options.fiat_currencies.clear();
            options.payment_methods.clear();
            options.assets.clear();
        }
        Ok(options)
    }
}
fn validate_options(options: &mut RampOptions) -> Result<(), &'static str> {
    if options.countries.iter().any(|country| {
        country.code.is_empty()
            || country.name.is_empty()
            || country
                .subdivisions
                .iter()
                .any(|subdivision| subdivision.code.is_empty() || subdivision.name.is_empty())
    }) {
        return Err("invalid Ramp country");
    }
    if options
        .fiat_currencies
        .iter()
        .any(|currency| currency.currency_code.is_empty())
        || options.payment_methods.iter().any(String::is_empty)
    {
        return Err("invalid Ramp currency or payment method");
    }
    for asset in &mut options.assets {
        if asset.name.is_empty() {
            return Err("invalid Ramp asset");
        }
        asset.icon_url = asset.icon_url.take().filter(|url| !url.is_empty());
    }
    Ok(())
}
