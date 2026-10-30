//! Reads an existing wallet. This example never creates, signs, or submits transactions.

use std::env;

use anyhow::{bail, ensure, Result};
use solana_pubkey::Pubkey;
use swig_developer_sdk::{ClientConfig, ListSwigTokenTransactionsArgs, Network, WalletClient};

pub async fn example(policy_id: Option<String>) -> Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(error) if error.not_found() => {}
        Err(_) => bail!("Could not load .env. Check its syntax and permissions."),
    }
    let swig_config_address: Pubkey = env::var("SWIG_CONFIG_ADDRESS")?.parse()?;
    let supplied_wallet_address = match env::var("SWIG_WALLET_ADDRESS") {
        Ok(address) => Some(address.parse::<Pubkey>()?),
        Err(env::VarError::NotPresent) => None,
        Err(error) => return Err(error.into()),
    };
    let network = match env::var("SWIG_NETWORK").as_deref() {
        Ok("devnet") | Err(env::VarError::NotPresent) => Network::Devnet,
        Ok("mainnet") => Network::Mainnet,
        _ => bail!("SWIG_NETWORK must be devnet or mainnet"),
    };
    let mut config = ClientConfig::new(env::var("SWIG_API_KEY")?, network);
    config.http.base_url = match env::var("SWIG_API_URL") {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(error) => return Err(error.into()),
    };
    let wallet = WalletClient::new(config)?;
    let roles = wallet.list_roles(&swig_config_address).await?;
    if let Some(address) = supplied_wallet_address {
        ensure!(
            address == roles.wallet_address,
            "SWIG_WALLET_ADDRESS does not match the existing wallet"
        );
    }
    println!("Config: {}", roles.swig_config_address);
    println!("Wallet: {}", roles.wallet_address);
    for role in roles.roles {
        println!("Role {}: {} permissions", role.role_id, role.actions.len());
    }
    if let Some(id) = policy_id {
        let policy = wallet.get_policy(&id).await?;
        println!("Policy: {} ({})", policy.name, policy.id);
        println!("Policy permissions: {}", policy.actions.len());
    }
    let usd = wallet.get_usd_balance(&swig_config_address).await?;
    let balances = wallet.list_token_balances(&swig_config_address).await?;
    println!("USD value: {}", usd.usd_value);
    if let Some(sol) = balances.sol_balance {
        println!("SOL: {} lamports", sol.lamports);
    }
    for (mint, token) in balances.token_balances {
        println!("{mint}: {} {}", token.ui_amount, token.symbol);
    }
    let history = wallet
        .list_token_transactions(ListSwigTokenTransactionsArgs {
            swig_config_address,
            limit: Some(10),
        })
        .await?;
    for transaction in history.transactions {
        println!(
            "{} at slot {}",
            transaction.transaction_signature, transaction.slot
        );
    }
    Ok(())
}
