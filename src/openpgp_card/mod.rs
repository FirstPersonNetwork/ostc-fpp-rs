/*!
*   Handles everything todo with openpgp-card tokens
*/

use crate::{CLI_BLUE, CLI_GREEN, CLI_PURPLE};
use anyhow::Result;
use card_backend_pcsc::PcscBackend;
use chrono::{DateTime, Utc};
use console::style;
use openpgp_card::{
    Card,
    ocard::{
        KeyType,
        algorithm::AlgorithmAttributes,
        crypto::PublicKeyMaterial,
        data::{Features, Fingerprint, KeyGenerationTime, KeySet, KeyStatus, TouchPolicy},
    },
    state::{Open, Transaction},
};
use std::fmt;

#[derive(Default)]
pub struct KeySlotInfo {
    /// PGP Public Key Fingerprint
    fingerprint: Option<String>,
    /// Time that this key was generated
    /// 2025-10-02 03:21:06 UTC
    creation_time: Option<String>,
    /// Algorithm used for this key
    algorithm: Option<AlgorithmAttributes>,
    /// Does this key require touch to use?
    touch_policy: Option<TouchPolicy>,
    /// Additional info relating to the touch policy
    touch_features: Option<Features>,
    /// Status of the key
    status: Option<KeyStatus>,
    /// Public key material
    public_key_material: Option<Vec<u8>>,
    /// Number of Digital Signatures created with this key
    signature_count: Option<u32>,
}

impl fmt::Debug for KeySlotInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "KeySlotInfo {{")?;
        if let Some(fp) = &self.fingerprint {
            writeln!(f, "  Fingerprint: {}", fp)?;
        }
        if let Some(ct) = &self.creation_time {
            writeln!(f, "  Creation Time: {}", ct)?;
        }
        if let Some(alg) = &self.algorithm {
            writeln!(f, "  Algorithm: {}", alg)?;
        }
        if let Some(tp) = &self.touch_policy {
            writeln!(f, "  Touch Policy: {:?}", tp)?;
        }
        if let Some(tf) = &self.touch_features {
            writeln!(f, "  Touch Features: {:?}", tf.to_string())?;
        }
        if let Some(status) = &self.status {
            writeln!(f, "  Status: {:?}", status)?;
        }
        if let Some(pk) = &self.public_key_material {
            writeln!(f, "  Public Key Material: {:02X?}", pk)?;
        }
        if let Some(sc) = &self.signature_count {
            writeln!(f, "  Signature Count: {}", sc)?;
        }
        writeln!(f, "}}")
    }
}

/// Get a list of active cards on this system
pub fn cards() -> Result<Vec<Card<Open>>> {
    let mut cards = vec![];

    for backend in PcscBackend::cards(None)? {
        let card = Card::<Open>::new(backend?)?;
        cards.push(card);
    }

    Ok(cards)
}

/// Formats the cardholder name
/// Returns None if the name is empty
pub fn format_cardholder_name(card_holder: &str) -> Option<String> {
    if card_holder.is_empty() {
        None
    } else {
        // cardholder name format is LAST_NAME<<FIRST_NAME<OTHER
        // NOTE: May not always contains the << Filler
        // See  ISO/IEC 7501-1 for more info

        if card_holder.contains("<<") {
            let parts: Vec<&str> = card_holder.split("<<").collect();
            let last_name = parts
                .first()
                .unwrap_or(&"")
                .replace("<", " ")
                .trim()
                .to_string();
            let first_names = parts
                .get(1)
                .unwrap_or(&"")
                .replace("<", " ")
                .trim()
                .to_string();
            Some(format!("{} {}", first_names, last_name))
        } else {
            Some(card_holder.replace("<", " ").trim().to_string())
        }
    }
}

pub fn print_cards(cards: &mut [Card<Open>]) -> Result<()> {
    println!(
        "{} {}",
        style("Cards found:").color256(CLI_BLUE),
        style(cards.len()).color256(CLI_GREEN)
    );
    for card in cards.iter_mut() {
        let mut open_card = card.transaction()?;
        let app_identifier = open_card.application_identifier()?;
        print!(
            "{} {} {} {}",
            style("Card Identifier:").color256(CLI_BLUE),
            style(app_identifier.ident()).color256(CLI_GREEN),
            style("Found token from manufacturer:").color256(CLI_BLUE),
            style(app_identifier.manufacturer_name()).color256(CLI_GREEN),
        );

        print!(" {}", style("Card Holder Name:").color256(CLI_BLUE));
        if let Some(cardholder) = format_cardholder_name(&open_card.cardholder_name()?) {
            println!("{}", style(cardholder).color256(CLI_GREEN));
        } else {
            println!("{}", style("<NOT SET>").color256(CLI_PURPLE).blink());
        }

        // Check key status for this hardware token
        let fps = open_card.fingerprints()?;
        let kgt = open_card.key_generation_times()?;

        println!("{}", style("Work in progress...").color256(CLI_PURPLE));
        let sign_info = get_key_info(&mut open_card, &fps, &kgt, KeyType::Signing)?;
        println!("SIGNING_KEY: {:#?}", sign_info);

        let auth_info = get_key_info(&mut open_card, &fps, &kgt, KeyType::Authentication)?;
        println!("AUTHENTICATION_KEY: {:#?}", auth_info);

        let enc_info = get_key_info(&mut open_card, &fps, &kgt, KeyType::Decryption)?;
        println!("DECRYPTION_KEY: {:#?}", enc_info);
    }

    Ok(())
}

pub fn get_key_info(
    card: &mut Card<Transaction>,
    fps: &KeySet<Fingerprint>,
    kgt: &KeySet<KeyGenerationTime>,
    key_type: KeyType,
) -> Result<KeySlotInfo> {
    let mut key_info = KeySlotInfo::default();
    let ki = card.key_information().ok().flatten();

    if let Some(fp) = fps.signature() {
        key_info.fingerprint = Some(fp.to_hex());
    }

    key_info.algorithm = Some(card.algorithm_attributes(key_type)?);

    if let Some(uif) = card.user_interaction_flag(key_type)? {
        key_info.touch_policy = Some(uif.touch_policy());
        key_info.touch_features = Some(uif.features());
    }

    if let Ok(PublicKeyMaterial::E(pkm)) = card.public_key_material(key_type) {
        key_info.public_key_material = Some(pkm.data().to_vec());
    }

    match key_type {
        KeyType::Signing => {
            if let Some(kgt) = kgt.signature() {
                key_info.creation_time = Some(format!("{}", DateTime::<Utc>::from(kgt)));
            }
            key_info.status = ki.map(|ki| ki.sig_status());
            key_info.signature_count = Some(card.digital_signature_count()?);
        }
        KeyType::Authentication => {
            if let Some(kgt) = kgt.authentication() {
                key_info.creation_time = Some(format!("{}", DateTime::<Utc>::from(kgt)));
            }
            key_info.status = ki.map(|ki| ki.aut_status());
        }
        KeyType::Decryption => {
            if let Some(kgt) = kgt.decryption() {
                key_info.creation_time = Some(format!("{}", DateTime::<Utc>::from(kgt)));
            }
            key_info.status = ki.map(|ki| ki.dec_status());
        }
        _ => {}
    }
    Ok(key_info)
}
