/*!
*  Public [crate::config::Config] information that is stored in plaintext on disk
*/

use serde::{Deserialize, Serialize};

use crate::config::KeySourceMaterial;

/// What are the keys being used for the top level DID?
#[derive(Serialize, Deserialize, Debug)]
pub struct CommunityDidKeysPaths {
    pub signing: KeySourceMaterial,
    pub authentication: KeySourceMaterial,
    pub encryption: KeySourceMaterial,
}

/// Primary top-level structure used for storing [crate::config::Config] data that is not sensitive
#[derive(Serialize, Deserialize, Debug)]
pub struct PublicConfig {
    /// What are the known keys for the community DID?
    pub community_did_keys: CommunityDidKeysPaths,
}
