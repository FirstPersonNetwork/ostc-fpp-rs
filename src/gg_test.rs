use anyhow::Result;
use pgp::{
    composed::{Deserializable, DetachedSignature, SignedPublicKey, SignedSecretKey},
    crypto::hash::HashAlgorithm,
    types::Password,
};

pub fn test_signature() -> Result<()> {
    const DATA: &[u8] = b"Hello world!";

    // Create a signature over DATA with the private key
    let (private_key, _headers) = SignedSecretKey::from_armor_file("key.sec.asc")?;
    let sig = DetachedSignature::sign_binary_data(
        rand::thread_rng(),
        &private_key.primary_key, // Sign with the primary (NOTE: This is not always the right key!)
        &Password::empty(),
        HashAlgorithm::Sha256,
        DATA,
    )?;

    // Verify signature with the public key
    let (public_key, _headers) = SignedPublicKey::from_armor_file("key.asc")?;
    sig.verify(&public_key, DATA)?; // Verify with primary key (NOTE: This is not always the right key!)

    Ok(())
}
