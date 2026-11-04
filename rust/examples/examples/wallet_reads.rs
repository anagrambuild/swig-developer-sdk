#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args().skip(1);
    let policy_id = match arguments.next().as_deref() {
        Some("--help" | "-h") => {
            println!("wallet_reads [--policy-id ID]\nRead roles, balances, and history. Optionally read a policy.\nRequired: SWIG_API_KEY, SWIG_CONFIG_ADDRESS. No signing key needed.");
            return Ok(());
        }
        Some("--policy-id") => {
            let Some(id) = arguments.next().filter(|value| !value.trim().is_empty()) else {
                anyhow::bail!("--policy-id requires a policy ID. Use --help.");
            };
            Some(id)
        }
        None => None,
        Some(_) => anyhow::bail!("Unexpected arguments. Use --help."),
    };
    anyhow::ensure!(
        arguments.next().is_none(),
        "Unexpected arguments. Use --help."
    );
    swig_developer_sdk_examples::wallet_reads::example(policy_id).await
}
