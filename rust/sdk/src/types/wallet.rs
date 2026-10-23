use solana_pubkey::Pubkey;

#[derive(Clone)]
pub struct WalletAddressInfo {
    pub swig_config_address: Pubkey,
    pub wallet_address: Pubkey,
}
