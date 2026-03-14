use std::sync::RwLock;

use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD, Engine};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Key, XChaCha20Poly1305, XNonce,
};
use zeroize::Zeroize;

use super::{Cipher, CipherKind, EncryptedPayload};
use crate::core::error::{EnvlyError, Result};

const ARGON2_MEMORY: u32 = 1 << 16; // 64 MiB
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;
const ARGON2_KEY_SIZE: usize = 32;

pub struct PassphraseCipher {
    passphrase: String,
    cached_key: RwLock<Option<[u8; 32]>>,
    cached_salt: RwLock<Option<[u8; 16]>>,
}

impl PassphraseCipher {
    pub fn new(passphrase: String) -> Self {
        Self {
            passphrase,
            cached_key: RwLock::new(None),
            cached_salt: RwLock::new(None),
        }
    }

    fn derive_key(&self, salt: &[u8]) -> Result<[u8; 32]> {
        let mut key = [0u8; ARGON2_KEY_SIZE];
        let params = Params::new(
            ARGON2_MEMORY,
            ARGON2_ITERATIONS,
            ARGON2_PARALLELISM,
            Some(ARGON2_KEY_SIZE),
        )
        .map_err(|e| EnvlyError::Crypto(e.to_string()))?;
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
            .hash_password_into(self.passphrase.as_bytes(), salt, &mut key)
            .map_err(|e| EnvlyError::Crypto(e.to_string()))?;
        Ok(key)
    }
}

impl Drop for PassphraseCipher {
    fn drop(&mut self) {
        self.passphrase.zeroize();
        if let Ok(mut guard) = self.cached_key.write() {
            if let Some(ref mut key) = *guard {
                key.zeroize();
            }
        }
    }
}

impl Cipher for PassphraseCipher {
    fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedPayload> {
        let (key, salt) = {
            let cached = self.cached_key.read().unwrap();
            let cached_salt = self.cached_salt.read().unwrap();
            match (*cached, *cached_salt) {
                (Some(k), Some(s)) => (k, s),
                _ => {
                    drop(cached);
                    drop(cached_salt);
                    let salt: [u8; 16] = rand_bytes();
                    let key = self.derive_key(&salt)?;
                    *self.cached_key.write().unwrap() = Some(key);
                    *self.cached_salt.write().unwrap() = Some(salt);
                    (key, salt)
                }
            }
        };

        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&key));
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| EnvlyError::Crypto(e.to_string()))?;

        Ok(EncryptedPayload {
            ciphertext: STANDARD.encode(&ciphertext),
            nonce: STANDARD.encode(nonce.as_slice()),
            salt: Some(STANDARD.encode(salt)),
        })
    }

    fn decrypt(&self, payload: &EncryptedPayload) -> Result<Vec<u8>> {
        let salt_b64 = payload.salt.as_deref().ok_or_else(|| {
            EnvlyError::Crypto("Missing salt in passphrase-encrypted payload".into())
        })?;
        let salt = STANDARD
            .decode(salt_b64)
            .map_err(|e| EnvlyError::Crypto(e.to_string()))?;
        let key = self.derive_key(&salt)?;

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

        let cipher = XChaCha20Poly1305::new(Key::from_slice(&key));
        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).map_err(|_| {
            EnvlyError::Crypto("Decryption failed: wrong passphrase or corrupted data".into())
        })?;

        if salt.len() != 16 {
            return Err(EnvlyError::Crypto(
                "Invalid salt length: expected 16 bytes".into(),
            ));
        }

        // Cache the derived key so subsequent encrypts skip Argon2id
        *self.cached_key.write().unwrap() = Some(key);
        let mut salt_arr = [0u8; 16];
        salt_arr.copy_from_slice(&salt);
        *self.cached_salt.write().unwrap() = Some(salt_arr);

        Ok(plaintext)
    }

    fn kind(&self) -> CipherKind {
        CipherKind::Passphrase
    }
}

fn rand_bytes<const N: usize>() -> [u8; N] {
    use chacha20poly1305::aead::rand_core::RngCore;
    use chacha20poly1305::aead::OsRng;
    let mut buf = [0u8; N];
    OsRng.fill_bytes(&mut buf);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let cipher = PassphraseCipher::new("my-strong-passphrase".into());
        let plaintext = b"hello world secret data";
        let payload = cipher.encrypt(plaintext).unwrap();
        let decrypted = cipher.decrypt(&payload).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_passphrase_fails() {
        let cipher1 = PassphraseCipher::new("correct-password".into());
        let cipher2 = PassphraseCipher::new("wrong-password".into());
        let payload = cipher1.encrypt(b"secret").unwrap();
        let result = cipher2.decrypt(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn corrupted_ciphertext_fails() {
        let cipher = PassphraseCipher::new("password".into());
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
    fn empty_plaintext_round_trip() {
        let cipher = PassphraseCipher::new("password".into());
        let payload = cipher.encrypt(b"").unwrap();
        let decrypted = cipher.decrypt(&payload).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn large_plaintext_round_trip() {
        let cipher = PassphraseCipher::new("password".into());
        let plaintext = vec![0xABu8; 100_000];
        let payload = cipher.encrypt(&plaintext).unwrap();
        let decrypted = cipher.decrypt(&payload).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn cached_key_used_after_decrypt() {
        let cipher = PassphraseCipher::new("password".into());
        assert!(cipher.cached_key.read().unwrap().is_none());

        let payload = cipher.encrypt(b"first").unwrap();
        let _ = cipher.decrypt(&payload).unwrap();

        assert!(cipher.cached_key.read().unwrap().is_some());
        assert!(cipher.cached_salt.read().unwrap().is_some());

        // Subsequent encrypt should use cached key (fast path)
        let payload2 = cipher.encrypt(b"second").unwrap();
        let decrypted = cipher.decrypt(&payload2).unwrap();
        assert_eq!(decrypted, b"second");
    }

    // --- New passphrase crypto tests ---

    #[test]
    fn decrypt_with_invalid_salt_length_fails() {
        let cipher = PassphraseCipher::new("password".into());
        let payload = cipher.encrypt(b"data").unwrap();

        // Tamper with salt to have wrong length
        let mut tampered = payload;
        tampered.salt = Some(STANDARD.encode([0u8; 8])); // 8 bytes instead of 16
        let result = cipher.decrypt(&tampered);
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_with_missing_salt_fails() {
        let cipher = PassphraseCipher::new("password".into());
        let mut payload = cipher.encrypt(b"data").unwrap();
        payload.salt = None;
        let result = cipher.decrypt(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_with_corrupted_nonce_fails() {
        let cipher = PassphraseCipher::new("password".into());
        let mut payload = cipher.encrypt(b"data").unwrap();
        // Set nonce to wrong length
        payload.nonce = STANDARD.encode([0u8; 8]);
        let result = cipher.decrypt(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn encrypt_produces_unique_nonces() {
        let cipher = PassphraseCipher::new("password".into());
        let p1 = cipher.encrypt(b"data").unwrap();
        let p2 = cipher.encrypt(b"data").unwrap();
        assert_ne!(p1.nonce, p2.nonce);
    }
}
