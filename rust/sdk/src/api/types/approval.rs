use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields, rename_all_fields = "camelCase")]
pub(crate) enum ParticipantApproval {
    Ed25519 {
        member_index: u32,
        ed25519: SignatureProof,
    },
    Secp256k1 {
        member_index: u32,
        secp256k1: SignatureProof,
    },
    WebAuthnP256 {
        member_index: u32,
        webauthn_p256: WebAuthnProof,
    },
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignatureProof {
    #[serde_as(as = "Base64")]
    pub signature: Vec<u8>,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WebAuthnProof {
    #[serde_as(as = "Base64")]
    pub authenticator_data: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub client_data_json: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub signature: Vec<u8>,
}

#[cfg(test)]
#[path = "../../../tests/api/approval.rs"]
mod tests;
