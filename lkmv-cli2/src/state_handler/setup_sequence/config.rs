/*! Contains specific Config extensions for the CLI Application. */

use anyhow::{Result, bail};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use console::style;
use ed25519_dalek_bip32::ExtendedSigningKey;
use lkmv::config::{
    Config, ConfigProtectionType, ExportedConfig, protected_config::ProtectedConfig,
    public_config::PublicConfig, secured_config::unlock_code_decrypt,
};
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};
use std::fs;
use tokio::sync::mpsc::UnboundedSender;

use crate::state_handler::{setup_sequence::MessageType, state::State};

pub trait ConfigExtension {
    /// Imports a backup of lkmv configuration settings from an encrypted file
    /// state: LKMV backend state
    /// state_tx: State update channel transmitter
    /// import_unlock_passphrase: Passphrase used to decrypt the imported configuration
    /// new_unlock_passphrase: New passphrase to protect the imported configuration
    /// file: Path to the file containing the exported configuration
    /// profile: Profile name to import the configuration into
    fn import(
        state: &mut State,
        state_tx: &UnboundedSender<State>,
        import_unlock_passphrase: &SecretString,
        new_unlock_passphrase: &SecretString,
        file: &str,
        profile: &str,
    ) -> Result<()>;
}

impl ConfigExtension for Config {
    /// Import previously exported configuration settings from an encrypted file
    fn import(
        state: &mut State,
        state_tx: &UnboundedSender<State>,
        import_unlock_passphrase: &SecretString,
        new_unlock_passphrase: &SecretString,
        file: &str,
        profile: &str,
    ) -> Result<()> {
        let content = match fs::read_to_string(file) {
            Ok(content) => content,
            Err(e) => {
                state
                    .setup
                    .config_import
                    .messages
                    .push(MessageType::Error(format!(
                        "Couldn't read from file ({file}). Reason: {e}"
                    )));
                let _ = state_tx.send(state.clone());
                bail!("File read error");
            }
        };

        let decoded = match BASE64_URL_SAFE_NO_PAD.decode(content) {
            Ok(decoded) => decoded,
            Err(e) => {
                state
                    .setup
                    .config_import
                    .messages
                    .push(MessageType::Error(format!(
                        "Couldn't base64 decode file content. Reason: {e}"
                    )));
                let _ = state_tx.send(state.clone());
                bail!("base64 decoding error");
            }
        };

        let seed_bytes = Sha256::digest(import_unlock_passphrase.expose_secret())
            .first_chunk::<32>()
            .expect("Couldn't get 32 bytes for passphrase hash")
            .to_owned();

        let decoded = unlock_code_decrypt(&seed_bytes, &decoded)?;

        let config: ExportedConfig = match serde_json::from_slice(&decoded) {
            Ok(config) => config,
            Err(e) => {
                state
                    .setup
                    .config_import
                    .messages
                    .push(MessageType::Error(format!(
                        "Couldn't deserialize configuration settings. Reason: {e}"
                    )));
                let _ = state_tx.send(state.clone());
                bail!("deserialization error");
            }
        };

        let bip32_root = ExtendedSigningKey::from_seed(
            BASE64_URL_SAFE_NO_PAD
                .decode(&config.sc.bip32_seed)
                .expect("Couldn't base64 decode BIP32 seed")
                .as_slice(),
        )?;
        let private_seed = ProtectedConfig::get_seed(&bip32_root, "m/0'/0'/0'")?;

        let private = if let Some(private) = &config.pc.private {
            ProtectedConfig::load(&private_seed, private)?
        } else {
            ProtectedConfig::default()
        };

        config
            .pc
            .save(profile, &private, &private_seed)
            .expect("Couldn't save Public Config");

        #[cfg(feature = "openpgp-card")]
        {
            let state_clone = state.clone();
            let state_tx_clone = state_tx.clone();
            config
                .sc
                .save(
                    profile,
                    if let ConfigProtectionType::Token(token) = &config.pc.protection {
                        Some(token)
                    } else {
                        None
                    },
                    Some(
                        &sha2::Sha256::digest(import_unlock_passphrase.expose_secret().as_bytes())
                            .to_vec(),
                    ),
                    &move || {
                        let mut state_mut = state_clone.clone();
                        state_mut
                            .setup
                            .config_import
                            .messages
                            .push(MessageType::Info(
                                "Please touch token hardware to unlock keys".to_string(),
                            ));
                        let _ = state_tx_clone.send(state_mut);
                    },
                )
                .expect("Couldn't save Secured Config");
        }

        #[cfg(not(feature = "openpgp-card"))]
        config
            .sc
            .save(
                profile,
                if let ConfigProtectionType::Token(token) = &config.pc.protection {
                    Some(token)
                } else {
                    None
                },
                Some(&import_unlock_passphrase.expose_secret().as_bytes().to_vec()),
            )
            .expect("Couldn't save Secured Config");

        Ok(())
    }
}
