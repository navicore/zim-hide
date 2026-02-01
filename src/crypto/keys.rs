use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;
use x25519_dalek::{PublicKey as X25519Public, StaticSecret as X25519Secret};

const PRIVATE_KEY_HEADER: &str = "-----BEGIN VVW PRIVATE KEY-----";
const PRIVATE_KEY_FOOTER: &str = "-----END VVW PRIVATE KEY-----";
const PUBLIC_KEY_HEADER: &str = "-----BEGIN VVW PUBLIC KEY-----";
const PUBLIC_KEY_FOOTER: &str = "-----END VVW PUBLIC KEY-----";

#[derive(Clone)]
pub struct PrivateKey {
    pub ed25519: SigningKey,
    pub x25519: X25519Secret,
}

#[derive(Clone)]
pub struct PublicKey {
    pub ed25519: VerifyingKey,
    pub x25519: X25519Public,
}

pub struct Keypair {
    pub private: PrivateKey,
    pub public: PublicKey,
}

impl Keypair {
    pub fn generate() -> Self {
        let ed25519_signing = SigningKey::generate(&mut OsRng);
        let ed25519_verifying = ed25519_signing.verifying_key();

        // Derive X25519 key from Ed25519 seed
        let x25519_secret = X25519Secret::random_from_rng(OsRng);
        let x25519_public = X25519Public::from(&x25519_secret);

        Self {
            private: PrivateKey {
                ed25519: ed25519_signing,
                x25519: x25519_secret,
            },
            public: PublicKey {
                ed25519: ed25519_verifying,
                x25519: x25519_public,
            },
        }
    }

    pub fn save(&self, base_path: &Path) -> Result<()> {
        let priv_path = base_path.with_extension("priv");
        let pub_path = base_path.with_extension("pub");

        self.private.save(&priv_path)?;
        self.public.save(&pub_path)?;

        Ok(())
    }
}

impl PrivateKey {
    pub fn save(&self, path: &Path) -> Result<()> {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(self.ed25519.as_bytes());
        bytes.extend_from_slice(self.x25519.as_bytes());

        let encoded = BASE64.encode(&bytes);
        let content = format!("{}\n{}\n{}\n", PRIVATE_KEY_HEADER, encoded, PRIVATE_KEY_FOOTER);

        fs::write(path, content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms)?;
        }

        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let content = content.trim();

        if !content.starts_with(PRIVATE_KEY_HEADER) || !content.ends_with(PRIVATE_KEY_FOOTER) {
            return Err(anyhow!("Invalid private key format"));
        }

        let encoded = content
            .strip_prefix(PRIVATE_KEY_HEADER)
            .unwrap()
            .strip_suffix(PRIVATE_KEY_FOOTER)
            .unwrap()
            .trim();

        let bytes = BASE64.decode(encoded)?;
        if bytes.len() != 64 {
            return Err(anyhow!(
                "Invalid private key length: expected 64 bytes, got {}",
                bytes.len()
            ));
        }

        let ed25519_bytes: [u8; 32] = bytes[0..32].try_into().unwrap();
        let x25519_bytes: [u8; 32] = bytes[32..64].try_into().unwrap();

        let ed25519 = SigningKey::from_bytes(&ed25519_bytes);
        let x25519 = X25519Secret::from(x25519_bytes);

        Ok(Self { ed25519, x25519 })
    }

    #[allow(dead_code)]
    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            ed25519: self.ed25519.verifying_key(),
            x25519: X25519Public::from(&self.x25519),
        }
    }
}

impl PublicKey {
    pub fn save(&self, path: &Path) -> Result<()> {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(self.ed25519.as_bytes());
        bytes.extend_from_slice(self.x25519.as_bytes());

        let encoded = BASE64.encode(&bytes);
        let content = format!("{}\n{}\n{}\n", PUBLIC_KEY_HEADER, encoded, PUBLIC_KEY_FOOTER);

        fs::write(path, content)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let content = content.trim();

        if !content.starts_with(PUBLIC_KEY_HEADER) || !content.ends_with(PUBLIC_KEY_FOOTER) {
            return Err(anyhow!("Invalid public key format"));
        }

        let encoded = content
            .strip_prefix(PUBLIC_KEY_HEADER)
            .unwrap()
            .strip_suffix(PUBLIC_KEY_FOOTER)
            .unwrap()
            .trim();

        let bytes = BASE64.decode(encoded)?;
        if bytes.len() != 64 {
            return Err(anyhow!(
                "Invalid public key length: expected 64 bytes, got {}",
                bytes.len()
            ));
        }

        let ed25519_bytes: [u8; 32] = bytes[0..32].try_into().unwrap();
        let x25519_bytes: [u8; 32] = bytes[32..64].try_into().unwrap();

        let ed25519 =
            VerifyingKey::from_bytes(&ed25519_bytes).map_err(|e| anyhow!("Invalid Ed25519 public key: {}", e))?;
        let x25519 = X25519Public::from(x25519_bytes);

        Ok(Self { ed25519, x25519 })
    }

    pub fn fingerprint(&self) -> String {
        use std::fmt::Write;
        let mut hasher = [0u8; 6];
        let bytes = self.ed25519.as_bytes();
        for (i, b) in bytes.iter().take(6).enumerate() {
            hasher[i] = *b;
        }
        let mut s = String::with_capacity(12);
        for b in hasher {
            write!(s, "{:02x}", b).unwrap();
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_keypair_generation_and_save_load() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().join("test");

        let keypair = Keypair::generate();
        keypair.save(&base_path).unwrap();

        let loaded_priv = PrivateKey::load(&base_path.with_extension("priv")).unwrap();
        let loaded_pub = PublicKey::load(&base_path.with_extension("pub")).unwrap();

        assert_eq!(
            keypair.private.ed25519.as_bytes(),
            loaded_priv.ed25519.as_bytes()
        );
        assert_eq!(
            keypair.public.ed25519.as_bytes(),
            loaded_pub.ed25519.as_bytes()
        );
    }

    #[test]
    fn test_fingerprint() {
        let keypair = Keypair::generate();
        let fp = keypair.public.fingerprint();
        assert_eq!(fp.len(), 12);
    }
}
