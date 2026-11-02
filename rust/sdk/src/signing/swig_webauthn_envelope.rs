use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};

const SOFTWARE_RP_ID: &str = "swig.invalid";
const SOFTWARE_ORIGIN: &str = "https://swig.invalid";

pub(super) struct SoftwareWebAuthnEnvelope {
    pub(super) authenticator_data: Vec<u8>,
    pub(super) client_data_json: Vec<u8>,
    pub(super) message: Vec<u8>,
}

pub(super) fn swig_webauthn_envelope(challenge: &[u8; 32]) -> SoftwareWebAuthnEnvelope {
    let challenge = URL_SAFE_NO_PAD.encode(challenge);
    let client_data_json = format!(
        r#"{{"type":"webauthn.get","challenge":"{challenge}","origin":"{SOFTWARE_ORIGIN}"}}"#,
    )
    .into_bytes();
    let mut authenticator_data = Sha256::digest(SOFTWARE_RP_ID.as_bytes()).to_vec();
    // The compiler requires this flag. In this software envelope it is not
    // evidence of a browser ceremony or human presence.
    authenticator_data.push(0x01);
    authenticator_data.extend_from_slice(&0_u32.to_be_bytes());

    let mut message = authenticator_data.clone();
    message.extend_from_slice(&Sha256::digest(&client_data_json));
    SoftwareWebAuthnEnvelope {
        authenticator_data,
        client_data_json,
        message,
    }
}
