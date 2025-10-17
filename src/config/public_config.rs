/*!
*  Public [crate::config::Config] information that is stored in plaintext on disk
*/

use crate::config::Config;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::Path};

/// Primary top-level structure used for storing [crate::config::Config] data that is not sensitive
#[derive(Serialize, Deserialize, Debug)]
pub struct PublicConfig {
    /// Identifier for a hardware token if being used
    pub token_id: Option<String>,

    /// Use an unlock code if hardware token isn't being used?
    /// true: Will take a unlock code --> Sha256 hash --> decrypt key
    /// false: Plain text retrieval from the OS Secure Store
    pub unlock_code: bool,

    /// Community DID
    pub community_did: String,
}

impl From<&Config> for PublicConfig {
    /// Extracts public information from the full Config
    fn from(cfg: &Config) -> Self {
        PublicConfig {
            token_id: cfg.token_id.clone(),
            unlock_code: cfg.unlock_code,
            community_did: cfg.community_did.id.clone(),
        }
    }
}

/// Private helper to determine where the config file is located
fn get_config_path() -> Result<String> {
    if let Ok(config_path) = env::var("LKMV_CONFIG") {
        Ok(config_path)
    } else if let Some(home) = dirs::home_dir()
        && let Some(home_str) = home.to_str()
    {
        Ok([home_str, "/.config/lkmv/config.json"].concat())
    } else {
        bail!("Couldn't determine Home directory");
    }
}

impl PublicConfig {
    /// Saves to disk the public configuration information
    /// Uses the default CONFIG_PATH const or ENV Variable LKMV_CONFIG
    pub fn save(&self) -> Result<()> {
        let cfg_path = get_config_path()?;
        let path = Path::new(&cfg_path);

        // Check that directory structure exists
        if let Some(parent_path) = path.parent()
            && !parent_path.exists()
        {
            // Create parent directories
            fs::create_dir_all(parent_path)?;
        }

        // Write config to disk
        fs::write(path, serde_json::to_string_pretty(self)?)?;

        Ok(())
    }

    /// Loads from disk the public information for LKMV to unlock it's secrets from the OS Secure
    /// Store
    pub fn load() -> Result<Self> {
        let cfg_path = get_config_path()?;
        let path = Path::new(&cfg_path);

        let file = fs::File::open(path).context(format!(
            "Couldn't load lkmv configuration file ({}) from disk",
            &cfg_path
        ))?;

        Ok(serde_json::from_reader(file)?)
    }
}
