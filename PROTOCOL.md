# Zimhide Protocol Specification

**Version:** 1
**Status:** Stable
**Last Updated:** 2026-02

This document describes the binary formats and cryptographic protocols used by zimhide. It is intended to enable independent implementations.

## Table of Contents

1. [Overview](#overview)
2. [Embedded Data Format](#embedded-data-format)
3. [Steganography Methods](#steganography-methods)
4. [Payload Format](#payload-format)
5. [Encryption](#encryption)
6. [Signatures](#signatures)
7. [Audio Compression](#audio-compression)
8. [Key File Format](#key-file-format)

---

## Overview

Zimhide embeds encrypted data into WAV audio files using steganography. The embedded data consists of:

1. A **header** identifying the format and options
2. A **payload** containing text and/or audio (optionally encrypted)
3. An optional **signature** for authenticity verification

All multi-byte integers are little-endian unless otherwise noted.

---

## Embedded Data Format

The complete embedded data structure:

```
┌─────────────────────────────────────────────────────────────┐
│                         HEADER                              │
├──────────┬─────────┬───────┬────────┬───────────────────────┤
│  Magic   │ Version │ Flags │ Method │    Payload Length     │
│ 4 bytes  │ 1 byte  │ 1 byte│ 1 byte │       4 bytes         │
├──────────┴─────────┴───────┴────────┴───────────────────────┤
│                         PAYLOAD                             │
│                    (variable length)                        │
├─────────────────────────────────────────────────────────────┤
│                    SIGNATURE (optional)                     │
│                        64 bytes                             │
└─────────────────────────────────────────────────────────────┘
```

### Header (11 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | Magic | ASCII `ZIMH` (0x5A 0x49 0x4D 0x48) |
| 4 | 1 | Version | Protocol version (currently `1`) |
| 5 | 1 | Flags | Bit flags (see below) |
| 6 | 1 | Method | Steganography method ID |
| 7 | 4 | Payload Length | Length of payload in bytes (u32 LE) |

### Flags Byte

| Bit | Mask | Meaning |
|-----|------|---------|
| 0 | 0x01 | Has text content |
| 1 | 0x02 | Has audio content |
| 2 | 0x04 | Payload is signed |
| 3 | 0x08 | Symmetric encryption (passphrase) |
| 4 | 0x10 | Asymmetric encryption (public key) |
| 5-7 | — | Reserved (must be 0) |

### Method ID

| Value | Method |
|-------|--------|
| 0 | LSB (Least Significant Bit) |
| 1 | Metadata (RIFF chunk) |
| 2 | Spread spectrum (reserved) |

### Signature

If the `is_signed` flag (bit 2) is set, a 64-byte Ed25519 signature immediately follows the payload. The signature is computed over the **payload bytes** (after encryption, if applicable).

---

## Steganography Methods

### Method 0: LSB (Least Significant Bit)

Data is embedded in the least significant bits of audio samples.

#### LSB Embedding Format

The LSB method prepends a 4-byte length before the embedded data:

```
┌────────────────┬──────────────────────────┐
│  Data Length   │         Data             │
│   4 bytes LE   │    (variable length)     │
└────────────────┴──────────────────────────┘
```

#### Bit Packing

Bits are packed LSB-first into audio samples:

1. For each audio sample (in file order):
   - Extract `bits_per_sample` bits from the data stream
   - Replace the lowest `bits_per_sample` bits of the sample
2. Default is 1 bit per sample
3. Supports 1-4 bits per sample (configurable)

#### Channel Selection

- **Both**: Use all samples (default)
- **Left**: Use only samples at even indices (0, 2, 4, ...)
- **Right**: Use only samples at odd indices (1, 3, 5, ...)

For mono files, all samples are used regardless of channel selection.

#### Capacity Calculation

```
usable_samples = total_samples × channel_factor
capacity_bits = usable_samples × bits_per_sample
capacity_bytes = capacity_bits / 8 - 4  (subtract length prefix)
```

Where `channel_factor` is 1.0 for "both", 0.5 for "left" or "right".

### Method 1: Metadata (RIFF Chunk)

Data is stored in a custom RIFF chunk within the WAV file structure.

#### Chunk Format

```
┌────────────┬──────────────┬─────────────────┬─────────┐
│  Chunk ID  │  Chunk Size  │      Data       │ Padding │
│  "zimH"    │   4 bytes LE │  (variable)     │ 0 or 1  │
└────────────┴──────────────┴─────────────────┴─────────┘
```

- **Chunk ID**: ASCII `zimH` (0x7A 0x69 0x6D 0x48)
- **Chunk Size**: Data length in bytes (u32 LE), excluding padding
- **Data**: The embedded data (header + payload + optional signature)
- **Padding**: One zero byte if data length is odd (RIFF word alignment)

The chunk is appended after existing WAV chunks. The RIFF file size field is updated accordingly.

---

## Payload Format

The decrypted payload structure (when both text and audio are present):

```
┌────────────────┬──────────────────┬────────────────┬───────────────┐
│  Text Length   │      Text        │  Audio Length  │     Audio     │
│   4 bytes LE   │   (UTF-8 bytes)  │   4 bytes LE   │    (bytes)    │
└────────────────┴──────────────────┴────────────────┴───────────────┘
```

- **Text Length**: Length of text in bytes (0 if no text)
- **Text**: UTF-8 encoded string
- **Audio Length**: Length of audio data in bytes (0 if no audio)
- **Audio**: Compressed audio (see [Audio Compression](#audio-compression))

Both length fields are always present, set to 0 if the corresponding content is absent.

---

## Encryption

### Symmetric Encryption (Passphrase)

Uses Argon2id for key derivation and ChaCha20-Poly1305 for encryption.

#### Ciphertext Format

```
┌────────────┬──────────────┬───────────┬─────────────────────────────┐
│ Salt Length│    Salt      │   Nonce   │         Ciphertext          │
│   1 byte   │  (variable)  │  12 bytes │   (plaintext + 16 auth tag) │
└────────────┴──────────────┴───────────┴─────────────────────────────┘
```

#### Key Derivation (Argon2id)

- **Algorithm**: Argon2id (default parameters from `argon2` crate)
- **Salt**: Random, encoded as base64 PHC string (typically 22 chars)
- **Output**: 32 bytes (256 bits)

#### Encryption (ChaCha20-Poly1305)

- **Key**: 32 bytes from Argon2id
- **Nonce**: 12 bytes, randomly generated
- **Auth Tag**: 16 bytes, appended to ciphertext

### Asymmetric Encryption (Public Key)

Uses X25519 for key exchange and XChaCha20-Poly1305 for encryption.

#### Ciphertext Format

```
┌──────────────────┬─────────────────────────────────┬──────────────┬─────────────┐
│ Recipient Count  │     Per-Recipient Blocks        │ Payload Nonce│  Ciphertext │
│     1 byte       │   (104 bytes × count)           │   24 bytes   │  (variable) │
└──────────────────┴─────────────────────────────────┴──────────────┴─────────────┘
```

#### Per-Recipient Block (104 bytes)

```
┌─────────────────────┬────────────────┬────────────────────┐
│  Ephemeral Public   │   Key Nonce    │    Wrapped Key     │
│      32 bytes       │    24 bytes    │      48 bytes      │
└─────────────────────┴────────────────┴────────────────────┘
```

- **Ephemeral Public**: X25519 public key for this recipient
- **Key Nonce**: Nonce for key wrapping
- **Wrapped Key**: Symmetric key encrypted with XChaCha20-Poly1305 (32 + 16 bytes)

#### Key Exchange Protocol

1. Generate random 32-byte symmetric key
2. For each recipient:
   a. Generate ephemeral X25519 keypair
   b. Compute shared secret: `ECDH(ephemeral_private, recipient_public)`
   c. Derive key encryption key (KEK) from shared secret
   d. Encrypt symmetric key with KEK using XChaCha20-Poly1305
3. Encrypt payload with symmetric key using XChaCha20-Poly1305

#### Key Encryption Key Derivation

The KEK is derived by hashing the shared secret with a domain separator:

```
for i in 0..4:
    hasher = new DefaultHasher()
    hash(b"zimhide-key-derivation")
    hash(i)
    hash(shared_secret)
    kek[i*8..(i+1)*8] = hasher.finish().to_le_bytes()
```

*Note: This uses Rust's DefaultHasher (SipHash-1-3). A future version may use HKDF.*

---

## Signatures

Ed25519 signatures provide authenticity verification.

### What is Signed

The signature is computed over the **payload bytes**. If encryption is used, this means the encrypted payload (ciphertext), not the plaintext.

### Signature Format

- **Algorithm**: Ed25519
- **Size**: 64 bytes
- **Location**: Immediately after payload (when `is_signed` flag is set)

### Verification

1. Read header to determine payload length and `is_signed` flag
2. Read payload bytes
3. Read 64-byte signature
4. Verify signature against payload using signer's Ed25519 public key

---

## Audio Compression

Embedded audio can be stored in two formats depending on build configuration.

### With Opus Compression (Default)

Audio is compressed using Opus codec at 48kHz.

#### Requirements

- Sample rate: 48000 Hz (required)
- Bit depth: 16-bit (required)
- Channels: Mono or stereo

#### Compressed Format

```
┌──────────────┬──────────┬─────────────┬────────────────────────────┐
│ Sample Rate  │ Channels │ Frame Count │         Frames             │
│  4 bytes LE  │ 2 bytes  │  2 bytes LE │       (variable)           │
└──────────────┴──────────┴─────────────┴────────────────────────────┘
```

**Header (8 bytes):**

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | Sample Rate | Always 48000 (preserved for output) |
| 4 | 2 | Channels | 1 = mono, 2 = stereo |
| 6 | 2 | Frame Count | Number of Opus frames |

**Each Frame:**

```
┌─────────────┬─────────────────┐
│ Frame Size  │   Opus Packet   │
│  2 bytes LE │   (variable)    │
└─────────────┴─────────────────┘
```

#### Opus Parameters

- **Frame duration**: 20ms (960 samples at 48kHz)
- **Application**: Audio
- **Bitrate**: 64 kbps (mono), 96 kbps (stereo)

### Without Opus (Raw WAV)

When built with `--no-default-features`, audio is stored as raw WAV file bytes without any processing.

---

## Key File Format

Keys use a PEM-like format with base64 encoding.

### Private Key File (.priv)

```
-----BEGIN ZIMHIDE PRIVATE KEY-----
<base64 encoded 64 bytes>
-----END ZIMHIDE PRIVATE KEY-----
```

**Decoded content (64 bytes):**

| Offset | Size | Field |
|--------|------|-------|
| 0 | 32 | Ed25519 signing key (seed) |
| 32 | 32 | X25519 private key |

File permissions should be set to 0600 (owner read/write only).

### Public Key File (.pub)

```
-----BEGIN ZIMHIDE PUBLIC KEY-----
<base64 encoded 64 bytes>
-----END ZIMHIDE PUBLIC KEY-----
```

**Decoded content (64 bytes):**

| Offset | Size | Field |
|--------|------|-------|
| 0 | 32 | Ed25519 verifying key |
| 32 | 32 | X25519 public key |

### Key Fingerprint

The fingerprint is the first 6 bytes of the Ed25519 public key, encoded as 12 hex characters.

---

## Implementation Notes

### Byte Order

All multi-byte integers are little-endian (LE).

### Error Handling

Implementations should reject:
- Unknown magic bytes
- Unsupported version numbers
- Invalid flag combinations
- Truncated data
- Authentication failures (wrong passphrase, wrong key, invalid signature)

### Security Considerations

1. **Key derivation**: Argon2id parameters should match the defaults in the `argon2` crate
2. **Nonces**: Must be randomly generated; never reuse with the same key
3. **Signatures**: Sign ciphertext, not plaintext (sign-then-encrypt is not used)
4. **Steganography**: LSB embedding is detectable by statistical analysis; metadata embedding is trivially visible

---

## Version History

| Version | Changes |
|---------|---------|
| 1 | Initial version with version byte |

---

## References

- [Opus Codec](https://opus-codec.org/)
- [RFC 8439 - ChaCha20-Poly1305](https://tools.ietf.org/html/rfc8439)
- [Argon2](https://github.com/P-H-C/phc-winner-argon2)
- [Ed25519](https://ed25519.cr.yp.to/)
- [X25519](https://cr.yp.to/ecdh.html)
- [WAV File Format](http://soundfile.sapp.org/doc/WaveFormat/)
