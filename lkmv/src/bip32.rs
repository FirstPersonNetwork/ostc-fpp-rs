// ****************************************************************************
// BIP32 Handling
// ****************************************************************************

use crate::{KeyPurpose, errors::LKMVError};
use affinidi_tdk::secrets_resolver::{
    crypto::ed25519::ed25519_private_to_x25519_private_key, secrets::Secret,
};
use ed25519_dalek_bip32::{DerivationPath, ExtendedSigningKey};

/// Returns a BIP32 Master Key
pub fn get_bip32_root(seed: &[u8]) -> Result<ExtendedSigningKey, LKMVError> {
    ExtendedSigningKey::from_seed(seed)
        .map_err(|e| LKMVError::BIP32(format!("Couldn't create BIP32 Master Key from seed: {}", e)))
}

pub trait Bip32Extension {
    fn get_secret_from_path(&self, path: &str, kp: KeyPurpose) -> Result<Secret, LKMVError>;
}

impl Bip32Extension for ExtendedSigningKey {
    /// Generates an SSI Secret from a BIP32 root
    /// path: BIP32 derivation path
    /// kp: KeyPurpose (SIGN, ENC, AUTH)
    fn get_secret_from_path(&self, path: &str, kp: KeyPurpose) -> Result<Secret, LKMVError> {
        let key = self
            .derive(&path.parse::<DerivationPath>().map_err(|e| {
                LKMVError::BIP32(format!(
                    "Invalid path ({}) for BIP32 key deriviation: {}",
                    path, e
                ))
            })?)
            .map_err(|e| {
                LKMVError::BIP32(format!(
                    "Failed to create ed25519 key material from BIP32: {}",
                    e
                ))
            })?;

        let secret = match kp {
            KeyPurpose::Signing | KeyPurpose::Authentication => {
                Secret::generate_ed25519(None, Some(key.signing_key.as_bytes()))
            }
            KeyPurpose::Encryption => {
                let x25519_seed = ed25519_private_to_x25519_private_key(key.signing_key.as_bytes());
                Secret::generate_x25519(None, Some(&x25519_seed)).map_err(|e| {
                    LKMVError::Secret(format!("Failed to create derived encryption key: {}", e))
                })?
            }
            _ => {
                return Err(LKMVError::Secret(format!(
                    "Invalid key purpose used to generate key material ({})",
                    kp
                )));
            }
        };

        Ok(secret)
    }
}
