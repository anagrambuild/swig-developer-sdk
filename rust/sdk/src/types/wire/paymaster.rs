use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum PaymasterKindWire {
    Name(PaymasterKindName),
    Number(u32),
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum PaymasterKindName {
    Api,
    Idp,
}

impl TryFrom<String> for PaymasterKindName {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "PAYMASTER_KIND_API" | "api" => Ok(Self::Api),
            "PAYMASTER_KIND_IDP" | "idp" => Ok(Self::Idp),
            _ => Err("invalid paymaster kind"),
        }
    }
}

#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaymasterBalanceWire {
    pub kind: PaymasterKindWire,
    pub address: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default, alias = "balance_lamports")]
    #[serde_as(as = "DisplayFromStr")]
    pub balance_lamports: u64,
    #[serde(default, alias = "balance_sol")]
    pub balance_sol: f64,
}

#[derive(Deserialize)]
pub(crate) struct PaymasterConfigurationWire {
    #[serde(default)]
    pub configured: bool,
}
