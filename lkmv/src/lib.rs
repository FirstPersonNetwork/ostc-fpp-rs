/*! Library interface for LKMV
 *! Allows for other applications to use the same data structures and routines
*/

use crate::errors::LKMVError;
use affinidi_tdk::didcomm::Message;
use serde::{Deserialize, Serialize};

pub mod errors;
pub mod maintainers;
pub mod relationships;
pub mod vrc;

/// Defined Message Types for LKMV
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MessageType {
    RelationshipRequest,
    RelationshipRequestRejected,
    RelationshipRequestAccepted,
    RelationshipRequestFinalize,
    TrustPing,
    TrustPong,
    VRCRequest,
    VRCRequestRejected,
    VRCIssued,
    MaintainersListRequest,
    MaintainersListResponse,
}

impl MessageType {
    pub fn friendly_name(&self) -> String {
        match self {
            MessageType::RelationshipRequest => "Relationship Request",
            MessageType::RelationshipRequestRejected => "Relationship Request Rejected",
            MessageType::RelationshipRequestAccepted => "Relationship Request Accepted",
            MessageType::RelationshipRequestFinalize => "Relationship Request Finalize",
            MessageType::TrustPing => "Trust Ping (Send)",
            MessageType::TrustPong => "Trust Pong (Receive)",
            MessageType::VRCRequest => "VRC Request",
            MessageType::VRCRequestRejected => "VRC Request Rejected",
            MessageType::VRCIssued => "VRC Issued",
            MessageType::MaintainersListRequest => "List Known Maintainers (request)",
            MessageType::MaintainersListResponse => "List Known Maintainers (response)",
        }
        .to_string()
    }
}

/// Convert TaskTypes to type string
impl From<MessageType> for String {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::RelationshipRequest => {
                "https://linuxfoundation.org/lkmv/1.0/relationship-request".to_string()
            }
            MessageType::RelationshipRequestRejected => {
                "https://linuxfoundation.org/lkmv/1.0/relationship-request-reject".to_string()
            }
            MessageType::RelationshipRequestAccepted => {
                "https://linuxfoundation.org/lkmv/1.0/relationship-request-accept".to_string()
            }
            MessageType::RelationshipRequestFinalize => {
                "https://linuxfoundation.org/lkmv/1.0/relationship-request-finalize".to_string()
            }
            MessageType::TrustPing => "https://didcomm.org/trust-ping/2.0/ping".to_string(),
            MessageType::TrustPong => {
                "https://didcomm.org/trust-ping/2.0/ping-response".to_string()
            }
            MessageType::VRCRequest => "https://firstperson.network/vrc/1.0/request".to_string(),
            MessageType::VRCRequestRejected => {
                "https://firstperson.network/vrc/1.0/rejected".to_string()
            }
            MessageType::VRCIssued => "https://firstperson.network/vrc/1.0/issued".to_string(),
            MessageType::MaintainersListRequest => {
                "https://kernel.org/maintainers/1.0/list".to_string()
            }
            MessageType::MaintainersListResponse => {
                "https://kernel.org/maintainers/1.0/list/response".to_string()
            }
        }
    }
}

/// Convert &str to a MessageType based on type URL
impl TryFrom<&str> for MessageType {
    type Error = LKMVError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "https://linuxfoundation.org/lkmv/1.0/relationship-request" => {
                Ok(MessageType::RelationshipRequest)
            }
            "https://linuxfoundation.org/lkmv/1.0/relationship-request-reject" => {
                Ok(MessageType::RelationshipRequestRejected)
            }
            "https://linuxfoundation.org/lkmv/1.0/relationship-request-accept" => {
                Ok(MessageType::RelationshipRequestAccepted)
            }
            "https://linuxfoundation.org/lkmv/1.0/relationship-request-finalize" => {
                Ok(MessageType::RelationshipRequestFinalize)
            }
            "https://didcomm.org/trust-ping/2.0/ping" => Ok(MessageType::TrustPing),
            "https://didcomm.org/trust-ping/2.0/ping-response" => Ok(MessageType::TrustPong),
            "https://firstperson.network/vrc/1.0/request" => Ok(MessageType::VRCRequest),
            "https://firstperson.network/vrc/1.0/rejected" => Ok(MessageType::VRCRequestRejected),
            "https://firstperson.network/vrc/1.0/issued" => Ok(MessageType::VRCIssued),
            "https://kernel.org/maintainers/1.0/list" => Ok(MessageType::MaintainersListRequest),
            "https://kernel.org/maintainers/1.0/list/response" => {
                Ok(MessageType::MaintainersListResponse)
            }
            _ => Err(LKMVError::InvalidMessage(value.to_string())),
        }
    }
}

/// Convert a DIDComm message to a MessageType
impl TryFrom<&Message> for MessageType {
    type Error = LKMVError;

    fn try_from(value: &Message) -> Result<Self, Self::Error> {
        value.type_.as_str().try_into()
    }
}
