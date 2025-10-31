/*!
*   DID Setup methods
*/

use crate::{CLI_BLUE, CLI_GREEN, CLI_PURPLE, LF_PUBLIC_MEDIATOR_DID, setup::CommunityDIDKeys};
use affinidi_tdk::{
    did_common::{
        Document,
        service::{Endpoint, Service},
        verification_method::{VerificationMethod, VerificationRelationship},
    },
    secrets_resolver::secrets::Secret,
};
use anyhow::{Context, Result};
use console::style;
use dialoguer::{Input, theme::ColorfulTheme};
use didwebvh_rs::{DIDWebVHState, parameters::Parameters, url::WebVHURL};
use ed25519_dalek_bip32::{DerivationPath, ExtendedSigningKey};
use serde_json::{Value, json};
use std::collections::HashMap;
use url::Url;

/// Contains configuration info relating to the DID Setup
pub struct DIDConfig {
    /// DID identifier
    pub did: String,
    /// DID Document
    pub document: Document,
}

/// Creates an initial DID representing the Community DID
pub fn did_setup(bip32_root: ExtendedSigningKey, keys: &mut CommunityDIDKeys) -> Result<DIDConfig> {
    println!();
    println!("{}", style("DID Setup").color256(CLI_BLUE));
    println!("{}", style("=========").color256(CLI_BLUE));

    println!(
        "{}",
        style("A WebVH method DID will be created to represent your Community DID")
            .color256(CLI_BLUE)
    );

    println!("{}", style("Your DID needs to be publicly hosted somewhere, github or similar is an easy place to start!").color256(CLI_BLUE));

    let raw_url: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter URL of where your DID will be hosted")
        .validate_with(|url: &String| {
            if Url::parse(url).is_ok() {
                Ok(())
            } else {
                Err("Please enter a valid URL")
            }
        })
        .interact()
        .unwrap();

    let did_url = WebVHURL::parse_url(&Url::parse(&raw_url)?)?;

    println!(
        "{} {}",
        style("WebVH Starting DID:").color256(CLI_BLUE),
        style(&did_url).color256(CLI_GREEN)
    );

    // Create the basic DID Document Structure
    let mut did_document = Document::new(&did_url.to_string())?;

    // Add the verification methods to the DID Document
    let mut property_set: HashMap<String, Value> = HashMap::new();

    // Signing Key
    property_set.insert(
        "publicKeyMultibase".to_string(),
        Value::String(keys.signing.secret.get_public_keymultibase()?),
    );
    let key_id = Url::parse(&[did_url.to_string(), "#key-1".to_string()].concat())?;
    did_document.verification_method.push(VerificationMethod {
        id: key_id.clone(),
        type_: "Multikey".to_string(),
        controller: Url::parse(&did_url.to_string())?,
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
        Value::String(keys.authentication.secret.get_public_keymultibase()?),
    );
    let key_id = Url::parse(&[did_url.to_string(), "#key-2".to_string()].concat())?;
    did_document.verification_method.push(VerificationMethod {
        id: key_id.clone(),
        type_: "Multikey".to_string(),
        controller: Url::parse(&did_url.to_string())?,
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
        Value::String(keys.decryption.secret.get_public_keymultibase()?),
    );
    let key_id = Url::parse(&[did_url.to_string(), "#key-3".to_string()].concat())?;
    did_document.verification_method.push(VerificationMethod {
        id: key_id.clone(),
        type_: "Multikey".to_string(),
        controller: Url::parse(&did_url.to_string())?,
        revoked: None,
        expires: None,
        property_set: property_set.clone(),
    });
    did_document
        .key_agreement
        .push(VerificationRelationship::Reference(key_id.clone()));

    // Add a service endpoint for this community
    // NOTE: This will use the public mediator

    let endpoint =
        Endpoint::Map(json!([{"accept": ["didcomm/v2"], "uri": LF_PUBLIC_MEDIATOR_DID}]));
    did_document.service.push(Service {
        id: Some(Url::parse(
            &[did_url.to_string(), "#public-didcomm".to_string()].concat(),
        )?),
        type_: vec!["DIDCommMessaging".to_string()],
        property_set: HashMap::new(),
        service_endpoint: endpoint,
    });

    // Create the WebVH Parameters
    let update_key = bip32_root
        .derive(&"m/0'/1'/0'".parse::<DerivationPath>().unwrap())
        .context("Failed to create Ed25519 signing key")?;
    let mut update_secret = Secret::generate_ed25519(None, Some(update_key.signing_key.as_bytes()));
    update_secret.id = [
        "did:key:",
        &update_secret.get_public_keymultibase()?,
        "#",
        &update_secret.get_public_keymultibase()?,
    ]
    .concat();

    let next_update_key = bip32_root
        .derive(&"m/0'/1'/1'".parse::<DerivationPath>().unwrap())
        .context("Failed to create Ed25519 signing key")?;
    let next_update_secret =
        Secret::generate_ed25519(None, Some(next_update_key.signing_key.as_bytes()));

    let parameters = Parameters::new()
        .with_key_pre_rotation(true)
        .with_update_keys(vec![update_secret.get_public_keymultibase()?])
        .with_next_key_hashes(vec![next_update_secret.get_public_keymultibase_hash()?])
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

    println!(
        "{}",
        style("WebVH Log Entry successfully created").color256(CLI_BLUE)
    );

    // save to disk
    log_entry.log_entry.save_to_file("did.jsonl")?;
    println!(
        "{} {}",
        style("DID Saved:").color256(CLI_BLUE),
        style("did.jsonl").color256(CLI_GREEN)
    );

    let did_id = log_entry.get_state().get("id").unwrap().as_str().unwrap();

    // Change the key ID's to match the DID VM ID's
    keys.signing.secret.id = [did_id, "#key-1"].concat();
    keys.authentication.secret.id = [did_id, "#key-2"].concat();
    keys.decryption.secret.id = [did_id, "#key-3"].concat();

    println!();
    println!(
        "{} {} {}{}{}",
        style("You will need to publish the").color256(CLI_BLUE),
        style("did.jsonl").color256(CLI_PURPLE),
        style("to the URL (").color256(CLI_BLUE),
        style(&did_url.get_http_url(None)?).color256(CLI_PURPLE),
        style(") before you will be able to resolve your DID publicly").color256(CLI_BLUE),
    );

    Ok(DIDConfig {
        did: did_id.to_string(),
        document: serde_json::from_value(
            log_entry
                .get_did_document()
                .context("Couldn't get initial DID Document state")?,
        )
        .context("Serializing initial DID Document state failed")?,
    })
}
