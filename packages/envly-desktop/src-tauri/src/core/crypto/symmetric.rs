use base64::{engine::general_purpose::STANDARD, Engine};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Key, XChaCha20Poly1305, XNonce,
};

use super::{Cipher, CipherKind, EncryptedPayload};
use crate::core::error::{EnvlyError, Result};

pub struct SymmetricCipher {
    key: [u8; 32],
}

impl SymmetricCipher {
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    #[allow(dead_code)]
    pub fn from_base64(key_b64: &str) -> Result<Self> {
        let bytes = STANDARD
            .decode(key_b64)
            .map_err(|e| EnvlyError::Crypto(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(EnvlyError::Crypto(
                "Symmetric key must be exactly 32 bytes".into(),
            ));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(Self { key })
    }

    pub fn generate_key() -> [u8; 32] {
        use chacha20poly1305::aead::rand_core::RngCore;
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }

    #[allow(dead_code)]
    pub fn key_as_base64(&self) -> String {
        STANDARD.encode(self.key)
    }
}

impl Drop for SymmetricCipher {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.key.zeroize();
    }
}

impl Cipher for SymmetricCipher {
    fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedPayload> {
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.key));
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| EnvlyError::Crypto(e.to_string()))?;

        Ok(EncryptedPayload {
            ciphertext: STANDARD.encode(&ciphertext),
            nonce: STANDARD.encode(nonce.as_slice()),
            salt: None,
        })
    }

    fn decrypt(&self, payload: &EncryptedPayload) -> Result<Vec<u8>> {
        let nonce_bytes = STANDARD
            .decode(&payload.nonce)
            .map_err(|e| EnvlyError::Crypto(e.to_string()))?;
        if nonce_bytes.len() != 24 {
            return Err(EnvlyError::Crypto(format!(
                "Invalid nonce length: expected 24 bytes, got {}",
                nonce_bytes.len()
            )));
        }
        let nonce = XNonce::from_slice(&nonce_bytes);
        let ciphertext = STANDARD
            .decode(&payload.ciphertext)
            .map_err(|e| EnvlyError::Crypto(e.to_string()))?;

        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.key));
        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).map_err(|_| {
            EnvlyError::Crypto("Decryption failed: wrong key or corrupted data".into())
        })?;

        Ok(plaintext)
    }

    fn kind(&self) -> CipherKind {
        CipherKind::Symmetric
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cipher() -> SymmetricCipher {
        SymmetricCipher::new(SymmetricCipher::generate_key())
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let cipher = test_cipher();
        let plaintext = b"hello world secret data";
        let payload = cipher.encrypt(plaintext).unwrap();
        let decrypted = cipher.decrypt(&payload).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let cipher1 = test_cipher();
        let cipher2 = test_cipher();
        let payload = cipher1.encrypt(b"secret").unwrap();
        let result = cipher2.decrypt(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn corrupted_ciphertext_fails() {
        let cipher = test_cipher();
        let mut payload = cipher.encrypt(b"data").unwrap();
        let mut bytes = STANDARD.decode(&payload.ciphertext).unwrap();
        if let Some(b) = bytes.first_mut() {
            *b ^= 0xFF;
        }
        payload.ciphertext = STANDARD.encode(&bytes);
        let result = cipher.decrypt(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn base64_key_round_trip() {
        let key = SymmetricCipher::generate_key();
        let cipher1 = SymmetricCipher::new(key);
        let b64 = cipher1.key_as_base64();
        let cipher2 = SymmetricCipher::from_base64(&b64).unwrap();

        let payload = cipher1.encrypt(b"test").unwrap();
        let decrypted = cipher2.decrypt(&payload).unwrap();
        assert_eq!(decrypted, b"test");
    }

    #[test]
    fn invalid_base64_key_rejected() {
        let result = SymmetricCipher::from_base64("not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn wrong_length_key_rejected() {
        let short_key = STANDARD.encode([0u8; 16]);
        let result = SymmetricCipher::from_base64(&short_key);
        assert!(result.is_err());
    }

    #[test]
    fn empty_plaintext_round_trip() {
        let cipher = test_cipher();
        let payload = cipher.encrypt(b"").unwrap();
        let decrypted = cipher.decrypt(&payload).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn payload_has_no_salt() {
        let cipher = test_cipher();
        let payload = cipher.encrypt(b"data").unwrap();
        assert!(payload.salt.is_none());
    }

    // --- New symmetric crypto tests ---

    #[test]
    fn decrypt_with_invalid_nonce_length_fails() {
        let cipher = test_cipher();
        let mut payload = cipher.encrypt(b"data").unwrap();
        payload.nonce = STANDARD.encode([0u8; 8]); // wrong length
        let result = cipher.decrypt(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn encrypt_produces_unique_nonces() {
        let cipher = test_cipher();
        let p1 = cipher.encrypt(b"same data").unwrap();
        let p2 = cipher.encrypt(b"same data").unwrap();
        assert_ne!(p1.nonce, p2.nonce);
    }

    #[test]
    fn large_plaintext_round_trip() {
        let cipher = test_cipher();
        let plaintext = vec![0xABu8; 100_000];
        let payload = cipher.encrypt(&plaintext).unwrap();
        let decrypted = cipher.decrypt(&payload).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
