use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::ClientSignatureRequest;

use super::{decode_hex32, AuthorityWire, SignatureSchemeWire, WireError};

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientSignatureRequestWire {
    pub scheme: SignatureSchemeWire,
    pub signer: String,
    #[serde(alias = "message_hash")]
    pub message_hash: String,
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    pub counter: u32,
}

impl TryFrom<ClientSignatureRequestWire> for ClientSignatureRequest {
    type Error = WireError;

    fn try_from(value: ClientSignatureRequestWire) -> Result<Self, Self::Error> {
        let authority = match value.scheme {
            SignatureSchemeWire::Secp256k1 => AuthorityWire::Secp256k1 {
                public_key: value.signer,
            },
            SignatureSchemeWire::Secp256r1 => AuthorityWire::Secp256r1 {
                public_key: value.signer,
            },
        }
        .try_into()?;
        Ok(Self {
            authority,
            message_hash: decode_hex32(&value.message_hash)?,
            slot: value.slot,
            counter: value.counter,
        })
    }
}

impl TryFrom<ClientSignatureRequest> for ClientSignatureRequestWire {
    type Error = WireError;

    fn try_from(value: ClientSignatureRequest) -> Result<Self, Self::Error> {
        let (scheme, signer) = match AuthorityWire::try_from(value.authority)? {
            AuthorityWire::Secp256k1 { public_key } => (SignatureSchemeWire::Secp256k1, public_key),
            AuthorityWire::Secp256r1 { public_key } => (SignatureSchemeWire::Secp256r1, public_key),
            _ => return Err(WireError::Invalid("unsupported client signature authority")),
        };
        Ok(Self {
            scheme,
            signer,
            message_hash: hex::encode(value.message_hash),
            slot: value.slot,
            counter: value.counter,
        })
    }
}
