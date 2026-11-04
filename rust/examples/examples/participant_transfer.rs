#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if matches!(arguments.as_slice(), [argument] if argument == "--help" || argument == "-h") {
        println!("participant_transfer: create a 2-of-2 Ed25519 + secp256r1 set and transfer Devnet SOL.");
        println!("Required: SWIG_API_KEY, SWIG_RPC_URL, SWIG_ED25519_PRIVATE_KEY, SWIG_SECP256R1_PRIVATE_KEY.");
        println!(
            "The Ed25519 manager is also a participant. Each run grants a fresh set an All role."
        );
        println!("Uses software WebAuthn, not a browser ceremony. Requires deployed ParticipantSet/WebAuthn support.");
        return Ok(());
    }
    anyhow::ensure!(arguments.is_empty(), "Unexpected arguments. Use --help.");
    swig_developer_sdk_examples::participant_transfer::example().await
}
