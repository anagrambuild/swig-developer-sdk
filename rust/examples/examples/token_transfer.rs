#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if matches!(arguments.as_slice(), [argument] if argument == "--help" || argument == "-h") {
        println!("token_transfer: transfer funded tokens from a Swig wallet.");
        println!(
            "Required: SWIG_API_KEY, SWIG_RPC_URL, SWIG_ED25519_PRIVATE_KEY, SWIG_TOKEN_MINT."
        );
        println!("Fund the wallet with the selected token before transferring. See README.md.");
        return Ok(());
    }
    anyhow::ensure!(arguments.is_empty(), "Unexpected arguments. Use --help.");
    swig_developer_sdk_examples::token_transfer::example().await
}
