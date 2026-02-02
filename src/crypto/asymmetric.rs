use super::keys::{PrivateKey, PublicKey};
use anyhow::{Result, anyhow};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;
use x25519_dalek::{PublicKey as X25519Public, StaticSecret as X25519Secret};

const XNONCE_SIZE: usize = 24;
const KEY_SIZE: usize = 32;
const EPHEMERAL_PUBLIC_SIZE: usize = 32;
const WRAPPED_KEY_SIZE: usize = KEY_SIZE + 16; // Key + auth tag

pub fn encrypt_asymmetric(plaintext: &[u8], recipients: &[PublicKey]) -> Result<Vec<u8>> {
    if recipients.is_empty() {
        return Err(anyhow!("At least one recipient is required"));
    }

    // Generate random symmetric key
    let mut symmetric_key = [0u8; KEY_SIZE];
    rand::thread_rng().fill_bytes(&mut symmetric_key);

    // Encrypt payload with symmetric key
    let mut payload_nonce = [0u8; XNONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut payload_nonce);

    let cipher = XChaCha20Poly1305::new_from_slice(&symmetric_key)
        .map_err(|e| anyhow!("Cipher creation failed: {}", e))?;

    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&payload_nonce), plaintext)
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // Build output
    let mut output = Vec::new();

    // Recipient count
    output.push(recipients.len() as u8);

    // For each recipient, wrap the symmetric key
    for recipient in recipients {
        // Generate ephemeral keypair
        let ephemeral_secret = X25519Secret::random_from_rng(rand::thread_rng());
        let ephemeral_public = X25519Public::from(&ephemeral_secret);

        // Perform key exchange
        let shared_secret = ephemeral_secret.diffie_hellman(&recipient.x25519);

        // Derive encryption key from shared secret
        let key_encryption_key = derive_key_encryption_key(shared_secret.as_bytes());

        // Encrypt symmetric key with derived key
        let mut key_nonce = [0u8; XNONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut key_nonce);

        let key_cipher = XChaCha20Poly1305::new_from_slice(&key_encryption_key)
            .map_err(|e| anyhow!("Key cipher creation failed: {}", e))?;

        let wrapped_key = key_cipher
            .encrypt(XNonce::from_slice(&key_nonce), symmetric_key.as_slice())
            .map_err(|e| anyhow!("Key wrapping failed: {}", e))?;

        // Write: ephemeral public + nonce + wrapped key
        output.extend_from_slice(ephemeral_public.as_bytes());
        output.extend_from_slice(&key_nonce);
        output.extend_from_slice(&wrapped_key);
    }

    // Write payload nonce and ciphertext
    output.extend_from_slice(&payload_nonce);
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

pub fn decrypt_asymmetric(data: &[u8], private_key: &PrivateKey) -> Result<Vec<u8>> {
    if data.is_empty() {
        return Err(anyhow!("Empty ciphertext"));
    }

    let recipient_count = data[0] as usize;
    if recipient_count == 0 {
        return Err(anyhow!("No recipients in ciphertext"));
    }

    // Size per recipient: ephemeral public (32) + nonce (24) + wrapped key (48)
    let per_recipient_size = EPHEMERAL_PUBLIC_SIZE + XNONCE_SIZE + WRAPPED_KEY_SIZE;
    let recipients_section_size = recipient_count * per_recipient_size;
    let header_size = 1 + recipients_section_size;

    if data.len() < header_size + XNONCE_SIZE + 16 {
        return Err(anyhow!("Ciphertext too short"));
    }

    // Try to decrypt the symmetric key with our private key
    let mut symmetric_key: Option<[u8; KEY_SIZE]> = None;

    for i in 0..recipient_count {
        let offset = 1 + i * per_recipient_size;

        let ephemeral_bytes: [u8; 32] = data[offset..offset + 32].try_into().unwrap();
        let ephemeral_public = X25519Public::from(ephemeral_bytes);

        let key_nonce = &data[offset + 32..offset + 32 + XNONCE_SIZE];
        let wrapped_key = &data[offset + 32 + XNONCE_SIZE..offset + per_recipient_size];

        // Perform key exchange
        let shared_secret = private_key.x25519.diffie_hellman(&ephemeral_public);
        let key_encryption_key = derive_key_encryption_key(shared_secret.as_bytes());

        // Try to decrypt
        let key_cipher = XChaCha20Poly1305::new_from_slice(&key_encryption_key)
            .map_err(|e| anyhow!("Key cipher creation failed: {}", e))?;

        if let Ok(decrypted_key) = key_cipher.decrypt(XNonce::from_slice(key_nonce), wrapped_key)
            && decrypted_key.len() == KEY_SIZE
        {
            let mut key = [0u8; KEY_SIZE];
            key.copy_from_slice(&decrypted_key);
            symmetric_key = Some(key);
            break;
        }
    }

    let symmetric_key =
        symmetric_key.ok_or_else(|| anyhow!("Could not decrypt: you may not be a recipient"))?;

    // Decrypt payload
    let payload_nonce = &data[header_size..header_size + XNONCE_SIZE];
    let ciphertext = &data[header_size + XNONCE_SIZE..];

    let cipher = XChaCha20Poly1305::new_from_slice(&symmetric_key)
        .map_err(|e| anyhow!("Cipher creation failed: {}", e))?;

    let plaintext = cipher
        .decrypt(XNonce::from_slice(payload_nonce), ciphertext)
        .map_err(|_| anyhow!("Payload decryption failed: corrupted data"))?;

    Ok(plaintext)
}

pub fn recipient_count(data: &[u8]) -> Option<u8> {
    data.first().copied()
}

fn derive_key_encryption_key(shared_secret: &[u8]) -> [u8; KEY_SIZE] {
    // Simple key derivation: hash the shared secret with a domain separator
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut result = [0u8; KEY_SIZE];

    // Use multiple rounds to fill the key
    for i in 0..4 {
        let mut hasher = DefaultHasher::new();
        b"vvw-key-derivation".hash(&mut hasher);
        i.hash(&mut hasher);
        shared_secret.hash(&mut hasher);
        let hash = hasher.finish();
        result[i * 8..(i + 1) * 8].copy_from_slice(&hash.to_le_bytes());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::Keypair;

    #[test]
    fn test_asymmetric_single_recipient() {
        let keypair = Keypair::generate();
        let plaintext = b"Secret message for one recipient";

        let encrypted =
            encrypt_asymmetric(plaintext, std::slice::from_ref(&keypair.public)).unwrap();
        let decrypted = decrypt_asymmetric(&encrypted, &keypair.private).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_asymmetric_multiple_recipients() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let plaintext = b"Secret message for multiple recipients";

        let recipients = [keypair1.public.clone(), keypair2.public.clone()];
        let encrypted = encrypt_asymmetric(plaintext, &recipients).unwrap();

        // Both recipients should be able to decrypt
        let decrypted1 = decrypt_asymmetric(&encrypted, &keypair1.private).unwrap();
        let decrypted2 = decrypt_asymmetric(&encrypted, &keypair2.private).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted1.as_slice());
        assert_eq!(plaintext.as_slice(), decrypted2.as_slice());
    }

    #[test]
    fn test_non_recipient_cannot_decrypt() {
        let recipient = Keypair::generate();
        let non_recipient = Keypair::generate();
        let plaintext = b"Secret message";

        let encrypted =
            encrypt_asymmetric(plaintext, std::slice::from_ref(&recipient.public)).unwrap();
        let result = decrypt_asymmetric(&encrypted, &non_recipient.private);

        assert!(result.is_err());
    }
}
