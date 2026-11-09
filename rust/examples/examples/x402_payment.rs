#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if matches!(arguments.as_slice(), [argument] if argument == "--help" || argument == "-h") {
        println!("x402_payment: pay the TypeScript merchant using a funded Ed25519 Swig wallet.");
        println!("Start the TypeScript x402 example with --server-only first.");
        println!(
            "Required: SWIG_API_KEY, SWIG_RPC_URL, SWIG_ED25519_PRIVATE_KEY, SWIG_CONFIG_ADDRESS."
        );
        println!("Copy its public SWIG_X402_RESOURCE_URL, SWIG_X402_ASSET, SWIG_X402_PAY_TO, and SWIG_X402_MAX_AMOUNT exports. Devnet only.");
        return Ok(());
    }
    anyhow::ensure!(arguments.is_empty(), "Unexpected arguments. Use --help.");
    swig_developer_sdk_examples::x402_payment::example().await
}
