use affinidi_tdk::secrets_resolver::secrets::Secret;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::error;

/// Known Maintainers
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Maintainers {
    pub alias: String,
    pub did: String,
}

/// LKM Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub maintainers: Vec<Maintainers>,
    pub mediator: String,
    pub our_did: String,
    pub secrets: Vec<Secret>,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let file = fs::File::open(path).context(format!(
            "Couldn't load lkm configuration file ({}) from disk",
            &path
        ))?;

        match serde_json::from_reader(file) {
            Ok(s) => Ok(s),
            Err(e) => {
                error!("ERROR: Couldn't Deserialize Config file. Reason: {}", e);
                bail!("Deserialization error")
            }
        }
    }
}
