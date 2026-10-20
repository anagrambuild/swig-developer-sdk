use crate::ParticipantApproval;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields, rename_all_fields = "camelCase")]
pub(crate) enum ApprovalWire {
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

impl From<ParticipantApproval> for ApprovalWire {
    fn from(value: ParticipantApproval) -> Self {
        match value {
            ParticipantApproval::Ed25519 {
                member_index,
                signature,
            } => Self::Ed25519 {
                member_index,
                ed25519: SignatureProof { signature },
            },
            ParticipantApproval::Secp256k1 {
                member_index,
                signature,
            } => Self::Secp256k1 {
                member_index,
                secp256k1: SignatureProof { signature },
            },
            ParticipantApproval::WebAuthnP256 {
                member_index,
                authenticator_data,
                client_data_json,
                signature,
            } => Self::WebAuthnP256 {
                member_index,
                webauthn_p256: WebAuthnProof {
                    authenticator_data,
                    client_data_json,
                    signature,
                },
            },
        }
    }
}

impl From<ApprovalWire> for ParticipantApproval {
    fn from(value: ApprovalWire) -> Self {
        match value {
            ApprovalWire::Ed25519 {
                member_index,
                ed25519,
            } => Self::Ed25519 {
                member_index,
                signature: ed25519.signature,
            },
            ApprovalWire::Secp256k1 {
                member_index,
                secp256k1,
            } => Self::Secp256k1 {
                member_index,
                signature: secp256k1.signature,
            },
            ApprovalWire::WebAuthnP256 {
                member_index,
                webauthn_p256,
            } => Self::WebAuthnP256 {
                member_index,
                authenticator_data: webauthn_p256.authenticator_data,
                client_data_json: webauthn_p256.client_data_json,
                signature: webauthn_p256.signature,
            },
        }
    }
}
