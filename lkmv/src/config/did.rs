use std::collections::HashMap;

use affinidi_tdk::{
    did_common::{
        Document,
        service::{Endpoint, Service},
        verification_method::{VerificationMethod, VerificationRelationship},
    },
    secrets_resolver::secrets::Secret,
};
use didwebvh_rs::{DIDWebVHError, DIDWebVHState, parameters::Parameters, url::WebVHURL};
use ed25519_dalek_bip32::{DerivationPath, ExtendedSigningKey};
use serde_json::{Value, json};
use url::Url;

use crate::{config::PersonaDIDKeys, errors::LKMVError};

pub fn create_initial_webvh_did(
    raw_url: &str,
    keys: &mut PersonaDIDKeys,
    mediator_did: &str,
    bip32_root: ExtendedSigningKey,
) -> Result<(String, Document), LKMVError> {
    let did_url = WebVHURL::parse_url(&Url::parse(raw_url).map_err(|e| {
        LKMVError::WebVH(DIDWebVHError::ValidationError(format!(
            "Invalid URL ({raw_url}). {e}"
        )))
    })?)?;

    // Create the basic DID Document Structure
    let mut did_document = Document::new(&did_url.to_string())
        .map_err(|e| LKMVError::Config(format!("Invalid DID URL: {e}")))?;

    // Add the verification methods to the DID Document
    let mut property_set: HashMap<String, Value> = HashMap::new();

    // Signing Key
    property_set.insert(
        "publicKeyMultibase".to_string(),
        Value::String(keys.signing.secret.get_public_keymultibase().map_err(|e| {
            DIDWebVHError::InvalidMethodIdentifier(format!(
                "Couldn't set signing verificationMethod publicKeybase: {e}"
            ))
        })?),
    );
    let key_id =
        Url::parse(&[did_url.to_string(), "#key-1".to_string()].concat()).map_err(|e| {
            DIDWebVHError::InvalidMethodIdentifier(format!(
                "Couldn't set verificationMethod Key ID for #key-1: {e}"
            ))
        })?;
    did_document.verification_method.push(VerificationMethod {
        id: key_id.clone(),
        type_: "Multikey".to_string(),
        controller: did_document.id.clone(),
        revoked: None,
        expires: None,
        property_set: property_set.clone(),
    });
    did_document
        .assertion_method
        .push(VerificationRelationship::Reference(key_id.clone()));

    // Authentication Key
    property_set.insert(
        "publicKeyMultibase".to_string(),
        Value::String(
            keys.authentication
                .secret
                .get_public_keymultibase()
                .map_err(|e| {
                    DIDWebVHError::InvalidMethodIdentifier(format!(
                        "Couldn't set authentication verificationMethod publicKeybase: {e}"
                    ))
                })?,
        ),
    );
    let key_id =
        Url::parse(&[did_url.to_string(), "#key-2".to_string()].concat()).map_err(|e| {
            DIDWebVHError::InvalidMethodIdentifier(format!(
                "Couldn't set verificationMethod key ID for #key-2: {e}"
            ))
        })?;
    did_document.verification_method.push(VerificationMethod {
        id: key_id.clone(),
        type_: "Multikey".to_string(),
        controller: did_document.id.clone(),
        revoked: None,
        expires: None,
        property_set: property_set.clone(),
    });
    did_document
        .authentication
        .push(VerificationRelationship::Reference(key_id.clone()));

    // Decryption Key
    property_set.insert(
        "publicKeyMultibase".to_string(),
        Value::String(
            keys.decryption
                .secret
                .get_public_keymultibase()
                .map_err(|e| {
                    DIDWebVHError::InvalidMethodIdentifier(format!(
                        "Couldn't set decryption verificationMethod publicKeybase: {e}"
                    ))
                })?,
        ),
    );
    let key_id =
        Url::parse(&[did_url.to_string(), "#key-3".to_string()].concat()).map_err(|e| {
            DIDWebVHError::InvalidMethodIdentifier(format!(
                "Couldn't set verificationMethod key ID for #key-3: {e}"
            ))
        })?;
    did_document.verification_method.push(VerificationMethod {
        id: key_id.clone(),
        type_: "Multikey".to_string(),
        controller: did_document.id.clone(),
        revoked: None,
        expires: None,
        property_set: property_set.clone(),
    });
    did_document
        .key_agreement
        .push(VerificationRelationship::Reference(key_id.clone()));

    // Add a service endpoint for this persona
    // NOTE: This will use the public mediator

    let endpoint = Endpoint::Map(json!([{"accept": ["didcomm/v2"], "uri": mediator_did}]));
    did_document.service.push(Service {
        id: Some(
            Url::parse(&[did_url.to_string(), "#public-didcomm".to_string()].concat()).map_err(
                |e| {
                    DIDWebVHError::InvalidMethodIdentifier(format!(
                        "Couldn't set Service Endpoint for #public-didcomm: {e}"
                    ))
                },
            )?,
        ),
        type_: vec!["DIDCommMessaging".to_string()],
        property_set: HashMap::new(),
        service_endpoint: endpoint,
    });

    // Create the WebVH Parameters
    let update_key = bip32_root
        .derive(&"m/2'/1'/0'".parse::<DerivationPath>().unwrap())
        .map_err(|e| {
            LKMVError::BIP32(format!(
                "Failed to create an Ed25519 log_entry signing key. {e}"
            ))
        })?;
    let mut update_secret = Secret::generate_ed25519(None, Some(update_key.signing_key.as_bytes()));
    update_secret.id = [
        "did:key:",
        &update_secret.get_public_keymultibase().map_err(|e| {
            LKMVError::Secret(format!(
                "update Secret Key was missing public key information! {e}"
            ))
        })?,
        "#",
        &update_secret.get_public_keymultibase().map_err(|e| {
            LKMVError::Secret(format!(
                "update Secret Key was missing public key information! {e}"
            ))
        })?,
    ]
    .concat();

    let next_update_key = bip32_root
        .derive(&"m/2'/1'/1'".parse::<DerivationPath>().unwrap())
        .map_err(|e| {
            LKMVError::BIP32(format!("Failed to create an Ed25519 next_update key. {e}"))
        })?;
    let next_update_secret =
        Secret::generate_ed25519(None, Some(next_update_key.signing_key.as_bytes()));

    let parameters = Parameters::new()
        .with_key_pre_rotation(true)
        .with_update_keys(vec![update_secret.get_public_keymultibase().map_err(
            |e| {
                LKMVError::Secret(format!(
                    "next_update Secret Key was missing public key information! {e}"
                ))
            },
        )?])
        .with_next_key_hashes(vec![
            next_update_secret
                .get_public_keymultibase_hash()
                .map_err(|e| {
                    LKMVError::Secret(format!(
                        "next_update Secret Key was missing public key information! {e}"
                    ))
                })?,
        ])
        .with_portable(true)
        .build();

    // Create the WebVH DID
    let mut didwebvh = DIDWebVHState::default();
    let log_entry = didwebvh.create_log_entry(
        None,
        &serde_json::to_value(&did_document)?,
        &parameters,
        &update_secret,
    )?;

    // Save the DID to local file
    log_entry.log_entry.save_to_file("did.jsonl")?;

    Ok((
        log_entry
            .get_state()
            .get("id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        serde_json::from_value(log_entry.get_did_document()?)?,
    ))
}
