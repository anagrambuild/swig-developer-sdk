//! Compile-only. Calling `example` spends Mainnet SOL from the Ed25519 signer.
//! The API paymaster supplies fees and a 1,000-lamport Jito tip. No wallet is created.

use std::env;

use anyhow::{anyhow, bail, ensure, Result};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_pubkey::{pubkey, Pubkey};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::{null_signer::NullSigner, Signer};
use solana_system_interface::instruction::transfer;
use solana_transaction::versioned::VersionedTransaction;
use swig_developer_sdk::{
    ClientConfig, Network, PaymasterClient, RecentBlockhash, SponsorSignedTransactionBundleArgs,
};

const JITO_TIP_ACCOUNT: Pubkey = pubkey!("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5");

pub async fn example() -> Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(error) if error.not_found() => {}
        Err(_) => bail!("Could not load .env. Check its syntax and permissions."),
    }
    ensure!(
        env::var("SWIG_NETWORK").as_deref() == Ok("mainnet"),
        "Bundle sponsorship requires SWIG_NETWORK=mainnet"
    );
    let signer = load_ed25519_signer()?;
    let destination: Pubkey = env::var("SWIG_DESTINATION")
        .map_err(|_| anyhow!("Set SWIG_DESTINATION to an explicitly chosen Mainnet recipient"))?
        .parse()?;
    let amount: u64 = match env::var("SWIG_AMOUNT_LAMPORTS") {
        Ok(value) => value.parse()?,
        Err(env::VarError::NotPresent) => 1_000_000,
        Err(_) => bail!("SWIG_AMOUNT_LAMPORTS must be valid text"),
    };
    ensure!(amount > 0, "Transfer amount must be positive");
    let mut config = ClientConfig::new(env::var("SWIG_API_KEY")?, Network::Mainnet);
    config.http.base_url = match env::var("SWIG_API_URL") {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(_) => bail!("SWIG_API_URL must be valid text"),
    };
    let rpc_url = env::var("SWIG_RPC_URL").map_err(|_| anyhow!("Set SWIG_RPC_URL"))?;
    let rpc_url = url::Url::parse(&rpc_url)
        .map_err(|_| anyhow!("SWIG_RPC_URL must be a valid HTTP(S) URL"))?;
    ensure!(
        matches!(rpc_url.scheme(), "http" | "https") && rpc_url.host_str().is_some(),
        "SWIG_RPC_URL must be a valid HTTP(S) URL"
    );
    let rpc = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
    let paymaster = PaymasterClient::new(config)?;
    let fee_payer = paymaster.get_balance().await?.address;
    ensure!(
        signer.pubkey() != fee_payer && destination != fee_payer && destination != signer.pubkey(),
        "Use distinct signer, recipient, and API fee payer addresses"
    );
    let total = amount
        .checked_mul(2)
        .ok_or_else(|| anyhow!("Transfer amount is too large"))?;
    let signer_account = rpc
        .get_account(&signer.pubkey())
        .await
        .map_err(|_| anyhow!("Could not load the signer account from RPC"))?;
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(signer_account.data.len())
        .await
        .map_err(|_| anyhow!("Could not fetch the rent requirement from RPC"))?;
    ensure!(
        signer_account.lamports
            >= total
                .checked_add(rent)
                .ok_or_else(|| anyhow!("Amount is too large"))?,
        "Fund the signer with enough Mainnet SOL for both transfers. No airdrop is attempted."
    );
    let transactions = build_bundle(
        &signer,
        fee_payer,
        destination,
        amount,
        rpc.get_latest_blockhash()
            .await
            .map_err(|_| anyhow!("Could not fetch the latest blockhash from RPC"))?,
    )?;
    println!("Signer: {}", signer.pubkey());
    println!("Recipient: {destination}. Two transfers total {total} lamports.");
    println!("The second transaction also pays the Jito tip from {fee_payer}.");

    // Preserve this exact ordered bundle until its outcome is known. Do not rebuild on failure.
    let receipt = paymaster
        .sponsor_bundle(SponsorSignedTransactionBundleArgs {
            transactions,
            idempotency_key: None,
        })
        .await
        .map_err(|error| {
            anyhow!(
                "Bundle submission failed or is uncertain: {error}. Reconcile before rerunning."
            )
        })?;
    println!("Bundle: {}", receipt.bundle_id);
    for signature in &receipt.signatures {
        println!("Submitted: {signature}");
        rpc.poll_for_signature(signature)
            .await
            .map_err(|_| anyhow!("Confirmation unknown for {signature}. Do not resubmit."))?;
        ensure!(
            rpc.confirm_transaction(signature).await.map_err(|_| {
                anyhow!("Confirmation unknown for {signature}. Inspect it before rerunning.")
            })?,
            "Transaction was not confirmed successfully: {signature}"
        );
    }
    Ok(())
}

fn build_bundle(
    signer: &Keypair,
    fee_payer: Pubkey,
    destination: Pubkey,
    amount: u64,
    blockhash: RecentBlockhash,
) -> Result<Vec<VersionedTransaction>> {
    let first = Message::new_with_blockhash(
        &[transfer(&signer.pubkey(), &destination, amount)],
        Some(&fee_payer),
        &blockhash,
    );
    let second = Message::new_with_blockhash(
        &[
            transfer(&signer.pubkey(), &destination, amount),
            transfer(&fee_payer, &JITO_TIP_ACCOUNT, 1_000),
        ],
        Some(&fee_payer),
        &blockhash,
    );
    let unsigned_paymaster = NullSigner::new(&fee_payer);
    let signers: [&dyn Signer; 2] = [&unsigned_paymaster, signer];
    Ok(vec![
        VersionedTransaction::try_new(VersionedMessage::Legacy(first), &signers)?,
        VersionedTransaction::try_new(VersionedMessage::Legacy(second), &signers)?,
    ])
}

fn load_ed25519_signer() -> Result<Keypair> {
    let encoded = env::var("SWIG_ED25519_PRIVATE_KEY")
        .map_err(|_| anyhow!("Set SWIG_ED25519_PRIVATE_KEY to a hex-encoded 64-byte keypair"))?;
    let bytes = hex::decode(encoded.trim()).map_err(|_| anyhow!("Invalid Ed25519 key hex"))?;
    Keypair::try_from(bytes.as_slice()).map_err(|_| anyhow!("Invalid 64-byte Ed25519 keypair"))
}
