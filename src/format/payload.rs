use anyhow::{anyhow, Result};

pub const MAGIC: &[u8; 4] = b"VVW\x01";
pub const SIGNATURE_SIZE: usize = 64;

#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub has_text: bool,
    pub has_audio: bool,
    pub is_signed: bool,
    pub symmetric_encryption: bool,
    pub asymmetric_encryption: bool,
}

impl Flags {
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.has_text {
            byte |= 1 << 0;
        }
        if self.has_audio {
            byte |= 1 << 1;
        }
        if self.is_signed {
            byte |= 1 << 2;
        }
        if self.symmetric_encryption {
            byte |= 1 << 3;
        }
        if self.asymmetric_encryption {
            byte |= 1 << 4;
        }
        byte
    }

    pub fn from_byte(byte: u8) -> Self {
        Self {
            has_text: (byte & (1 << 0)) != 0,
            has_audio: (byte & (1 << 1)) != 0,
            is_signed: (byte & (1 << 2)) != 0,
            symmetric_encryption: (byte & (1 << 3)) != 0,
            asymmetric_encryption: (byte & (1 << 4)) != 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StegoMethodId {
    Lsb = 0,
    Metadata = 1,
    Spread = 2,
}

impl TryFrom<u8> for StegoMethodId {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Lsb),
            1 => Ok(Self::Metadata),
            2 => Ok(Self::Spread),
            _ => Err(anyhow!("Unknown steganography method: {}", value)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Header {
    pub flags: Flags,
    pub method: StegoMethodId,
    pub payload_length: u32,
}

impl Header {
    pub const SIZE: usize = 4 + 1 + 1 + 4; // magic + flags + method + length

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.extend_from_slice(MAGIC);
        bytes.push(self.flags.to_byte());
        bytes.push(self.method as u8);
        bytes.extend_from_slice(&self.payload_length.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(anyhow!(
                "Header too short: expected {} bytes, got {}",
                Self::SIZE,
                bytes.len()
            ));
        }

        if &bytes[0..4] != MAGIC {
            return Err(anyhow!("Invalid magic bytes - not a VVW file"));
        }

        let flags = Flags::from_byte(bytes[4]);
        let method = StegoMethodId::try_from(bytes[5])?;
        let payload_length = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);

        Ok(Self {
            flags,
            method,
            payload_length,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Payload {
    pub text: Option<String>,
    pub audio: Option<Vec<u8>>,
}

impl Payload {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Text length and content
        if let Some(ref text) = self.text {
            let text_bytes = text.as_bytes();
            bytes.extend_from_slice(&(text_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(text_bytes);
        } else {
            bytes.extend_from_slice(&0u32.to_le_bytes());
        }

        // Audio length and content
        if let Some(ref audio) = self.audio {
            bytes.extend_from_slice(&(audio.len() as u32).to_le_bytes());
            bytes.extend_from_slice(audio);
        } else {
            bytes.extend_from_slice(&0u32.to_le_bytes());
        }

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 {
            return Err(anyhow!("Payload too short"));
        }

        let mut offset = 0;

        // Read text
        let text_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        let text = if text_len > 0 {
            if offset + text_len > bytes.len() {
                return Err(anyhow!("Payload truncated: text extends beyond data"));
            }
            let text_bytes = &bytes[offset..offset + text_len];
            offset += text_len;
            Some(String::from_utf8(text_bytes.to_vec())?)
        } else {
            None
        };

        // Read audio
        if offset + 4 > bytes.len() {
            return Err(anyhow!("Payload truncated: missing audio length"));
        }
        let audio_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        let audio = if audio_len > 0 {
            if offset + audio_len > bytes.len() {
                return Err(anyhow!("Payload truncated: audio extends beyond data"));
            }
            Some(bytes[offset..offset + audio_len].to_vec())
        } else {
            None
        };

        Ok(Self { text, audio })
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddedData {
    pub header: Header,
    pub payload: Vec<u8>, // Raw payload bytes (may be encrypted)
    pub signature: Option<[u8; SIGNATURE_SIZE]>,
}

impl EmbeddedData {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.payload);
        if let Some(sig) = &self.signature {
            bytes.extend_from_slice(sig);
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let header = Header::from_bytes(bytes)?;
        let payload_start = Header::SIZE;
        let payload_end = payload_start + header.payload_length as usize;

        if bytes.len() < payload_end {
            return Err(anyhow!("Data truncated: payload extends beyond data"));
        }

        let payload = bytes[payload_start..payload_end].to_vec();

        let signature = if header.flags.is_signed {
            let sig_start = payload_end;
            let sig_end = sig_start + SIGNATURE_SIZE;
            if bytes.len() < sig_end {
                return Err(anyhow!("Data truncated: signature extends beyond data"));
            }
            let mut sig = [0u8; SIGNATURE_SIZE];
            sig.copy_from_slice(&bytes[sig_start..sig_end]);
            Some(sig)
        } else {
            None
        };

        Ok(Self {
            header,
            payload,
            signature,
        })
    }

    pub fn total_size(&self) -> usize {
        Header::SIZE
            + self.payload.len()
            + if self.signature.is_some() {
                SIGNATURE_SIZE
            } else {
                0
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flags_roundtrip() {
        let flags = Flags {
            has_text: true,
            has_audio: false,
            is_signed: true,
            symmetric_encryption: false,
            asymmetric_encryption: true,
        };
        let byte = flags.to_byte();
        let decoded = Flags::from_byte(byte);
        assert_eq!(flags.has_text, decoded.has_text);
        assert_eq!(flags.has_audio, decoded.has_audio);
        assert_eq!(flags.is_signed, decoded.is_signed);
        assert_eq!(flags.symmetric_encryption, decoded.symmetric_encryption);
        assert_eq!(flags.asymmetric_encryption, decoded.asymmetric_encryption);
    }

    #[test]
    fn test_payload_roundtrip() {
        let payload = Payload {
            text: Some("Hello, world!".to_string()),
            audio: Some(vec![1, 2, 3, 4, 5]),
        };
        let bytes = payload.to_bytes();
        let decoded = Payload::from_bytes(&bytes).unwrap();
        assert_eq!(payload.text, decoded.text);
        assert_eq!(payload.audio, decoded.audio);
    }

    #[test]
    fn test_header_roundtrip() {
        let header = Header {
            flags: Flags {
                has_text: true,
                ..Default::default()
            },
            method: StegoMethodId::Lsb,
            payload_length: 1234,
        };
        let bytes = header.to_bytes();
        let decoded = Header::from_bytes(&bytes).unwrap();
        assert_eq!(header.payload_length, decoded.payload_length);
    }
}
