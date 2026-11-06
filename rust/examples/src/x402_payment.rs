//! Devnet payment using the TypeScript merchant and facilitator in server-only mode.
//! The resource owns payment settlement. Never sponsor or submit its payment separately.

use std::env;

use anyhow::{anyhow, bail, ensure, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_pubkey::{pubkey, Pubkey};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signature::Signature;
use solana_signer::{null_signer::NullSigner, Signer};
use solana_transaction::versioned::VersionedTransaction;
use swig_developer_sdk::{
    x402, ClientConfig, Network, PaymentRequirement, PrepareX402PaymentArgs, WalletAddressInfo,
    WalletAuthority, WalletClient,
};
use url::Url;

const SWIG_PROGRAM_ID: Pubkey = pubkey!("swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB");

#[derive(Deserialize)]
struct SettlementReceipt {
    success: bool,
    transaction: String,
    network: String,
}

pub async fn example() -> Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(error) if error.not_found() => {}
        Err(_) => bail!("Could not load .env. Check its syntax and permissions."),
    }
    ensure!(
        matches!(
            env::var("SWIG_NETWORK").as_deref(),
            Ok("devnet") | Err(env::VarError::NotPresent)
        ),
        "This example is for local/Devnet use only"
    );
    let signer = load_ed25519_signer()?;
    let resource_url = Url::parse(&env::var("SWIG_X402_RESOURCE_URL")?)?;
    ensure!(
        resource_url.username().is_empty()
            && resource_url.password().is_none()
            && resource_url.fragment().is_none()
            && (resource_url.scheme() == "https"
                || resource_url.scheme() == "http"
                    && matches!(
                        resource_url.host_str(),
                        Some("localhost" | "127.0.0.1" | "[::1]")
                    )),
        "Use HTTPS or loopback HTTP without URL credentials or fragment"
    );
    let approved_asset: Pubkey = env::var("SWIG_X402_ASSET")?.parse()?;
    let approved_recipient: Pubkey = env::var("SWIG_X402_PAY_TO")?.parse()?;
    let maximum_amount: u64 = env::var("SWIG_X402_MAX_AMOUNT")?.parse()?;
    ensure!(maximum_amount > 0, "The payment limit must be positive");
    let expected_network = "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1";
    let mut config = ClientConfig::new(env::var("SWIG_API_KEY")?, Network::Devnet);
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
    let client = WalletClient::new(config)?;
    let authority = WalletAuthority::Ed25519 {
        public_key: signer.pubkey(),
    };
    let config_address = env::var("SWIG_CONFIG_ADDRESS")
        .map_err(|_| anyhow!("Set SWIG_CONFIG_ADDRESS from the TypeScript server-only setup"))?
        .parse()?;
    let wallet = require_existing_wallet(&rpc, config_address).await?;
    println!("Ed25519 payer: {}", signer.pubkey());
    println!("SWIG_CONFIG_ADDRESS={}", wallet.swig_config_address);
    println!("SWIG_WALLET_ADDRESS={}", wallet.wallet_address);

    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let response = http
        .get(resource_url.clone())
        .send()
        .await
        .map_err(|_| anyhow!("Resource lookup failed"))?;
    let required = x402::parse_payment_required(response.status(), response.headers())?;
    ensure!(
        Url::parse(&required.resource.url)? == resource_url,
        "Requirement belongs to another resource"
    );
    let expected_index = required.accepts.iter().position(|requirement| {
        matches!(requirement, PaymentRequirement::ExactSvm(offer)
            if offer.network == expected_network
                && offer.asset == approved_asset
                && offer.pay_to == approved_recipient
                && offer.amount.parse::<u64>().is_ok_and(|amount| amount > 0 && amount <= maximum_amount))
    }).ok_or_else(|| anyhow!("No offer matches the approved network, asset, recipient, and limit"))?;
    let prepared = client
        .prepare_x402_payment(PrepareX402PaymentArgs {
            swig_config_address: wallet.swig_config_address,
            requester_authority: authority,
            payment_required: required,
            accepted_index: None,
        })
        .await?;
    ensure!(
        prepared.accepted_index == u32::try_from(expected_index)?,
        "The backend selected a different payment offer than the one approved locally"
    );
    let Some(PaymentRequirement::ExactSvm(offer)) = prepared
        .payment_required
        .accepts
        .get(usize::try_from(prepared.accepted_index)?)
    else {
        bail!("Expected the selected exact-SVM offer");
    };
    let facilitator_key = offer.extra.fee_payer;
    let transaction = prepared.prepared_transaction;
    transaction.transaction.sanitize()?;
    ensure!(
        transaction
            .wallet
            .as_ref()
            .is_some_and(
                |value| value.swig_config_address == wallet.swig_config_address
                    && value.wallet_address == wallet.wallet_address
            )
            && transaction.signature_requests.is_empty()
            && transaction.participant_set_approval_plan.is_none()
            && transaction
                .transaction
                .signatures
                .iter()
                .all(|value| *value == Signature::default()),
        "Expected an unsigned Ed25519 payment for this wallet"
    );
    let message = transaction.transaction.message;
    ensure!(
        facilitator_key != signer.pubkey()
            && message.header().num_required_signatures == 2
            && message.static_account_keys().first() == Some(&facilitator_key)
            && message.static_account_keys().get(1) == Some(&signer.pubkey()),
        "Expected the facilitator and requester as transaction signers"
    );
    // Offer checks do not independently audit instructions. This example trusts backend construction.
    let facilitator = NullSigner::new(&facilitator_key);
    let signers: [&dyn Signer; 2] = [&facilitator, &signer];
    let signed = VersionedTransaction::try_new(message, &signers)?;
    let payment =
        x402::create_x402_payment(prepared.payment_required, prepared.accepted_index, signed)?;

    // This is the only payment submission. A lost response must not trigger a fresh payment.
    let response = http
        .get(resource_url)
        .headers(payment.payment_signature_headers)
        .send()
        .await
        .map_err(|_| {
            anyhow!("Paid request failed. Reconcile with the resource before paying again.")
        })?;
    ensure!(
        response.status().is_success(),
        "Paid request did not succeed. Reconcile before paying again."
    );
    println!("Paid resource response: {}", response.status());
    let receipt_header = response
        .headers()
        .get("payment-response")
        .ok_or_else(|| anyhow!("Missing settlement receipt. Reconcile before paying again."))?
        .to_str()
        .map_err(|_| anyhow!("Invalid settlement receipt. Reconcile before paying again."))?;
    let receipt_bytes = STANDARD
        .decode(receipt_header)
        .map_err(|_| anyhow!("Invalid settlement receipt. Reconcile before paying again."))?;
    let receipt: SettlementReceipt = serde_json::from_slice(&receipt_bytes)
        .map_err(|_| anyhow!("Invalid settlement receipt. Reconcile before paying again."))?;
    ensure!(
        receipt.success && receipt.network == expected_network,
        "Settlement receipt does not confirm this network. Reconcile before paying again."
    );
    let signature: Signature = receipt
        .transaction
        .parse()
        .map_err(|_| anyhow!("Invalid settlement signature. Reconcile before paying again."))?;
    println!("Payment settled: {signature}");
    ensure!(rpc.confirm_transaction(&signature).await
        .map_err(|_| anyhow!("Settlement confirmation unavailable. Inspect the printed signature before rerunning."))?,
        "Settlement not confirmed. Inspect the printed signature before rerunning.");
    let resource = response.text().await.map_err(|_| {
        anyhow!("Payment settled but the resource body could not be read. Do not pay again.")
    })?;
    println!("Resource: {resource}");
    Ok(())
}

fn load_ed25519_signer() -> Result<Keypair> {
    let encoded = env::var("SWIG_ED25519_PRIVATE_KEY")
        .map_err(|_| anyhow!("Set SWIG_ED25519_PRIVATE_KEY to a hex-encoded 64-byte keypair"))?;
    let bytes = hex::decode(encoded.trim()).map_err(|_| anyhow!("Invalid Ed25519 key hex"))?;
    Keypair::try_from(bytes.as_slice()).map_err(|_| anyhow!("Invalid 64-byte Ed25519 keypair"))
}

async fn require_existing_wallet(
    rpc: &RpcClient,
    swig_config_address: Pubkey,
) -> Result<WalletAddressInfo> {
    let account = rpc
        .get_account(&swig_config_address)
        .await
        .map_err(|_| anyhow!("Could not load the Swig configuration from RPC"))?;
    ensure!(
        account.owner == SWIG_PROGRAM_ID,
        "Not a Swig configuration account"
    );
    let (wallet_address, _) = Pubkey::try_find_program_address(
        &[b"swig-wallet-address", swig_config_address.as_ref()],
        &SWIG_PROGRAM_ID,
    )
    .ok_or_else(|| anyhow!("Could not derive wallet address"))?;
    match env::var("SWIG_WALLET_ADDRESS") {
        Ok(value) => ensure!(
            value.parse::<Pubkey>()? == wallet_address,
            "Wallet address does not match config"
        ),
        Err(env::VarError::NotPresent) => {}
        Err(_) => bail!("SWIG_WALLET_ADDRESS must be valid text"),
    }
    Ok(WalletAddressInfo {
        swig_config_address,
        wallet_address,
    })
}
