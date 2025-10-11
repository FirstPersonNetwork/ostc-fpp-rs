/*!
*  Secured [crate::config::Config] information that is stored in the OS Secure Storage
*/

/// Secured Configuration information for lkmv tool
/// Try to keep this as small as possible for ease of secure storage
use std::collections::HashMap;

use affinidi_tdk::secrets_resolver::secrets::Secret;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub struct SecuredConfig {
    /// Secrets stored in the OS Secure Storage
    /// key: #key-id
    /// value: Secret
    pub keys: HashMap<String, Secret>,
}
