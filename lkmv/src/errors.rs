/*! Common error types for LKMV
*/

use affinidi_data_integrity::DataIntegrityError;
use affinidi_tdk::{didcomm, messaging::errors::ATMError};
use thiserror::Error;

/// Linux Kernel Maintainer Verification Errors
#[derive(Error, Debug)]
pub enum LKMVError {
    #[error("Invalid Message Type: {0}")]
    InvalidMessage(String),

    #[error("Missing Secret Key Material. Key-ID: {0}")]
    MissingSecretKeyMaterial(String),

    #[error("Serialize/Deseriale Error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("DataIntegrityProof Error: {0}")]
    DataIntegrityProof(#[from] DataIntegrityError),

    #[error("ATM Error: {0}")]
    ATM(#[from] ATMError),

    #[error("DIDComm Error: {0}")]
    DIDComm(#[from] didcomm::error::Error),
}
