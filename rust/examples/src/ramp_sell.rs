//! Compile-only. Requires Mainnet SOL, a configured sandbox ramp, and an eligible customer.
//! Provider sandbox mode does not make Mainnet transfers free. Ramp owns deposit submission.

use std::env;

use anyhow::{anyhow, bail, ensure, Result};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_pubkey::{pubkey, Pubkey};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signature::Signature;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use swig_developer_sdk::{
    ClientConfig, CreateRampOrderArgs, CryptoAmount, CryptoAsset, GetRampOptionsArgs,
    GetRampQuotesArgs, Network, PaymasterClient, PrepareCreateWalletArgs, PrepareRampTransferArgs,
    RampClient, RampConfig, RampDirection, RampEnvironment, RampLocation, RampOrder,
    RampOrderContext, RampOrderRequest, RampOrderStatus, RampQuoteDetails, RampTransfer,
    RampTransferState, SponsorSignedTransactionArgs, SubmitRampTransferArgs, WalletAddressInfo,
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
        env::var("SWIG_NETWORK").as_deref() == Ok("mainnet"),
        "Ramp deposits require SWIG_NETWORK=mainnet"
    );
    let signer = load_ed25519_signer()?;
    let customer_id = env::var("SWIG_RAMP_CUSTOMER_ID")?;
    let configuration_id = env::var("SWIG_RAMP_CONFIGURATION_ID")?;
    let country_code = match env::var("SWIG_RAMP_COUNTRY") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => "US".into(),
        Err(_) => bail!("SWIG_RAMP_COUNTRY must be valid text"),
    };
    let currency_code = match env::var("SWIG_RAMP_CURRENCY") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => "USD".into(),
        Err(_) => bail!("SWIG_RAMP_CURRENCY must be valid text"),
    }
    .trim()
    .to_ascii_uppercase();
    let amount: u64 = match env::var("SWIG_AMOUNT_LAMPORTS") {
        Ok(value) => value.parse()?,
        Err(env::VarError::NotPresent) => 1_000_000,
        Err(_) => bail!("SWIG_AMOUNT_LAMPORTS must be valid text"),
    };
    ensure!(amount > 0, "Sell amount must be positive");
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
    let wallet_client = WalletClient::new(config.clone())?;
    let paymaster = PaymasterClient::new(config.clone())?;
    let ramp = RampClient::new(
        config,
        Some(RampConfig {
            configuration_id,
            environment: RampEnvironment::Sandbox,
        }),
    )?;
    let authority = WalletAuthority::Ed25519 {
        public_key: signer.pubkey(),
    };
    let wallet = match env::var("SWIG_CONFIG_ADDRESS") {
        Ok(value) => require_existing_wallet(&rpc, value.parse()?).await?,
        Err(env::VarError::NotPresent) => {
            ensure!(
                env::var_os("SWIG_WALLET_ADDRESS").is_none(),
                "SWIG_WALLET_ADDRESS requires SWIG_CONFIG_ADDRESS"
            );
            create_wallet(&wallet_client, &paymaster, &rpc, authority.clone()).await?
        }
        Err(_) => bail!("SWIG_CONFIG_ADDRESS must be valid text"),
    };
    println!("SWIG_CONFIG_ADDRESS={}", wallet.swig_config_address);
    println!("SWIG_WALLET_ADDRESS={}", wallet.wallet_address);
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
    ensure!(
        balance
            >= amount
                .checked_add(rent)
                .ok_or_else(|| anyhow!("Sell amount is too large"))?,
        "Fund this wallet with Mainnet SOL, then reuse its config. No airdrop is attempted."
    );

    let options = ramp
        .get_options(GetRampOptionsArgs {
            direction: RampDirection::Sell,
            country_code: Some(country_code.clone()),
            fiat_currency_code: Some(currency_code.clone()),
        })
        .await?;
    ensure!(
        options
            .assets
            .iter()
            .any(|option| option.asset == CryptoAsset::Sol),
        "SOL selling is unavailable"
    );
    let location = RampLocation {
        country_code,
        subdivision_code: None,
    };
    let order_request = RampOrderRequest::Sell {
        sell: CryptoAmount {
            asset: CryptoAsset::Sol,
            base_units: amount,
        },
        receive_fiat_currency_code: currency_code,
    };
    let quotes = ramp
        .get_quotes(GetRampQuotesArgs {
            location: location.clone(),
            order: order_request.clone(),
        })
        .await?;
    // This sandbox walkthrough selects the first eligible quote. A product would ask the user.
    let quote = quotes
        .into_iter()
        .find(|quote| matches!(quote.details, RampQuoteDetails::Sell(_)))
        .ok_or_else(|| anyhow!("No sell quote is available for this amount and location"))?;
    println!(
        "Route: {} / {}",
        quote.route.provider, quote.route.payment_method
    );
    let request_id = Keypair::new().pubkey().to_string();
    println!("Order request ID: {request_id}. Keep this ID if creation is uncertain.");
    let created = ramp.create_order(CreateRampOrderArgs {
        request_id: request_id.clone(),
        context: RampOrderContext { customer_id, swig_config_address: wallet.swig_config_address, location },
        route: quote.route,
        order: order_request,
    }).await.map_err(|error| anyhow!("Order creation failed or is uncertain for request {request_id}: {error}. Do not create a replacement blindly."))?;
    let RampOrder::Sell(created) = created else {
        bail!("Expected a sell order");
    };
    let order_id = created.id;
    println!("Order: {order_id}");
    let RampOrder::Sell(order) = ramp.get_order(&order_id).await? else {
        bail!("Expected a sell order");
    };
    ensure!(order.status == RampOrderStatus::AwaitingTransfer && order.transfer.is_none(),
        "Order {order_id} is not ready for a new deposit. Complete provider steps and inspect this order, not a replacement.");
    let deposit = order
        .deposit
        .ok_or_else(|| anyhow!("Order has no canonical deposit"))?;
    ensure!(
        deposit.amount.asset == CryptoAsset::Sol
            && deposit.amount.base_units > 0
            && deposit.amount.base_units <= amount,
        "Deposit exceeds the approved SOL amount"
    );
    ensure!(
        deposit.address != signer.pubkey()
            && deposit.address != wallet.wallet_address
            && deposit.address != wallet.swig_config_address,
        "Deposit destination must be independent of the signer and wallet"
    );

    let prepared = ramp.prepare_transfer(PrepareRampTransferArgs {
        order_id: order_id.clone(), requester_authority: authority, fee_payer: signer.pubkey(),
    }).await.map_err(|error| anyhow!("Preparation failed or is uncertain for order {order_id}: {error}. Read that order before another prepare."))?;
    ensure!(
        prepared.deposit.address == deposit.address
            && prepared.deposit.amount.asset == deposit.amount.asset
            && prepared.deposit.amount.base_units == deposit.amount.base_units,
        "Preparation changed the canonical deposit"
    );
    let transfer_id = prepared.transfer.transfer_id;
    println!("Transfer: {transfer_id}");
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
        "Expected an unsigned Ed25519 deposit for this wallet"
    );
    let message = transaction.transaction.message;
    ensure!(
        message.header().num_required_signatures == 1
            && message.static_account_keys().first() == Some(&signer.pubkey()),
        "Expected the Ed25519 signer to pay the deposit fee"
    );
    let solana_message::VersionedMessage::Legacy(legacy_message) = &message else {
        bail!("Ramp requires a legacy transaction");
    };
    let fee = rpc
        .get_fee_for_message(legacy_message)
        .await
        .map_err(|_| anyhow!("Could not fetch the deposit fee from RPC"))?;
    ensure!(
        rpc.get_balance(&signer.pubkey())
            .await
            .map_err(|_| anyhow!("Could not read the signer balance from RPC"))?
            >= fee,
        "Fund the Ed25519 signer for the deposit fee"
    );
    // Metadata checks do not audit instructions. This example trusts backend deposit construction.
    let signed = VersionedTransaction::try_new(message, &[&signer])?;
    let attempt = SubmitRampTransferArgs {
        order_id: order_id.clone(),
        transfer_id: transfer_id.clone(),
        signed_transaction: Some(signed),
    };
    // Keep exactly this attempt in memory while resolving uncertainty. Never rebuild or resubmit here.
    let result = match ramp.submit_transfer(attempt.clone()).await {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "Submission uncertain for transfer {transfer_id}: {error}. Reading the same order."
            );
            let RampOrder::Sell(current) = ramp.get_order(&attempt.order_id).await? else {
                bail!("Expected the same sell order");
            };
            let current = current.transfer.ok_or_else(|| {
                anyhow!("No transfer outcome for {transfer_id}. Reconcile before any retry.")
            })?;
            ensure!(
                current.transfer_id == attempt.transfer_id,
                "The order references another attempt. Do not resubmit."
            );
            current
        }
    };
    let signature = submission_signature(&result)?;
    println!("Deposit submitted: {signature}");
    rpc.poll_for_signature(&signature).await.map_err(|_| {
        anyhow!("Confirmation unknown for transfer {transfer_id}, signature {signature}. Reconcile the same attempt before retrying.")
    })?;
    ensure!(
        rpc.confirm_transaction(&signature).await.map_err(|_| {
            anyhow!("Confirmation unknown for transfer {transfer_id}, signature {signature}. Reconcile the same attempt before retrying.")
        })?,
        "Deposit did not confirm successfully: {signature}. Reconcile transfer {transfer_id}."
    );
    println!("Deposit confirmed: {signature}");
    println!("Read order {order_id} separately for fiat settlement. Deposit acceptance is not settlement.");
    Ok(())
}

fn submission_signature(transfer: &RampTransfer) -> Result<Signature> {
    match transfer.state {
        RampTransferState::Submitted { signature } | RampTransferState::Landed { signature } => {
            Ok(signature)
        }
        RampTransferState::Prepared
        | RampTransferState::Expired
        | RampTransferState::Failed { .. } => {
            bail!(
                "Transfer {} has no successful submission outcome. Reconcile it before retrying.",
                transfer.transfer_id
            );
        }
    }
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
    let transaction = prepared.transaction;
    ensure!(
        prepared.signature_requests.is_empty()
            && prepared.participant_set_approval_plan.is_none()
            && transaction
                .signatures
                .iter()
                .all(|value| *value == Signature::default())
            && transaction.message.header().num_required_signatures == 1
            && transaction.message.static_account_keys().first() == Some(&fee_payer),
        "Expected only the API paymaster to sign creation"
    );
    let receipt = paymaster
        .sponsor(SponsorSignedTransactionArgs {
            transaction,
            idempotency_key: None,
        })
        .await?;
    println!("Creation submitted: {}", receipt.signature);
    rpc.poll_for_signature(&receipt.signature)
        .await
        .map_err(|_| {
            anyhow!("Creation confirmation unknown. Inspect its signature before rerunning.")
        })?;
    ensure!(
        rpc.confirm_transaction(&receipt.signature)
            .await
            .map_err(|_| {
                anyhow!(
                    "Creation confirmation unknown for {}. Inspect it before rerunning.",
                    receipt.signature
                )
            })?,
        "Creation did not confirm successfully"
    );
    Ok(creation.wallet)
}
