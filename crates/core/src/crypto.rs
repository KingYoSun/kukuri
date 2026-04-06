use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use bech32::{Bech32, Hrp};
use hkdf::Hkdf;
use secp256k1::ecdh::SharedSecret;
use secp256k1::rand::rng;
use secp256k1::schnorr::Signature;
use secp256k1::{Keypair, Parity, PublicKey, SECP256K1, SecretKey, XOnlyPublicKey};
use sha2::{Digest, Sha256};

use crate::Pubkey;

pub const LEGACY_SECRET_HRP: &str = "nsec";

#[derive(Clone)]
pub struct KukuriKeys {
    secret_key: SecretKey,
}

impl std::fmt::Debug for KukuriKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KukuriKeys").finish_non_exhaustive()
    }
}

impl KukuriKeys {
    pub fn generate() -> Self {
        Self {
            secret_key: SecretKey::new(&mut rng()),
        }
    }

    pub fn parse(secret: &str) -> Result<Self> {
        let secret_key = parse_secret_key(secret)?;
        Ok(Self { secret_key })
    }

    pub fn public_key_hex(&self) -> String {
        let keypair = Keypair::from_secret_key(SECP256K1, &self.secret_key);
        let (pubkey, _) = keypair.x_only_public_key();
        pubkey.to_string()
    }

    pub fn public_key(&self) -> Pubkey {
        Pubkey(self.public_key_hex())
    }

    pub fn export_secret_hex(&self) -> String {
        hex::encode(self.secret_key.secret_bytes())
    }

    pub fn sign_schnorr(&self, message: &[u8]) -> Signature {
        let keypair = Keypair::from_secret_key(SECP256K1, &self.secret_key);
        SECP256K1.sign_schnorr(message, &keypair)
    }
}

fn parse_secret_key(secret: &str) -> Result<SecretKey> {
    let trimmed = secret.trim();
    if let Ok(bytes) = hex::decode(trimmed)
        && bytes.len() == 32
    {
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow!("invalid hex secret key length"))?;
        return SecretKey::from_byte_array(bytes).context("invalid hex secret key");
    }

    let (hrp, bytes) = bech32::decode(trimmed).context("failed to decode secret key")?;
    if hrp.as_str() != LEGACY_SECRET_HRP {
        bail!("unsupported secret key hrp `{}`", hrp.as_str());
    }
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("invalid bech32 secret key length"))?;
    SecretKey::from_byte_array(bytes).context("invalid bech32 secret key")
}

pub fn encode_secret_key_bech32(secret_key_hex: &str, hrp: &str) -> Result<String> {
    let bytes = hex::decode(secret_key_hex).context("invalid hex secret key")?;
    if bytes.len() != 32 {
        bail!("invalid secret key length");
    }
    bech32::encode::<Bech32>(
        Hrp::parse(hrp).context("invalid secret key hrp")?,
        bytes.as_slice(),
    )
    .context("failed to encode secret key")
}

pub fn generate_keys() -> KukuriKeys {
    KukuriKeys::generate()
}

pub(crate) fn sha256_digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub(crate) fn pairwise_shared_secret(
    local_keys: &KukuriKeys,
    remote_pubkey: &Pubkey,
) -> Result<SharedSecret> {
    let remote_xonly = XOnlyPublicKey::from_str(remote_pubkey.as_str())
        .context("invalid remote x-only public key")?;
    let remote_public = PublicKey::from_x_only_public_key(remote_xonly, Parity::Even);
    let keypair = Keypair::from_secret_key(SECP256K1, &local_keys.secret_key);
    let (_, parity) = keypair.x_only_public_key();
    let local_secret = if parity == Parity::Odd {
        local_keys.secret_key.negate()
    } else {
        local_keys.secret_key
    };
    Ok(SharedSecret::new(&remote_public, &local_secret))
}

pub(crate) fn validate_pubkey(value: &str) -> Result<()> {
    XOnlyPublicKey::from_str(value).context("invalid x-only public key")?;
    Ok(())
}

pub(crate) fn now_timestamp_millis() -> Result<i64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before unix epoch")?
        .as_millis() as i64)
}

pub(crate) fn derive_hkdf_key(
    salt: &[u8],
    ikm: &[u8],
    info: &[u8],
    label: &str,
) -> Result<[u8; 32]> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut key = [0u8; 32];
    hkdf.expand(info, &mut key)
        .map_err(|_| anyhow!("failed to derive {label}"))?;
    Ok(key)
}
