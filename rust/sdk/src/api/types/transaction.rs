use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use super::encoding::SignatureScheme;

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientSignatureRequest {
    pub scheme: SignatureScheme,
    pub signer: String,
    #[serde(alias = "message_hash")]
    pub message_hash: String,
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    pub counter: u32,
}
