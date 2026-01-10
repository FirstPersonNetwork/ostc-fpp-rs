use bip39::Mnemonic;
use rand::RngCore;
use zeroize::Zeroize;

#[derive(Clone, Debug)]
pub struct BIP32_39 {
    pub mnemonic: Mnemonic,
}

impl Default for BIP32_39 {
    fn default() -> Self {
        // Create 256 bits of entropy
        let mut entropy = [0u8; 32];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut entropy);

        let mnemonic = match Mnemonic::from_entropy(&entropy) {
            Ok(mnemonic) => {
                entropy.zeroize(); // Clear entropy from memory
                mnemonic
            }
            Err(e) => {
                panic!("Error creating BIP39 mnemonic from entropy: {e}");
            }
        };

        BIP32_39 { mnemonic }
    }
}

impl BIP32_39 {
    /// Returns a string representing the mnemonic words representing the BIP32 seed
    pub fn get_mnemonic_string(&self) -> String {
        self.mnemonic.words().collect::<Vec<&str>>().join(" ")
    }
}
