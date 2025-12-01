/*!
*   Verified Relationship Credentials (VRC)
*/

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_RED, config::Config, log::LogFamily,
    tasks::MessageType,
};
use affinidi_data_integrity::DataIntegrityProof;
use affinidi_tdk::{
    didcomm::Message,
    secrets_resolver::{SecretsResolver, ThreadedSecretsResolver},
};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use console::style;
use serde::{Deserialize, Serialize, Serializer};
use std::{rc::Rc, time::SystemTime};
use uuid::Uuid;

pub mod interact;
pub mod request;

/// Verifiable Relationship Credential Specification
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Vrc {
    /// JSON-LD links to contexts
    /// Must contain at least:
    /// https://www.w3.org/ns/credentials/v2
    /// https://firstperson.network/credentials/relationship/v1
    #[serde(rename = "@context")]
    pub context: Vec<String>,

    /// Credential type identifiers
    /// Must contain at least:
    /// VerifiableCredential
    /// RelationshipCredential
    #[serde(rename = "type")]
    pub type_: Vec<String>,

    /// DID of the entity issuing this credential
    pub issuer: String,

    /// ISO 8601 format of when this credentials become valid from
    #[serde(serialize_with = "iso8601_format")]
    pub valid_from: DateTime<Utc>,

    /// Human-readable name or title of this relationship
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,

    /// Human-readable description of the credential or the relationship
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,

    /// The relationship assertion between the entities involved
    pub credential_subject: CredentialSubject,

    /// Cryptographic proof of credential authenticity
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub proof: Option<DataIntegrityProof>,
}

fn iso8601_format<S>(timestamp: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(
        timestamp
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            .as_str(),
    )
}

impl Default for Vrc {
    fn default() -> Self {
        Vrc {
            context: vec![
                "https://www.w3.org/ns/credentials/v2".to_string(),
                "https://firstperson.network/credentials/relationship/v1".to_string(),
            ],
            type_: vec![
                "VerifiableCredential".to_string(),
                "RelationshipCredential".to_string(),
            ],
            issuer: String::new(),
            valid_from: Utc::now(),
            name: None,
            description: None,
            credential_subject: CredentialSubject::default(),
            proof: None,
        }
    }
}

impl Vrc {
    /// Creates a DIDComm message containing this VRC
    pub fn message(
        &self,
        from: &Rc<String>,
        to: &Rc<String>,
        task_id: Option<&Rc<String>>,
    ) -> Result<Message> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut message = Message::build(
            Uuid::new_v4().to_string(),
            MessageType::VRCIssued.into(),
            serde_json::to_value(self).context("Couldn't serialize VRC into JSON")?,
        )
        .from(from.to_string())
        .to(to.to_string())
        .created_time(now)
        .expires_time(60 * 60 * 48); // 48 hours

        if let Some(thid) = task_id {
            message = message.thid(thid.to_string());
        }

        Ok(message.finalize())
    }
}

/// The relationship assertion between the entities involved
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CredentialSubject {
    /// Information about the asserting ("from") entity
    pub from: FromSubject,

    /// Information about the target ("to") entity
    pub to: ToSubject,

    /// Optional: URI or term from a published vocabulary specifying the nature of the relationship
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub relationship_type: Option<String>,

    /// Optional: Start date of the relationship, in ISO 8601 format
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub start_date: Option<DateTime<Utc>>,

    /// Optional: End date of the relationship, if applicable, in ISO 8601 format
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub end_date: Option<DateTime<Utc>>,

    /// Optional: Describes the live witnessing session linking the Fair Witness and the participants, if any
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub session: Option<Session>,
}

/// Information about the asserting ("from") entity
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FromSubject {
    /// DID of the "from" entity
    pub did: String,

    /// Human-readable name of the "from" entity
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,

    /// Array of verifiable, subject-controlled identifiers/personas (such as DIDs or resolvable URIs) for trustworthy correlation across decentralized identity systems
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub also_known_as: Vec<String>,

    /// Optional: An array of cryptographic proofs, each generated by signing a
    /// canonical message (such as the subject's DID or a credential hash) with
    /// the private key of an identifier listed in alsoKnownAs. Used to demonstrate
    /// that the entity controls the referenced identifier.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub linkage_proofs: Vec<LinkageProof>,

    /// Optional: An array of references to externally published proofs or
    /// verifiable credentials that demonstrate the entity’s control over an
    /// identifier in alsoKnownAs
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub external_proofs: Vec<ExternalProof>,
}

impl FromSubject {
    /// Create a new From Subject including proofs
    /// NOTE: Does not support external proofs at this time
    ///
    /// from_did: DID of the VRC Issuer
    /// to_did: DID of the VRC Recipient
    pub async fn new(
        from_did: String,
        to_did: String,
        name: Option<String>,
        also_known_as: Vec<String>,
        secrets: &ThreadedSecretsResolver,
    ) -> Result<Self> {
        // A linkage_proof is derived from the following:
        // from_did
        // to_did
        // Alias DID

        let mut linkage_proofs = Vec::new();
        for alias in &also_known_as {
            let Some(secret) = secrets
                .get_secret([alias, "#key-1"].concat().as_str())
                .await
            else {
                println!(
                    "{} {}",
                    style("Couldn't find Secret key material for #key-id:").color256(CLI_RED),
                    style([alias, "#key-1"].concat()).color256(CLI_ORANGE)
                );
                bail!("COuldn't find Secret key material!");
            };

            let proof = DataIntegrityProof::sign_jcs_data(
                &[&from_did, &to_did, alias.as_str()].concat(),
                None,
                &secret,
                None,
            )?;
            linkage_proofs.push(LinkageProof {
                type_: proof.type_,
                identifier: alias.to_string(),
                created: DateTime::parse_from_rfc3339(&proof.created.unwrap())
                    .unwrap()
                    .to_utc(),
                proof_value: proof.proof_value.unwrap(),
                nonce: None,
            });
        }

        Ok(FromSubject {
            did: from_did,
            name,
            also_known_as,
            linkage_proofs,
            external_proofs: Vec::new(),
        })
    }
}

/// An array of cryptographic proofs, each generated by signing a canonical message
/// (such as the subject's DID or a credential hash) with the private key of an
/// identifier listed in alsoKnownAs. Used to demonstrate that the entity controls
/// the referenced identifier.
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LinkageProof {
    /// The proof or signature type (e.g., "Ed25519Signature2020", "PGPSignature2020")
    #[serde(rename = "type")]
    pub type_: String,

    /// The identifier (from alsoKnownAs) for which control is being proven
    pub identifier: String,

    /// The ISO 8601 date and time when the proof was created
    #[serde(serialize_with = "iso8601_format")]
    pub created: DateTime<Utc>,

    /// The cryptographic signature value, such as a JWS or PGP armored block,
    /// demonstrating control of the identifier
    pub proof_value: String,

    /// A unique nonce that used when coordinating an exchange session
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub nonce: Option<String>,
}

/// An array of references to externally published proofs or verifiable credentials
/// that demonstrate the entity’s control over an identifier in alsoKnownAs
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExternalProof {
    /// The identifier (from alsoKnownAs) to which the external proof refers
    pub identifier: String,

    /// A HTTPS URL, DID URL, or other resolvable URI where the verifiable proof
    /// can be found
    pub proof_url: String,

    /// The type or standard of the external proof
    /// (e.g., "VerifiableCredential", "LinkedDataSignature2020", "PGPSignature2020")
    #[serde(rename = "type", skip_serializing_if = "Option::is_none", default)]
    pub type_: Option<String>,

    /// The ISO 8601 date and time when the external proof was created.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created: Option<DateTime<Utc>>,
}

/// Information about the target ("to") entity
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToSubject {
    /// DID of the "to" entity
    pub did: String,

    /// Human-readable name of the "to" entity
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,

    /// Optional: Array of verifiable, subject-controlled identifiers/personas
    /// (such as DIDs or resolvable URIs) for trustworthy correlation across
    /// decentralized identity systems
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub also_known_as: Vec<String>,
}

impl ToSubject {
    /// Creates a new ToSubject
    /// did: DID of the "to" entity
    /// name: Optional friendly name for the "to" entity
    /// also_known_as: Optional array of verifiable, subject-controlled identifiers/personas
    pub fn new(did: String, name: Option<String>, also_known_as: Option<Vec<String>>) -> Self {
        ToSubject {
            did,
            name,
            also_known_as: also_known_as.unwrap_or_default(),
        }
    }
}

/// Describes the live witnessing session linking the Fair Witness and the
/// participants, if any
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// A unique session identifier (e.g. UUID or URN)
    pub id: String,

    /// Optional: The DID of the session witness of any
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub witness_id: Option<String>,
}

// ****************************************************************************
// VRC Request Structure
// ****************************************************************************

/// Structure of a request to someone to issue a VRC. Contains hints and information to help the
/// issuer create the VRC.
/// NOTE: It does not guarantee that the issuer will issue a VRC with the requested details.
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VrcRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional: Include a reason for the VRC Request?
    pub reason: Option<String>,

    /// Include the r_did if one exists?
    /// If true, will add r_did for this relationship to alsoKnownAs array of the "to" subject.
    /// Defaults to false
    pub include_r_did: bool,

    /// Optional: Relationship Type URI that you would like the issuer to use
    /// NOTE: The issuer may not honor this, and replace with their own value.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    /// Would you like to include the start date in the VRC?
    /// NOTE: The issuer may not honor this
    pub start_date: bool,

    /// Would you like to include the end date in the VRC?
    /// NOTE: The issuer may not honor this
    pub end_date: bool,

    /// Optional: Friendly name for yourself to include in the VRC
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl VrcRequest {
    /// Create a new VRCRequest with default values
    pub fn print(&self) {
        print!("{}", style("VRC Request Reason: ").color256(CLI_BLUE));
        if let Some(reason) = &self.reason {
            println!("{}", style(reason).color256(CLI_PURPLE));
        } else {
            println!("{}", style("NO REASON PROVIDED").color256(CLI_ORANGE));
        }

        print!(
            "{}",
            style("VRC Relationship Type Requested: ").color256(CLI_BLUE)
        );
        if let Some(type_) = &self.type_ {
            println!("{}", style(type_).color256(CLI_PURPLE));
        } else {
            println!("{}", style("NO TYPE REQUESTED").color256(CLI_ORANGE));
        }

        print!(
            "{} {} ",
            style("Friendly Name?").color256(CLI_BLUE),
            self.name
                .as_deref()
                .map(|m| style(m).color256(CLI_GREEN))
                .unwrap_or(style("N/A").color256(CLI_ORANGE))
        );
        print!(
            "{}",
            style("Include r_did in alsoKnownAs?: ").color256(CLI_BLUE)
        );
        if self.include_r_did {
            print!("{}", style("YES").color256(CLI_GREEN));
        } else {
            print!("{}", style("NO").color256(CLI_ORANGE));
        }

        print!(" {}", style("Requesting Start Date?: ").color256(CLI_BLUE));
        if self.start_date {
            print!("{}", style("YES").color256(CLI_GREEN));
        } else {
            print!("{}", style("NO").color256(CLI_ORANGE));
        }

        print!(" {}", style("Requesting End Date?: ").color256(CLI_BLUE));
        if self.end_date {
            println!("{}", style("YES").color256(CLI_GREEN));
        } else {
            println!("{}", style("NO").color256(CLI_ORANGE));
        }
    }

    /// Creates a DIDCOmm message for the request
    pub fn create_message(&self, to: &Rc<String>, from: &Rc<String>) -> Result<Message> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(Message::build(
            Uuid::new_v4().to_string(),
            "https://firstperson.network/vrc/1.0/request".to_string(),
            serde_json::to_value(self)?,
        )
        .from(from.to_string())
        .to(to.to_string())
        .created_time(now)
        .expires_time(60 * 60 * 48) // 48 hours
        .finalize())
    }
}

// ****************************************************************************
// VRC Request Reject Structure
// ****************************************************************************

/// VRC Request Rejected body
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VRCRequestReject {
    /// Optional: A reason for the rejection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl VRCRequestReject {
    /// Creates a DIDCOmm message for the rejection
    pub fn create_message(
        to: &Rc<String>,
        from: &Rc<String>,
        thid: &Rc<String>,
        reason: Option<String>,
    ) -> Result<Message> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(Message::build(
            Uuid::new_v4().to_string(),
            "https://firstperson.network/vrc/1.0/rejected".to_string(),
            serde_json::to_value(VRCRequestReject { reason })?,
        )
        .from(from.to_string())
        .to(to.to_string())
        .thid(thid.to_string())
        .created_time(now)
        .expires_time(60 * 60 * 48) // 48 hours
        .finalize())
    }
}

impl Config {
    /// Handles rejection of a VRC request
    pub fn handle_vrc_reject(
        &mut self,
        task_id: &Rc<String>,
        reason: Option<&str>,
        from: &Rc<String>,
    ) -> Result<()> {
        let reason = if let Some(reason) = reason {
            reason.to_string()
        } else {
            "NO REASON PROVIDED".to_string()
        };

        self.public.logs.insert(
            LogFamily::Relationship,
            format!(
                "Removed VRC ({}) request as rejected by remote entity Reason: {}",
                task_id, reason
            ),
        );

        self.private.tasks.remove(task_id);

        self.public.logs.insert(
            LogFamily::Task,
            format!(
                "VRC request rejected by remote DID({}) Task ID({}) Reason({})",
                from, task_id, reason
            ),
        );

        Ok(())
    }
}
