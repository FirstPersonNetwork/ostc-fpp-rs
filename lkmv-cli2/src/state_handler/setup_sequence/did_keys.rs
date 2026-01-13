use anyhow::Result;
use bip39::Mnemonic;
use chrono::Utc;
use lkmv::{
    KeyPurpose,
    bip32::{Bip32Extension, get_bip32_root},
    config::{KeyInfo, PersonaDIDKeys, secured_config::KeySourceMaterial},
};

use crate::ui::pages::setup_flow::did_keys_ask::DIDKeysAsk;

impl DIDKeysAsk {
    pub fn create_keys(mnemonic: &Mnemonic) -> Result<PersonaDIDKeys> {
        let bip32_root = get_bip32_root(mnemonic.to_entropy().as_slice())?;

        let created = Utc::now();

        // Signing key
        let mut sign_secret = bip32_root.get_secret_from_path("m/1'/0'/0'", KeyPurpose::Signing)?;
        sign_secret.id = sign_secret.get_public_keymultibase()?;

        let signing = KeyInfo {
            secret: sign_secret,
            source: KeySourceMaterial::Derived {
                path: "m/1'/0'/0'".to_string(),
            },
            expiry: None,
            created,
        };

        // Authentication Key
        let mut auth_secret =
            bip32_root.get_secret_from_path("m/1'/0'/1'", KeyPurpose::Authentication)?;
        auth_secret.id = auth_secret.get_public_keymultibase()?;

        let authentication = KeyInfo {
            secret: auth_secret,
            source: KeySourceMaterial::Derived {
                path: "m/1'/0'/1'".to_string(),
            },
            expiry: None,
            created,
        };

        // Encrypt/Decrypt Key
        let mut dec_secret =
            bip32_root.get_secret_from_path("m/1'/0'/2'", KeyPurpose::Encryption)?;
        dec_secret.id = dec_secret.get_public_keymultibase()?;

        let decryption = KeyInfo {
            secret: dec_secret,
            source: KeySourceMaterial::Derived {
                path: "m/1'/0'/2'".to_string(),
            },
            expiry: None,
            created,
        };

        Ok(PersonaDIDKeys {
            signing,
            authentication,
            decryption,
        })
    }
}
