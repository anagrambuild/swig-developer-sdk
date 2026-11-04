#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if matches!(arguments.as_slice(), [argument] if argument == "--help" || argument == "-h") {
        println!("secp256r1_transfer: create or reuse a Devnet wallet and transfer SOL.");
        println!("Required: SWIG_API_KEY, SWIG_RPC_URL, SWIG_SECP256R1_PRIVATE_KEY.");
        println!("Uses a raw secp256r1 key, not a browser passkey. See README.md for options.");
        return Ok(());
    }
    anyhow::ensure!(arguments.is_empty(), "Unexpected arguments. Use --help.");
    swig_developer_sdk_examples::secp256r1_transfer::example().await
}
