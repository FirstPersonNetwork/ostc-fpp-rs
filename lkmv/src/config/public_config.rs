/*!
*  Public [crate::config::Config] information that is stored in plaintext on disk
*/

use crate::{
    config::{Config, private_config::PrivateConfig},
    errors::LKMVError,
    logs::Logs,
};
use secrecy::SecretVec;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::Path, rc::Rc};
use tracing::warn;

/// Primary structure used for storing [crate::config::Config] data that is not sensitive
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PublicConfig {
    /// Identifier for a hardware token if being used
    pub token_id: Option<String>,

    /// Use an unlock code if hardware token isn't being used?
    /// true: Will take a unlock code --> Sha256 hash --> decrypt key
    /// false: Plain text retrieval from the OS Secure Store
    pub unlock_code: bool,

    /// Persona DID
    pub persona_did: Rc<String>,

    /// Mediator DID
    pub mediator_did: String,

    /// Human friendly name to use when referring to ourself
    pub friendly_name: String,

    /// Linux Organisation DID
    pub lk_did: String,

    #[serde(default)]
    pub logs: Logs,

    #[serde(default)]
    pub private: Option<String>,
}

impl From<&Config> for PublicConfig {
    /// Extracts public information from the full Config
    fn from(cfg: &Config) -> Self {
        cfg.public.clone()
    }
}

/// Private helper to determine where the config file is located
fn get_config_path(profile: &str) -> Result<String, LKMVError> {
    let path = if let Ok(config_path) = env::var("LKMV_CONFIG_PATH") {
        if config_path.ends_with('/') {
            config_path
        } else {
            [&config_path, "/"].concat()
        }
    } else if let Some(home) = dirs::home_dir()
        && let Some(home_str) = home.to_str()
    {
        [home_str, "/.config/lkmv/"].concat()
    } else {
        return Err(LKMVError::Config(
            "Couldn't determine Home directory".to_string(),
        ));
    };

    if profile == "default" {
        Ok([&path, "config.json"].concat())
    } else {
        Ok([&path, "config-", profile, ".json"].concat())
    }
}

impl PublicConfig {
    /// Saves to disk the public configuration information
    /// Uses the default CONFIG_PATH const or ENV Variable LKMV_CONFIG_PATH
    pub fn save(
        &self,
        profile: &str,
        private: &PrivateConfig,
        private_seed: &SecretVec<u8>,
    ) -> Result<(), LKMVError> {
        let cfg_path = get_config_path(profile)?;
        let path = Path::new(&cfg_path);

        // Check that directory structure exists
        if let Some(parent_path) = path.parent()
            && !parent_path.exists()
        {
            // Create parent directories
            fs::create_dir_all(parent_path).map_err(|e| {
                LKMVError::Config(format!(
                    "Couldn't create parent directory ({}): {}",
                    parent_path.to_string_lossy(),
                    e
                ))
            })?;
        }

        let public = PublicConfig {
            private: Some(private.save(private_seed)?),
            ..self.clone()
        };
        // Write config to disk
        fs::write(path, serde_json::to_string_pretty(&public)?).map_err(|e| {
            LKMVError::Config(format!(
                "Couldn't write public config to file ({}): {}",
                path.to_string_lossy(),
                e
            ))
        })?;

        Ok(())
    }

    /// Loads from disk the public information for LKMV to unlock it's secrets from the OS Secure
    /// Store
    pub fn load(profile: &str) -> Result<Self, LKMVError> {
        let cfg_path = get_config_path(profile)?;
        let path = Path::new(&cfg_path);

        let file = fs::File::open(path).map_err(|e| {
            LKMVError::Config(format!(
                "Couldn't load lkmv configuration file ({}) from disk: {e}",
                &cfg_path
            ))
        })?;

        match serde_json::from_reader(file) {
            Ok(s) => Ok(s),
            Err(e) => {
                warn!("Couldn't Deserialize PublicConfig. Reason: {e}");
                Err(e.into())
            }
        }
    }
}
