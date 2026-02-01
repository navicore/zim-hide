use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;

const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;

pub fn encrypt_symmetric(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    // Generate salt
    let salt = SaltString::generate(&mut OsRng);
    let salt_bytes = salt.as_str().as_bytes();

    // Derive key using Argon2id
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(passphrase.as_bytes(), &salt)
        .map_err(|e| anyhow!("Key derivation failed: {}", e))?;

    let key_bytes = hash.hash.ok_or_else(|| anyhow!("No hash output"))?;
    let key: [u8; KEY_SIZE] = key_bytes.as_bytes()[..KEY_SIZE]
        .try_into()
        .map_err(|_| anyhow!("Invalid key length"))?;

    // Generate nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| anyhow!("Cipher creation failed: {}", e))?;

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // Format: salt_length (1 byte) + salt + nonce + ciphertext
    let mut output = Vec::with_capacity(1 + salt_bytes.len() + NONCE_SIZE + ciphertext.len());
    output.push(salt_bytes.len() as u8);
    output.extend_from_slice(salt_bytes);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

pub fn decrypt_symmetric(data: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    if data.is_empty() {
        return Err(anyhow!("Empty ciphertext"));
    }

    let salt_len = data[0] as usize;
    if data.len() < 1 + salt_len + NONCE_SIZE + 16 {
        // 16 is auth tag
        return Err(anyhow!("Ciphertext too short"));
    }

    let salt_bytes = &data[1..1 + salt_len];
    let salt_str = std::str::from_utf8(salt_bytes)?;
    let salt = SaltString::from_b64(salt_str).map_err(|e| anyhow!("Invalid salt: {}", e))?;

    let nonce_start = 1 + salt_len;
    let nonce_bytes = &data[nonce_start..nonce_start + NONCE_SIZE];
    let nonce = Nonce::from_slice(nonce_bytes);

    let ciphertext = &data[nonce_start + NONCE_SIZE..];

    // Derive key
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(passphrase.as_bytes(), &salt)
        .map_err(|e| anyhow!("Key derivation failed: {}", e))?;

    let key_bytes = hash.hash.ok_or_else(|| anyhow!("No hash output"))?;
    let key: [u8; KEY_SIZE] = key_bytes.as_bytes()[..KEY_SIZE]
        .try_into()
        .map_err(|_| anyhow!("Invalid key length"))?;

    // Decrypt
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| anyhow!("Cipher creation failed: {}", e))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("Decryption failed: wrong passphrase or corrupted data"))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symmetric_roundtrip() {
        let plaintext = b"Hello, world! This is a secret message.";
        let passphrase = "my_secure_passphrase";

        let encrypted = encrypt_symmetric(plaintext, passphrase).unwrap();
        let decrypted = decrypt_symmetric(&encrypted, passphrase).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_wrong_passphrase() {
        let plaintext = b"Secret data";
        let encrypted = encrypt_symmetric(plaintext, "correct").unwrap();
        let result = decrypt_symmetric(&encrypted, "wrong");
        assert!(result.is_err());
    }
}
