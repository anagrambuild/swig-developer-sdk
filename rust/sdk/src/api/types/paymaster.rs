use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::api::error::ApiConversionError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "PaymasterKindValue")]
pub(crate) enum PaymasterKind {
    Unspecified,
    Api,
    Idp,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PaymasterKindValue {
    Number(u32),
    Name(String),
}

impl TryFrom<PaymasterKindValue> for PaymasterKind {
    type Error = ApiConversionError;

    fn try_from(value: PaymasterKindValue) -> Result<Self, Self::Error> {
        match value {
            PaymasterKindValue::Number(0) => Ok(Self::Unspecified),
            PaymasterKindValue::Number(1) => Ok(Self::Api),
            PaymasterKindValue::Number(2) => Ok(Self::Idp),
            PaymasterKindValue::Name(value) => match value.as_str() {
                "PAYMASTER_KIND_UNSPECIFIED" => Ok(Self::Unspecified),
                "PAYMASTER_KIND_API" | "api" => Ok(Self::Api),
                "PAYMASTER_KIND_IDP" | "idp" => Ok(Self::Idp),
                _ => Err(ApiConversionError::Invalid("invalid paymaster kind")),
            },
            _ => Err(ApiConversionError::Invalid("invalid paymaster kind")),
        }
    }
}

#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaymasterBalance {
    #[serde(default)]
    pub configured: bool,
    pub kind: Option<PaymasterKind>,
    pub address: Option<String>,
    pub id: Option<String>,
    pub label: Option<String>,
    #[serde(default, alias = "balance_lamports")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub balance_lamports: Option<u64>,
    #[serde(alias = "balance_sol")]
    pub balance_sol: Option<f64>,
}
