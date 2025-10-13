/*!
*   Handles everything todo with openpgp-card tokens
*/

use crate::{CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_RED};
use anyhow::Result;
use card_backend_pcsc::PcscBackend;
use chrono::{DateTime, Utc};
use console::style;
use openpgp_card::{
    Card,
    ocard::{
        KeyType,
        algorithm::{self, AlgorithmAttributes},
        crypto::PublicKeyMaterial,
        data::{Features, Fingerprint, KeyGenerationTime, KeySet, KeyStatus, TouchPolicy},
    },
    state::{Open, Transaction},
};
use std::fmt;

pub mod write;

/// Tags what the key is used for
#[derive(Default, Debug, PartialEq)]
pub enum KeyPurpose {
    Signing,
    Authentication,
    Encryption,
    #[default]
    Unknown,
}

impl fmt::Display for KeyPurpose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyPurpose::Signing => write!(f, "Signing"),
            KeyPurpose::Authentication => write!(f, "Authentication"),
            KeyPurpose::Encryption => write!(f, "Encryption"),
            KeyPurpose::Unknown => write!(f, "Unknown"),
        }
    }
}

impl From<KeyType> for KeyPurpose {
    fn from(kt: KeyType) -> Self {
        match kt {
            KeyType::Signing => KeyPurpose::Signing,
            KeyType::Authentication => KeyPurpose::Authentication,
            KeyType::Decryption => KeyPurpose::Encryption,
            _ => KeyPurpose::Unknown,
        }
    }
}

pub struct KeySlotInfo {
    /// Purpose for this key (signing/authentication/encryption)
    purpose: KeyPurpose,
    /// PGP Public Key Fingerprint
    fingerprint: Option<String>,
    /// Time that this key was generated
    /// 2025-10-02 03:21:06 UTC
    creation_time: Option<String>,
    /// Algorithm used for this key
    algorithm: Option<AlgorithmAttributes>,
    /// Does this key require touch to use?
    touch_policy: TouchPolicy,
    /// Additional info relating to the touch policy
    touch_features: Features,
    /// Status of the key
    status: Option<KeyStatus>,
    /// Public key material
    public_key_material: Option<Vec<u8>>,
    /// Number of Digital Signatures created with this key
    signature_count: Option<u32>,
}

impl Default for KeySlotInfo {
    fn default() -> Self {
        KeySlotInfo {
            purpose: KeyPurpose::Unknown,
            fingerprint: None,
            creation_time: None,
            algorithm: None,
            touch_policy: TouchPolicy::Off,
            touch_features: Features::from(0_u8),
            status: None,
            public_key_material: None,
            signature_count: None,
        }
    }
}

impl fmt::Debug for KeySlotInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "KeyPurpose: {:?}", self.purpose)?;
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
        writeln!(f, "  Touch Policy: {:?}", self.touch_policy)?;
        writeln!(f, "  Touch Features: {:?}", self.touch_features.to_string())?;
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

        print!(" {}", style("Card Holder Name: ").color256(CLI_BLUE));
        if let Some(cardholder) = format_cardholder_name(&open_card.cardholder_name()?) {
            println!("{}", style(cardholder).color256(CLI_GREEN));
        } else {
            println!("{}", style("<NOT SET>").color256(CLI_ORANGE));
        }

        // Check key status for this hardware token
        let fps = open_card.fingerprints()?;
        let kgt = open_card.key_generation_times()?;

        let sign_info = get_key_info(&mut open_card, &fps, &kgt, KeyType::Signing)?;
        print_key_info(&sign_info);

        let auth_info = get_key_info(&mut open_card, &fps, &kgt, KeyType::Authentication)?;
        print_key_info(&auth_info);

        let enc_info = get_key_info(&mut open_card, &fps, &kgt, KeyType::Decryption)?;
        print_key_info(&enc_info);
    }

    Ok(())
}

/// Retrieves key slot information from a hardware token
pub fn get_key_info(
    card: &mut Card<Transaction>,
    fps: &KeySet<Fingerprint>,
    kgt: &KeySet<KeyGenerationTime>,
    key_type: KeyType,
) -> Result<KeySlotInfo> {
    let mut key_info = KeySlotInfo {
        purpose: key_type.into(),
        ..Default::default()
    };
    let ki = card.key_information().ok().flatten();

    key_info.algorithm = Some(card.algorithm_attributes(key_type)?);

    if let Some(uif) = card.user_interaction_flag(key_type)? {
        key_info.touch_policy = uif.touch_policy();
        key_info.touch_features = uif.features();
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
            if let Some(fp) = fps.signature() {
                key_info.fingerprint = Some(fp.to_hex());
            }
        }
        KeyType::Authentication => {
            if let Some(kgt) = kgt.authentication() {
                key_info.creation_time = Some(format!("{}", DateTime::<Utc>::from(kgt)));
            }
            key_info.status = ki.map(|ki| ki.aut_status());
            if let Some(fp) = fps.authentication() {
                key_info.fingerprint = Some(fp.to_hex());
            }
        }
        KeyType::Decryption => {
            if let Some(kgt) = kgt.decryption() {
                key_info.creation_time = Some(format!("{}", DateTime::<Utc>::from(kgt)));
            }
            key_info.status = ki.map(|ki| ki.dec_status());
            if let Some(fp) = fps.decryption() {
                key_info.fingerprint = Some(fp.to_hex());
            }
        }
        _ => {}
    }
    Ok(key_info)
}

/// Checks that everything is ok with the keyslot
pub fn check_keyslot(ki: &KeySlotInfo) -> bool {
    if let Some(KeyStatus::NotPresent) = &ki.status {
        return false;
    }

    match &ki.purpose {
        KeyPurpose::Signing => {
            if let Some(AlgorithmAttributes::Ecc(attr)) = &ki.algorithm {
                if attr.curve() != &algorithm::Curve::Ed25519 {
                    return false;
                }
            } else {
                return false;
            }

            // Best practice for Signing key is for it to require some form of user interface
            if ki.touch_policy == TouchPolicy::Off {
                return false;
            }

            if ki.public_key_material.is_none() {
                return false;
            }

            true
        }
        KeyPurpose::Authentication => {
            if let Some(AlgorithmAttributes::Ecc(attr)) = &ki.algorithm {
                if attr.curve() != &algorithm::Curve::Ed25519 {
                    return false;
                }
            } else {
                return false;
            }

            if ki.public_key_material.is_none() {
                return false;
            }

            true
        }
        KeyPurpose::Encryption => {
            if let Some(AlgorithmAttributes::Ecc(attr)) = &ki.algorithm {
                if attr.curve() != &algorithm::Curve::Curve25519 {
                    return false;
                }
            } else {
                return false;
            }

            if ki.public_key_material.is_none() {
                return false;
            }

            true
        }
        KeyPurpose::Unknown => false,
    }
}

/// Prints a hardware token key details to the console
pub fn print_key_info(ki: &KeySlotInfo) {
    if let Some(KeyStatus::NotPresent) = &ki.status {
        println!(
            "  {}{}{}{}",
            style("Keyslot (").color256(CLI_BLUE),
            style(&ki.purpose).color256(CLI_ORANGE),
            style(") is").color256(CLI_BLUE),
            style(" NOT_SET").color256(CLI_RED)
        );
        return;
    }

    match (&ki.purpose, &ki.algorithm) {
        (KeyPurpose::Signing, Some(algo)) | (KeyPurpose::Authentication, Some(algo)) => {
            if let AlgorithmAttributes::Ecc(attr) = algo {
                if attr.curve() == &algorithm::Curve::Ed25519 {
                    print!(
                        "  {}{}{}{}{}",
                        style("Keyslot (").color256(CLI_BLUE),
                        style(&ki.purpose).color256(CLI_ORANGE),
                        style(") Algorithm (").color256(CLI_BLUE),
                        style("Ed25519").color256(CLI_GREEN),
                        style(")").color256(CLI_BLUE),
                    );
                } else {
                    println!(
                        "  {}{}{}{}",
                        style("Keyslot (").color256(CLI_BLUE),
                        style(&ki.purpose).color256(CLI_ORANGE),
                        style(") expected crypto algorithm Ed25519, this is an ECC algo but not of type Ed25519. Instead it is: ")
                            .color256(CLI_BLUE),
                        style(format!("{:?}", attr.curve())).color256(CLI_RED)
                    );
                    return;
                }
            } else {
                println!(
                    "  {}{}{}{}",
                    style("Keyslot (").color256(CLI_BLUE),
                    style(&ki.purpose).color256(CLI_ORANGE),
                    style(") expected crypto algorithm Ed25519, instead this is key is: ")
                        .color256(CLI_BLUE),
                    style(algo).color256(CLI_RED)
                );
                return;
            }
        }
        (KeyPurpose::Encryption, Some(algo)) => {
            if let AlgorithmAttributes::Ecc(attr) = algo {
                if attr.curve() == &algorithm::Curve::Curve25519 {
                    print!(
                        "  {}{}{}{}{}",
                        style("Keyslot (").color256(CLI_BLUE),
                        style(&ki.purpose).color256(CLI_ORANGE),
                        style(") Algorithm (").color256(CLI_BLUE),
                        style("X25519").color256(CLI_GREEN),
                        style(")").color256(CLI_BLUE),
                    );
                } else {
                    println!(
                        "  {}{}{}{}",
                        style("Keyslot (").color256(CLI_BLUE),
                        style(&ki.purpose).color256(CLI_ORANGE),
                        style(") expected crypto algorithm X25519, this is an ECC algo but not of type X25519. Instead it is: ")
                            .color256(CLI_BLUE),
                        style(format!("{:?}", attr.curve())).color256(CLI_RED)
                    );
                    return;
                }
            } else {
                println!(
                    "  {}{}{}{}",
                    style("Keyslot (").color256(CLI_BLUE),
                    style(&ki.purpose).color256(CLI_ORANGE),
                    style(") expected crypto algorithm X25519, instead this is key is: ")
                        .color256(CLI_BLUE),
                    style(algo).color256(CLI_RED)
                );
                return;
            }
        }
        _ => {
            println!("{ki:#?}");
            return;
        }
    }

    if let Some(fp) = &ki.fingerprint {
        print!(
            " {}{}{}",
            style("Fingerprint (").color256(CLI_BLUE),
            style(fp).color256(CLI_GREEN),
            style(")").color256(CLI_BLUE)
        );
    } else {
        print!(
            " {}{}{}",
            style("Fingerprint (").color256(CLI_BLUE),
            style("<NOT SET>").color256(CLI_RED),
            style(")").color256(CLI_BLUE)
        );
    }

    // How to unlock the token
    if ki.purpose == KeyPurpose::Signing {
        // Best practice for Signing key is for it to require some form of user interface
        if ki.touch_policy == TouchPolicy::Off {
            print!(
                " {}{}{}{}{}",
                style("Touch Policy (").color256(CLI_BLUE),
                style(ki.touch_policy).color256(CLI_RED).blink(),
                style(" :: ").color256(CLI_BLUE),
                style(&ki.touch_features).color256(CLI_GREEN),
                style(")").color256(CLI_BLUE)
            );
        } else {
            print!(
                " {}{}{}{}{}",
                style("Touch Policy (").color256(CLI_BLUE),
                style(ki.touch_policy).color256(CLI_GREEN),
                style(" :: ").color256(CLI_BLUE),
                style(&ki.touch_features).color256(CLI_GREEN),
                style(")").color256(CLI_BLUE)
            );
        }
    } else {
        print!(
            " {}{}{}{}{}",
            style("Touch Policy (").color256(CLI_BLUE),
            style(ki.touch_policy).color256(CLI_GREEN),
            style(" :: ").color256(CLI_BLUE),
            style(&ki.touch_features).color256(CLI_GREEN),
            style(")").color256(CLI_BLUE)
        );
    }

    // Status of the key
    if let Some(status) = &ki.status {
        print!(" {}", style("Key Status (").color256(CLI_BLUE));
        match status {
            KeyStatus::Imported => print!("{}", style(status).color256(CLI_GREEN)),
            KeyStatus::Generated => print!("{}", style(status).color256(CLI_ORANGE)),
            KeyStatus::NotPresent => print!("{}", style(status).color256(CLI_RED)),
            KeyStatus::Unknown(_) => print!("{}", style(status).color256(CLI_RED)),
        }
        print!("{}", style(")").color256(CLI_BLUE));
    }

    if let Some(ct) = &ki.creation_time {
        print!(
            " {}{}{}",
            style("Creation Time (").color256(CLI_BLUE),
            style(ct).color256(CLI_GREEN),
            style(")").color256(CLI_BLUE)
        );
    }

    println!();
}
