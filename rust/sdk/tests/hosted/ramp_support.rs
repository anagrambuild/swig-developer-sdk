use crate::{
    hosted_support::config, ClientError, CryptoAmount, CryptoAsset, Network, RampClient,
    RampConfig, RampDirection, RampEnvironment, RampLocation, RampOrderRequest,
};
use serde_json::{json, Value};
pub(crate) fn client(network: Network) -> Result<RampClient, ClientError> {
    let mut config = config();
    config.network = network;
    RampClient::new(
        config,
        Some(RampConfig {
            configuration_id: "configuration".into(),
            environment: RampEnvironment::Sandbox,
        }),
    )
}
pub(crate) fn sell_request() -> RampOrderRequest {
    RampOrderRequest::Sell {
        sell: CryptoAmount {
            asset: CryptoAsset::Sol,
            base_units: u64::MAX,
        },
        receive_fiat_currency_code: "USD".into(),
    }
}

pub(crate) fn location() -> RampLocation {
    RampLocation {
        country_code: "US".into(),
        subdivision_code: Some("US-CA".into()),
    }
}

pub(crate) fn quote() -> Value {
    json!({
        "sell":{"asset":{"sol":{}},"baseUnits":"12"},
        "receive":{"currencyCode":"USD","minorUnits":"1"},
        "totalFee":{"currencyCode":"USD","minorUnits":"0"},
        "exchangeRate":"0.123"
    })
}
pub(crate) fn order(id: &str) -> Value {
    json!({"order": {
        "id":id,"status":"RAMP_ORDER_STATUS_AWAITING_TRANSFER",
        "createdAt":"2026-10-19T12:00:00Z","updatedAt":"2026-10-19T12:00:00Z",
        "sell":{"quote":quote(),"launchUrl":""}
    }})
}
pub(crate) fn positional_order_responses(
) -> Result<Vec<(RampDirection, Value)>, Box<dyn std::error::Error>> {
    let deposit = json!({"address":solana_pubkey::Pubkey::new_from_array([7;32]).to_string(),
        "amount":{"asset":{"sol":{}},"baseUnits":"12"}});
    let mut sell = order("saved");
    sell["order"]["sell"]["deposit"] = deposit.clone();
    sell["order"]["sell"]["transfer"] = transfer("transfer");
    let mut buy = order("saved");
    buy["order"].as_object_mut().ok_or("order")?.remove("sell");
    buy["order"]["buy"] = json!({"quote":{
        "spend":{"currencyCode":"USD","minorUnits":"1"},
        "receive":{"asset":{"sol":{}},"baseUnits":"12"},
        "totalFee":{"currencyCode":"USD","minorUnits":"0"},"exchangeRate":"1"
    },"launchUrl":null});
    let mut responses = Vec::new();
    for (direction, name, payload, amount_field, fiat_field) in [
        (RampDirection::Buy, "buy", buy, "receive", "spend"),
        (RampDirection::Sell, "sell", sell, "sell", "receive"),
    ] {
        let order = &payload["order"];
        let details = &order[name];
        let quote = &details["quote"];
        let details_array = if direction == RampDirection::Buy {
            json!([quote, null])
        } else {
            json!([quote, null, deposit, transfer("transfer")])
        };
        let quote_array = if direction == RampDirection::Buy {
            json!([quote["spend"], quote["receive"], quote["totalFee"], "1"])
        } else {
            json!([quote["sell"], quote["receive"], quote["totalFee"], "1"])
        };
        let mut cases = vec![
            (String::new(), json!([order])),
            (
                "/order".into(),
                json!([
                    order["id"],
                    order["status"],
                    order["createdAt"],
                    order["updatedAt"],
                    order["buy"],
                    order["sell"]
                ]),
            ),
            (format!("/order/{name}"), details_array),
            (format!("/order/{name}/quote"), quote_array),
            (
                format!("/order/{name}/quote/{amount_field}"),
                json!([{"sol":{}},"12"]),
            ),
            (
                format!("/order/{name}/quote/{fiat_field}"),
                json!(["USD", "1"]),
            ),
            (format!("/order/{name}/quote/totalFee"), json!(["USD", "0"])),
            (
                format!("/order/{name}/quote/{amount_field}/asset/sol"),
                json!([]),
            ),
        ];
        if direction == RampDirection::Sell {
            cases.extend([
                (
                    "/order/sell/deposit".into(),
                    json!([deposit["address"], deposit["amount"]]),
                ),
                (
                    "/order/sell/deposit/amount".into(),
                    json!([{"sol":{}},"12"]),
                ),
                ("/order/sell/deposit/amount/asset/sol".into(), json!([])),
                (
                    "/order/sell/transfer".into(),
                    json!([
                        "transfer",
                        "TRANSFER_STATE_PREPARED",
                        null,
                        "2026-10-19T12:00:00Z"
                    ]),
                ),
            ]);
        }
        for (path, replacement) in cases {
            let mut invalid = payload.clone();
            *invalid.pointer_mut(&path).ok_or("order path")? = replacement;
            responses.push((direction, invalid));
        }
    }
    Ok(responses)
}

pub(crate) fn transfer(id: &str) -> Value {
    json!({"transferId":id,"state":"TRANSFER_STATE_PREPARED","expiresAt":"2026-10-19T12:00:00Z"})
}
