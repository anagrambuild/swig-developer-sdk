#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if matches!(arguments.as_slice(), [argument] if argument == "--help" || argument == "-h") {
        println!("ed25519_transfer: create or reuse a Devnet wallet and transfer SOL.");
        println!("Required: SWIG_API_KEY, SWIG_ED25519_PRIVATE_KEY, SWIG_RPC_URL.");
        println!(
            "Optional: SWIG_DESTINATION, SWIG_CONFIG_ADDRESS, SWIG_WALLET_ADDRESS. Devnet only."
        );
        return Ok(());
    }
    anyhow::ensure!(arguments.is_empty(), "Unexpected arguments. Use --help.");
    swig_developer_sdk_examples::ed25519_transfer::example().await
}
