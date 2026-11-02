use super::ramp_order::Transfer;
use serde::{Deserialize, Serialize};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubmitTransferRequest {
    #[serde(skip)]
    pub order_id: String,
    pub transfer_id: String,
    pub signed_transaction: String,
}
#[derive(Deserialize)]
pub(crate) struct SubmitResponse {
    pub transfer: Transfer,
}
