//! Build a Memo instruction locally, execute it through Swig, and sponsor the resulting transaction.
//!
//! This file is compiled, not run by Cargo tests. API and RPC must use the same chain.

use std::env;

use anyhow::{anyhow, bail, ensure, Result};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_pubkey::{pubkey, Pubkey};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signature::Signature;
use solana_signer::{null_signer::NullSigner, Signer};
use solana_transaction::versioned::VersionedTransaction;
use swig_developer_sdk::{
    ClientConfig, Instruction, Network, PaymasterClient, PrepareCreateWalletArgs,
    PrepareTransactionArgs, PreparedTransaction, SponsorSignedTransactionArgs, WalletAddressInfo,
    WalletAuthority, WalletClient,
};

const SWIG_PROGRAM_ID: Pubkey = pubkey!("swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB");

pub async fn example() -> Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(error) if error.not_found() => {}
        Err(_) => bail!("Could not load .env. Check its syntax and permissions."),
    }
    let network = match env_or_default("SWIG_NETWORK", "devnet")?.as_str() {
        "devnet" => Network::Devnet,
        "mainnet" => Network::Mainnet,
        _ => bail!("SWIG_NETWORK must be devnet or mainnet"),
    };
    let signer = load_ed25519_signer()?;
    let config_address = optional_pubkey("SWIG_CONFIG_ADDRESS")?;
    let wallet_address = optional_pubkey("SWIG_WALLET_ADDRESS")?;
    ensure!(
        config_address.is_some() || wallet_address.is_none(),
        "SWIG_WALLET_ADDRESS requires SWIG_CONFIG_ADDRESS"
    );
    let mut config = ClientConfig::new(
        env::var("SWIG_API_KEY").map_err(|_| anyhow!("Set SWIG_API_KEY"))?,
        network,
    );
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
    let wallet_client = WalletClient::new(config.clone())?;
    let paymaster = PaymasterClient::new(config)?;
    let authority = WalletAuthority::Ed25519 {
        public_key: signer.pubkey(),
    };
    let wallet = match config_address {
        Some(address) => require_existing_wallet(&rpc, address, wallet_address).await?,
        None => create_wallet(&wallet_client, &paymaster, &rpc, authority.clone()).await?,
    };
    let fee_payer = paymaster.get_balance().await?.address;
    ensure!(fee_payer != signer.pubkey(), "Use a separate API paymaster");
    println!("Signer: {}", signer.pubkey());
    println!("SWIG_CONFIG_ADDRESS={}", wallet.swig_config_address);
    println!("SWIG_WALLET_ADDRESS={}", wallet.wallet_address);

    let memo = Instruction {
        program_id: pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"),
        accounts: vec![],
        data: b"Swig Rust SDK custom transaction".to_vec(),
    };
    let prepared = wallet_client
        .prepare_transaction(PrepareTransactionArgs {
            swig_config_address: wallet.swig_config_address,
            fee_payer,
            requester_authority: authority,
            instructions: vec![memo],
            address_lookup_table_accounts: vec![],
        })
        .await?;
    let transaction = sign_prepared_transaction(prepared, &signer, fee_payer)?;
    let receipt = paymaster
        .sponsor(SponsorSignedTransactionArgs {
            transaction,
            idempotency_key: None,
        })
        .await?;
    println!("Transaction submitted: {}", receipt.signature);
    confirm_transaction(&rpc, &receipt.signature).await?;
    println!("Custom Memo transaction confirmed");
    Ok(())
}

fn load_ed25519_signer() -> Result<Keypair> {
    let encoded = env::var("SWIG_ED25519_PRIVATE_KEY")
        .map_err(|_| anyhow!("Set SWIG_ED25519_PRIVATE_KEY to a hex-encoded 64-byte keypair"))?;
    let bytes = hex::decode(encoded.trim())
        .map_err(|_| anyhow!("SWIG_ED25519_PRIVATE_KEY must be hexadecimal"))?;
    Keypair::try_from(bytes.as_slice())
        .map_err(|_| anyhow!("SWIG_ED25519_PRIVATE_KEY must contain a valid 64-byte keypair"))
}

async fn require_existing_wallet(
    rpc: &RpcClient,
    swig_config_address: Pubkey,
    supplied_wallet_address: Option<Pubkey>,
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
    .ok_or_else(|| anyhow!("Could not derive the wallet address"))?;
    if let Some(supplied) = supplied_wallet_address {
        ensure!(
            supplied == wallet_address,
            "SWIG_WALLET_ADDRESS does not match SWIG_CONFIG_ADDRESS"
        );
    }
    Ok(WalletAddressInfo {
        swig_config_address,
        wallet_address,
    })
}

async fn create_wallet(
    client: &WalletClient,
    paymaster: &PaymasterClient,
    rpc: &RpcClient,
    authority: WalletAuthority,
) -> Result<WalletAddressInfo> {
    let fee_payer = paymaster.get_balance().await?.address;
    let creation = client
        .prepare_create_wallet(PrepareCreateWalletArgs {
            fee_payer,
            policy_id: None,
            initial_user: Some(authority),
        })
        .await?;
    println!(
        "New SWIG_CONFIG_ADDRESS={}",
        creation.wallet.swig_config_address
    );
    println!("New SWIG_WALLET_ADDRESS={}", creation.wallet.wallet_address);
    let prepared = creation.transaction;
    ensure!(
        prepared.signature_requests.is_empty() && prepared.participant_set_approval_plan.is_none(),
        "Unexpected creation signing requirements"
    );
    let transaction = prepared.transaction;
    ensure!(
        transaction
            .signatures
            .iter()
            .all(|signature| *signature == Signature::default())
            && transaction.message.header().num_required_signatures == 1
            && transaction.message.static_account_keys().first() == Some(&fee_payer),
        "This creation flow expects only the API paymaster to sign"
    );
    let receipt = paymaster
        .sponsor(SponsorSignedTransactionArgs {
            transaction,
            idempotency_key: None,
        })
        .await?;
    println!("Creation submitted: {}", receipt.signature);
    confirm_transaction(rpc, &receipt.signature).await?;
    Ok(creation.wallet)
}

fn sign_prepared_transaction(
    prepared: PreparedTransaction,
    signer: &Keypair,
    fee_payer: Pubkey,
) -> Result<VersionedTransaction> {
    ensure!(
        prepared.signature_requests.is_empty() && prepared.participant_set_approval_plan.is_none(),
        "Unexpected additional signing requirements"
    );
    ensure!(
        prepared
            .transaction
            .signatures
            .iter()
            .all(|signature| *signature == Signature::default()),
        "Expected an unsigned transaction"
    );
    let message = prepared.transaction.message;
    ensure!(
        message.header().num_required_signatures == 2
            && message.static_account_keys().first() == Some(&fee_payer)
            && message.static_account_keys().get(1) == Some(&signer.pubkey()),
        "Expected the API paymaster and requester as transaction signers"
    );
    let unsigned_paymaster = NullSigner::new(&fee_payer);
    let signers: [&dyn Signer; 2] = [&unsigned_paymaster, signer];
    Ok(VersionedTransaction::try_new(message, &signers)?)
}

async fn confirm_transaction(rpc: &RpcClient, signature: &Signature) -> Result<()> {
    rpc.poll_for_signature(signature).await.map_err(|_| {
        anyhow!("Confirmation unknown for {signature}. Inspect it before rerunning.")
    })?;
    ensure!(
        rpc.confirm_transaction(signature).await.map_err(|_| {
            anyhow!("Confirmation unknown for {signature}. Inspect it before rerunning.")
        })?,
        "Transaction not confirmed successfully: {signature}. Inspect it before rerunning."
    );
    Ok(())
}

fn optional_pubkey(name: &str) -> Result<Option<Pubkey>> {
    match env::var(name) {
        Ok(value) => value
            .parse()
            .map(Some)
            .map_err(|_| anyhow!("{name} must be a public key")),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(_) => bail!("{name} must be valid text"),
    }
}

fn env_or_default(name: &str, default: &str) -> Result<String> {
    match env::var(name) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(default.to_owned()),
        Err(_) => bail!("{name} must be valid text"),
    }
}
