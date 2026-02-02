use super::keys::{PrivateKey, PublicKey};
use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature, Signer, Verifier};

pub const SIGNATURE_SIZE: usize = 64;

pub fn sign_message(message: &[u8], private_key: &PrivateKey) -> [u8; SIGNATURE_SIZE] {
    let signature = private_key.ed25519.sign(message);
    signature.to_bytes()
}

pub fn verify_signature(
    message: &[u8],
    signature: &[u8; SIGNATURE_SIZE],
    public_key: &PublicKey,
) -> Result<()> {
    let signature = Signature::from_bytes(signature);

    public_key
        .ed25519
        .verify(message, &signature)
        .map_err(|_| anyhow!("Signature verification failed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::Keypair;

    #[test]
    fn test_sign_and_verify() {
        let keypair = Keypair::generate();
        let message = b"Important message to sign";

        let signature = sign_message(message, &keypair.private);
        verify_signature(message, &signature, &keypair.public).unwrap();
    }

    #[test]
    fn test_wrong_key_verification_fails() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let message = b"Important message";

        let signature = sign_message(message, &keypair1.private);
        let result = verify_signature(message, &signature, &keypair2.public);

        assert!(result.is_err());
    }

    #[test]
    fn test_modified_message_verification_fails() {
        let keypair = Keypair::generate();
        let message = b"Original message";
        let modified = b"Modified message";

        let signature = sign_message(message, &keypair.private);
        let result = verify_signature(modified, &signature, &keypair.public);

        assert!(result.is_err());
    }
}
