#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if matches!(arguments.as_slice(), [argument] if argument == "--help" || argument == "-h") {
        println!("custom_transaction: create or reuse a wallet and submit a Memo instruction.");
        println!("Required: SWIG_API_KEY, SWIG_RPC_URL, SWIG_ED25519_PRIVATE_KEY.");
        println!("Defaults to Devnet. See README.md for options.");
        return Ok(());
    }
    anyhow::ensure!(arguments.is_empty(), "Unexpected arguments. Use --help.");
    swig_developer_sdk_examples::custom_transaction::example().await
}
