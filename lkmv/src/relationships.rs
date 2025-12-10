use crate::errors::LKMVError;
use affinidi_tdk::{
    didcomm::{Message, PackEncryptedOptions},
    messaging::{ATM, profiles::ATMProfile},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{sync::Arc, time::SystemTime};
use uuid::Uuid;

// ****************************************************************************
// Message Body Structure types
// ****************************************************************************

/// DIDComm message body sent to the remote party when requesting a relationship
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RelationshipRequestBody {
    pub reason: Option<String>,
    pub did: String,
}

/// DIDComm message body sent to the initiator of a relationship request when the request is
/// rejected
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RelationshipRejectBody {
    pub reason: Option<String>,
}

/// Body of a Relationship Rquest accept message
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RelationshipAcceptBody {
    pub did: String,
}

// ****************************************************************************
// Message Handling
// ****************************************************************************

/// Creates and send the relationship rejection message to the remote party
/// atm: Affinidi Trusted Messaging instance
/// from_profile: ATM Profile of the responder
/// to: DID of who we will send this rejection message to
/// mediator_did: DID of the mediator to forward this message through
/// reason: Optional reason for rejecting the relationship request
/// thid: Thread ID for the DIDComm message
pub async fn create_send_message_rejected(
    atm: &ATM,
    from_profile: &Arc<ATMProfile>,
    to: &str,
    mediator_did: &str,
    reason: Option<&str>,
    thid: &str,
) -> Result<(), LKMVError> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let msg = Message::build(
        Uuid::new_v4().into(),
        "https://linuxfoundation.org/lkmv/1.0/relationship-request-reject".to_string(),
        json!(RelationshipRejectBody {
            reason: reason.map(|r| r.to_string())
        }),
    )
    .from(from_profile.inner.did.to_string())
    .to(to.to_string())
    .thid(thid.to_string())
    .created_time(now)
    .expires_time(60 * 60 * 48) // 48 hours
    .finalize();

    // Pack the message
    let (msg, _) = msg
        .pack_encrypted(
            to,
            Some(&from_profile.inner.did),
            Some(&from_profile.inner.did),
            &atm.get_tdk().did_resolver,
            &atm.get_tdk().secrets_resolver,
            &PackEncryptedOptions {
                forward: false,
                ..Default::default()
            },
        )
        .await?;

    atm.forward_and_send_message(
        from_profile,
        false,
        &msg,
        None,
        mediator_did,
        to,
        None,
        None,
        false,
    )
    .await?;

    Ok(())
}

/// Creates and sends the relationship request accept message to the remote party
/// atm: Affinidi Trusted Messaging instance
/// from_profile: ATM Profile of the responder
/// to: DID of who we will send this rejection message to
/// mediator_did: DID of the mediator to forward this message through
/// r_did: The relationship DID to use for this relationship (May be the P-DID or R-DID)
/// thid: Thread ID for the DIDComm message
pub async fn create_send_message_accepted(
    atm: &ATM,
    from_profile: &Arc<ATMProfile>,
    to: &str,
    mediator_did: &str,
    r_did: &str,
    thid: &str,
) -> Result<(), LKMVError> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let msg = Message::build(
        Uuid::new_v4().into(),
        "https://linuxfoundation.org/lkmv/1.0/relationship-request-accept".to_string(),
        json!(RelationshipAcceptBody {
            did: r_did.to_string()
        }),
    )
    .from(from_profile.inner.did.to_string())
    .to(to.to_string())
    .thid(thid.to_string())
    .created_time(now)
    .expires_time(60 * 60 * 48) // 48 hours
    .finalize();

    // Pack the message
    // Pack the message
    let (msg, _) = msg
        .pack_encrypted(
            to,
            Some(&from_profile.inner.did),
            Some(&from_profile.inner.did),
            &atm.get_tdk().did_resolver,
            &atm.get_tdk().secrets_resolver,
            &PackEncryptedOptions {
                forward: false,
                ..Default::default()
            },
        )
        .await?;

    atm.forward_and_send_message(
        from_profile,
        false,
        &msg,
        None,
        mediator_did,
        to,
        None,
        None,
        false,
    )
    .await?;

    Ok(())
}
