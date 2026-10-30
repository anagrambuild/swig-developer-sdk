//! Devnet SOL transfer. Run with `cargo run --example secp256r1_transfer`.
//! Uses a local raw secp256r1 key, not a browser passkey or WebAuthn ceremony.

use std::{env, sync::Arc};

use anyhow::{anyhow, bail, ensure, Result};
use p256::ecdsa::SigningKey;
use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_pubkey::{pubkey, Pubkey};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signature::Signature;
use solana_signer::Signer;
use swig_developer_sdk::{
    ClientConfig, Network, PaymasterClient, PrepareCreateWalletArgs, PrepareTransferSolArgs,
    Secp256r1TransactionSigner, SponsorSignedTransactionArgs, TransactionSigner, WalletAddressInfo,
    WalletAuthority, WalletClient,
};

const SWIG_PROGRAM_ID: Pubkey = pubkey!("swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB");

pub async fn example() -> Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(error) if error.not_found() => {}
        Err(_) => bail!("Could not load .env. Check its syntax and permissions."),
    }
    ensure!(
        env_or_default("SWIG_NETWORK", "devnet")? == "devnet",
        "This example is for local/Devnet use only"
    );

    let (authority, signer) = load_secp256r1_signer()?;
    let destination: Pubkey = match env::var("SWIG_DESTINATION") {
        Ok(value) => value.parse()?,
        Err(env::VarError::NotPresent) => {
            let address = Keypair::new().pubkey();
            println!("Temporary Devnet recipient: {address}. Its private key is not saved.");
            address
        }
        Err(_) => bail!("SWIG_DESTINATION must be valid text"),
    };
    let amount: u64 = env_or_default("SWIG_AMOUNT_LAMPORTS", "1000000")?.parse()?;
    ensure!(amount > 0, "The transfer amount must be positive");

    let mut config = ClientConfig::new(
        env::var("SWIG_API_KEY").map_err(|_| anyhow!("Set SWIG_API_KEY"))?,
        Network::Devnet,
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
    let client = WalletClient::new(config.clone())?;
    let paymaster = PaymasterClient::new(config)?;

    let wallet = match env::var("SWIG_CONFIG_ADDRESS") {
        Ok(value) => require_existing_wallet(&rpc, value.parse()?).await?,
        Err(env::VarError::NotPresent) => {
            ensure!(
                env::var_os("SWIG_WALLET_ADDRESS").is_none(),
                "SWIG_WALLET_ADDRESS requires SWIG_CONFIG_ADDRESS"
            );
            create_wallet(&client, &paymaster, &rpc, authority.clone()).await?
        }
        Err(_) => bail!("SWIG_CONFIG_ADDRESS must be valid text"),
    };
    println!("SWIG_CONFIG_ADDRESS={}", wallet.swig_config_address);
    println!("SWIG_WALLET_ADDRESS={}", wallet.wallet_address);
    ensure!(
        destination != wallet.swig_config_address && destination != wallet.wallet_address,
        "The recipient must be outside this wallet"
    );

    let account = rpc
        .get_account_with_commitment(&wallet.wallet_address, CommitmentConfig::confirmed())
        .await
        .map_err(|_| anyhow!("Could not read the wallet account from RPC"))?
        .value;
    let (balance, data_len) = match account {
        Some(account) => (account.lamports, account.data.len()),
        None => (0, 0),
    };
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(data_len)
        .await
        .map_err(|_| anyhow!("Could not fetch the rent requirement from RPC"))?;
    let required_balance = amount
        .checked_add(rent)
        .ok_or_else(|| anyhow!("Transfer amount is too large"))?;
    if balance < required_balance {
        let signature = rpc
            .request_airdrop(&wallet.wallet_address, 2_000_000_000)
            .await
            .map_err(|_| anyhow!("Could not request the 2 SOL airdrop from RPC"))?;
        println!("2 SOL airdrop submitted: {signature}");
        confirm_transaction(&rpc, &signature).await?;
        ensure!(
            rpc.get_balance(&wallet.wallet_address)
                .await
                .map_err(|_| anyhow!("Could not read the wallet balance from RPC"))?
                >= required_balance,
            "The wallet still needs funding for this transfer and its rent reserve"
        );
    }

    let fee_payer = paymaster.get_balance().await?.address;
    let prepared = client
        .prepare_transfer_sol(PrepareTransferSolArgs {
            swig_config_address: wallet.swig_config_address,
            fee_payer,
            requester_authority: authority,
            destination,
            amount,
        })
        .await?;
    ensure!(
        !prepared.signature_requests.is_empty() && prepared.participant_set_approval_plan.is_none(),
        "Expected a standalone secp256r1 signature request"
    );
    ensure!(
        prepared
            .transaction
            .message
            .header()
            .num_required_signatures
            == 1
            && prepared.transaction.message.static_account_keys().first() == Some(&fee_payer),
        "Expected only the API paymaster as an outer transaction signer"
    );
    let transaction = signer.sign_prepared_transaction(prepared).await?;
    let receipt = paymaster
        .sponsor(SponsorSignedTransactionArgs {
            transaction,
            idempotency_key: None,
        })
        .await?;
    println!("Transfer submitted: {}", receipt.signature);
    confirm_transaction(&rpc, &receipt.signature).await?;
    println!("Transferred {amount} lamports to {destination}");
    println!(
        "Wallet balance: {} lamports",
        rpc.get_balance(&wallet.wallet_address)
            .await
            .map_err(|_| anyhow!("Could not read the wallet balance from RPC"))?
    );
    Ok(())
}

fn load_secp256r1_signer() -> Result<(WalletAuthority, Secp256r1TransactionSigner)> {
    let encoded = env::var("SWIG_SECP256R1_PRIVATE_KEY")
        .map_err(|_| anyhow!("Set SWIG_SECP256R1_PRIVATE_KEY to a 32-byte secret in hex"))?;
    let secret: [u8; 32] = hex::decode(encoded.trim())
        .map_err(|_| anyhow!("SWIG_SECP256R1_PRIVATE_KEY must be hexadecimal"))?
        .try_into()
        .map_err(|_| anyhow!("SWIG_SECP256R1_PRIVATE_KEY must contain exactly 32 bytes"))?;
    let key = SigningKey::from_slice(&secret)
        .map_err(|_| anyhow!("SWIG_SECP256R1_PRIVATE_KEY is not a valid secp256r1 secret"))?;
    let public_key = key.verifying_key().to_encoded_point(true);
    let authority = WalletAuthority::Secp256r1 {
        public_key: public_key.as_bytes().try_into()?,
    };
    println!(
        "secp256r1 public key: {}",
        hex::encode(public_key.as_bytes())
    );
    Ok((authority, Secp256r1TransactionSigner::new(Arc::new(key))))
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
    .ok_or_else(|| anyhow!("Could not derive the wallet address"))?;
    match env::var("SWIG_WALLET_ADDRESS") {
        Ok(value) => ensure!(
            value.parse::<Pubkey>()? == wallet_address,
            "SWIG_WALLET_ADDRESS does not match SWIG_CONFIG_ADDRESS"
        ),
        Err(env::VarError::NotPresent) => {}
        Err(_) => bail!("SWIG_WALLET_ADDRESS must be valid text"),
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

fn env_or_default(name: &str, default: &str) -> Result<String> {
    match env::var(name) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(default.to_owned()),
        Err(_) => bail!("{name} must be valid text"),
    }
}
