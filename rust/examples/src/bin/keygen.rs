//! Prints throwaway private keys as environment variables. Keep the output private.

use std::{
    env,
    io::{self, Write},
};

use anyhow::{ensure, Result};
use rand_core::OsRng;
use solana_keypair::Keypair;

fn main() -> Result<()> {
    let arguments: Vec<_> = env::args_os().skip(1).collect();
    if matches!(arguments.as_slice(), [argument] if argument == "--help" || argument == "-h") {
        writeln!(io::stdout().lock(), "keygen\nPrints three throwaway private keys as environment-variable assignments.\nNo files are written. Keep this output private and never use these keys with production funds.")?;
        return Ok(());
    }
    ensure!(arguments.is_empty(), "Unexpected arguments. Use --help.");

    let ed25519 = Keypair::new();
    let secp256r1 = p256::ecdsa::SigningKey::random(&mut OsRng);
    let secp256k1 = k256::ecdsa::SigningKey::random(&mut OsRng);
    writeln!(
        io::stdout().lock(),
        "SWIG_ED25519_PRIVATE_KEY={}\nSWIG_SECP256K1_PRIVATE_KEY={}\nSWIG_SECP256R1_PRIVATE_KEY={}",
        hex::encode(ed25519.to_bytes()),
        hex::encode(secp256k1.to_bytes()),
        hex::encode(secp256r1.to_bytes()),
    )?;
    Ok(())
}
