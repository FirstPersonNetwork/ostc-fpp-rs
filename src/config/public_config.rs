/*!
*  Public [crate::config::Config] information that is stored in plaintext on disk
*/

use crate::{CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, LF_PUBLIC_MEDIATOR_DID, config::Config};
use anyhow::{Context, Result, bail};
use console::style;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::Path};

/// Primary structure used for storing [crate::config::Config] data that is not sensitive
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PublicConfig {
    /// Identifier for a hardware token if being used
    pub token_id: Option<String>,

    /// Use an unlock code if hardware token isn't being used?
    /// true: Will take a unlock code --> Sha256 hash --> decrypt key
    /// false: Plain text retrieval from the OS Secure Store
    pub unlock_code: bool,

    /// Community DID
    pub community_did: String,

    /// Mediator DID
    pub mediator_did: String,
}

impl From<&Config> for PublicConfig {
    /// Extracts public information from the full Config
    fn from(cfg: &Config) -> Self {
        cfg.public.clone()
    }
}

/// Private helper to determine where the config file is located
fn get_config_path(profile: &str) -> Result<String> {
    if let Ok(config_path) = env::var("LKMV_CONFIG") {
        Ok(config_path)
    } else if let Some(home) = dirs::home_dir()
        && let Some(home_str) = home.to_str()
    {
        let f_name = if profile == "default" {
            "config.json".to_string()
        } else {
            ["config-", profile, ".json"].concat()
        };

        Ok([home_str, "/.config/lkmv/", &f_name].concat())
    } else {
        bail!("Couldn't determine Home directory");
    }
}

impl PublicConfig {
    /// Saves to disk the public configuration information
    /// Uses the default CONFIG_PATH const or ENV Variable LKMV_CONFIG
    pub fn save(&self, profile: &str) -> Result<()> {
        let cfg_path = get_config_path(profile)?;
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
    pub fn load(profile: &str) -> Result<Self> {
        let cfg_path = get_config_path(profile)?;
        let path = Path::new(&cfg_path);

        let file = fs::File::open(path).context(format!(
            "Couldn't load lkmv configuration file ({}) from disk",
            &cfg_path
        ))?;

        Ok(serde_json::from_reader(file)?)
    }

    /// Prints information relating to the Public configuration to console
    pub fn status(&self) {
        println!();
        println!("{}", style("Configuration information").color256(CLI_BLUE));
        println!("{}", style("=========================").color256(CLI_BLUE));
        if let Some(token_id) = &self.token_id {
            println!(
                "{} {}",
                style("Hardware Token:").color256(CLI_BLUE),
                style(token_id).color256(CLI_PURPLE)
            );
            println!(
                "{} {}",
                style("Using unlock code?").color256(CLI_BLUE),
                style("NOT-REQUIRED").color256(CLI_PURPLE)
            );
        } else {
            println!(
                "{} {}",
                style("Hardware Token:").color256(CLI_BLUE),
                style("No hardware token configured").color256(CLI_ORANGE)
            );
            print!("{} ", style("Using unlock code?").color256(CLI_BLUE));
            if self.unlock_code {
                println!("{}", style("YES").color256(CLI_GREEN));
            } else {
                println!("{}", style("NO").color256(CLI_ORANGE));
            }
        }

        println!(
            "{} {}",
            style("Community DID:").color256(CLI_BLUE),
            style(&self.community_did).color256(CLI_PURPLE)
        );
        print!("{} ", style("Mediator DID:").color256(CLI_BLUE));
        if self.mediator_did == LF_PUBLIC_MEDIATOR_DID {
            println!("{}", style(LF_PUBLIC_MEDIATOR_DID).color256(CLI_GREEN));
        } else {
            println!(
                "{} {}",
                style(&self.mediator_did).color256(CLI_ORANGE),
                style("Mediator is customised (not an issue if deliberate)").color256(CLI_BLUE)
            );
        }
    }
}
