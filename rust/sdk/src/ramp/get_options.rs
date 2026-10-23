use crate::{http::HttpError, CryptoAsset, RampClient, RampEnvironment};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub enum RampDirection {
    #[serde(rename = "RAMP_DIRECTION_BUY", alias = "buy")]
    Buy,
    #[serde(rename = "RAMP_DIRECTION_SELL", alias = "sell")]
    Sell,
}

pub struct GetRampOptionsArgs {
    pub direction: RampDirection,
    pub country_code: Option<String>,
    pub fiat_currency_code: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RampSubdivision {
    pub code: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RampCountry {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub subdivisions: Vec<RampSubdivision>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RampFiatCurrencyOption {
    #[serde(alias = "currency_code")]
    pub currency_code: String,
    pub exponent: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RampAssetOption {
    pub asset: CryptoAsset,
    pub name: String,
    #[serde(alias = "icon_url", skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub decimals: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RampOptions {
    #[serde(default)]
    pub countries: Vec<RampCountry>,
    #[serde(default, alias = "fiat_currencies")]
    pub fiat_currencies: Vec<RampFiatCurrencyOption>,
    #[serde(default, alias = "payment_methods")]
    pub payment_methods: Vec<String>,
    #[serde(default)]
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

impl TryFrom<String> for RampDirection {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "RAMP_DIRECTION_BUY" | "buy" => Ok(Self::Buy),
            "RAMP_DIRECTION_SELL" | "sell" => Ok(Self::Sell),
            _ => Err("invalid Ramp direction"),
        }
    }
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
        let configuration_id = utf8_percent_encode(&config.configuration_id, NON_ALPHANUMERIC);
        let environment = match config.environment {
            RampEnvironment::Sandbox => "RAMP_ENVIRONMENT_SANDBOX",
            RampEnvironment::Production => "RAMP_ENVIRONMENT_PRODUCTION",
        };
        let direction = match args.direction {
            RampDirection::Buy => "RAMP_DIRECTION_BUY",
            RampDirection::Sell => "RAMP_DIRECTION_SELL",
        };
        let mut path = format!("/wallet/api/ramp/options?configurationId={configuration_id}&environment={environment}&direction={direction}");
        if let Some(country) = &args.country_code {
            path.push_str(&format!(
                "&countryCode={}",
                utf8_percent_encode(country, NON_ALPHANUMERIC)
            ));
        }
        if let Some(currency) = &args.fiat_currency_code {
            path.push_str(&format!(
                "&fiatCurrencyCode={}",
                utf8_percent_encode(currency, NON_ALPHANUMERIC)
            ));
        }
        let bytes = self.http.get(&path).await.map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => GetRampOptionsError::RequestFailed,
            HttpError::Timeout => GetRampOptionsError::RequestTimedOut,
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
        let mut options = normalize_get_options_response(&bytes)?;
        if args.country_code.is_none() {
            options.fiat_currencies.clear();
            options.payment_methods.clear();
            options.assets.clear();
        }
        Ok(options)
    }
}
fn normalize_get_options_response(bytes: &[u8]) -> Result<RampOptions, GetRampOptionsError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| GetRampOptionsError::InvalidResponse("malformed Ramp options"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    let fields = payload
        .as_object()
        .ok_or(GetRampOptionsError::InvalidResponse(
            "malformed Ramp options",
        ))?;
    if let Some(countries) = fields.get("countries").and_then(Value::as_array) {
        for country in countries {
            if !country.is_object()
                || country
                    .get("subdivisions")
                    .and_then(Value::as_array)
                    .is_some_and(|subdivisions| subdivisions.iter().any(|value| !value.is_object()))
            {
                return Err(GetRampOptionsError::InvalidResponse(
                    "malformed Ramp options",
                ));
            }
        }
    }
    for name in ["fiatCurrencies", "fiat_currencies", "assets"] {
        if fields
            .get(name)
            .and_then(Value::as_array)
            .is_some_and(|entries| entries.iter().any(|value| !value.is_object()))
        {
            return Err(GetRampOptionsError::InvalidResponse(
                "malformed Ramp options",
            ));
        }
    }
    let mut options: RampOptions = serde_json::from_value(payload)
        .map_err(|_| GetRampOptionsError::InvalidResponse("malformed Ramp options"))?;
    validate_options(&mut options).map_err(GetRampOptionsError::InvalidResponse)?;
    Ok(options)
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
