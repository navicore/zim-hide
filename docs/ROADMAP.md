# VVW Roadmap

## Overview

VVW is a WAV steganography toolkit for embedding and extracting encrypted text and audio. This document outlines the implementation phases and current status.

---

## Phase 1: Core Infrastructure ✅

**Status: Complete**

- [x] Project scaffolding with Cargo.toml
- [x] CLI skeleton with clap subcommands
- [x] WAV reading/writing (using `hound`)
- [x] LSB embed/extract (text only, no encryption)
- [x] Basic `encode` and `decode` commands

### Crate Structure

```
vvw/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry, clap setup
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── encode.rs
│   │   ├── decode.rs
│   │   ├── play.rs
│   │   ├── keygen.rs
│   │   └── inspect.rs
│   ├── stego/
│   │   ├── mod.rs
│   │   ├── lsb.rs           # LSB embedding/extraction
│   │   ├── metadata.rs      # RIFF chunk method
│   │   └── traits.rs        # Common interface
│   ├── crypto/
│   │   ├── mod.rs
│   │   ├── keys.rs          # Key generation, serialization
│   │   ├── symmetric.rs     # Passphrase encryption
│   │   ├── asymmetric.rs    # Public key encryption
│   │   └── signing.rs       # Ed25519 signatures
│   ├── wav/
│   │   ├── mod.rs
│   │   ├── reader.rs        # WAV parsing
│   │   └── writer.rs        # WAV writing
│   ├── format/
│   │   ├── mod.rs
│   │   └── payload.rs       # Serialization of embedded data
│   └── audio/
│       ├── mod.rs
│       └── compress.rs      # Audio compression (placeholder)
└── tests/
    └── integration.rs       # End-to-end tests
```

---

## Phase 2: Cryptography ✅

**Status: Complete**

- [x] Key generation and serialization (`keygen`)
- [x] Symmetric encryption (passphrase mode with Argon2id + ChaCha20-Poly1305)
- [x] Asymmetric encryption (X25519 + XChaCha20-Poly1305)
- [x] Ed25519 signing and verification
- [x] Multi-recipient support

### Key Format

**Private key (`*.priv`):**
```
-----BEGIN VVW PRIVATE KEY-----
[base64 of 32-byte Ed25519 seed + 32-byte X25519 private]
-----END VVW PRIVATE KEY-----
```

**Public key (`*.pub`):**
```
-----BEGIN VVW PUBLIC KEY-----
[base64 of 32-byte Ed25519 public + 32-byte X25519 public]
-----END VVW PUBLIC KEY-----
```

### Encryption Envelope

**Symmetric:**
```
[1 byte]   Salt length
[N bytes]  Salt (Argon2 salt string)
[12 bytes] Nonce
[N bytes]  ChaCha20-Poly1305 ciphertext
```

**Asymmetric (per recipient):**
```
[1 byte]   Recipient count
For each recipient:
  [32 bytes] Ephemeral public key (X25519)
  [24 bytes] Nonce
  [48 bytes] Encrypted symmetric key (XChaCha20-Poly1305)
[24 bytes] Payload nonce
[N bytes]  XChaCha20-Poly1305 ciphertext
```

---

## Phase 3: Audio Embedding 🔄

**Status: Partially Complete**

- [x] Basic audio embedding (raw WAV bytes)
- [x] Audio extraction
- [x] `play` command with system player detection
- [ ] Opus compression for embedded audio
- [ ] Opus decompression on extraction

### Planned Implementation

Add Opus codec support for efficient audio embedding:

```toml
# Add to Cargo.toml
opus = "0.3"  # Or audiopus
```

The `compress_audio()` function in `src/audio/compress.rs` should:
1. Read input WAV file
2. Compress to Opus format
3. Return compressed bytes

The `decompress_audio()` function should:
1. Decompress Opus data
2. Write to WAV file

---

## Phase 4: Metadata Method ✅

**Status: Complete**

- [x] RIFF chunk reading/writing
- [x] Metadata steganography method (`--method metadata`)
- [x] Method auto-detection on decode
- [x] `inspect` command

### RIFF Chunk Format

Custom chunk ID: `vvwD` (lowercase = non-standard per RIFF spec)

```
[4 bytes]  Chunk ID: "vvwD"
[4 bytes]  Chunk size (little-endian)
[N bytes]  VVW payload data
[0-1 byte] Padding (if odd size)
```

---

## Phase 5: Polish 🔲

**Status: Not Started**

- [ ] Better error messages with context
- [ ] Progress indicators for large files
- [ ] `--quiet` and `--verbose` flags
- [ ] Shell completions (bash, zsh, fish)
- [ ] Man page generation
- [ ] More comprehensive test coverage
- [ ] Benchmarks

---

## Phase 6: Spread Spectrum (Future) 🔲

**Status: Not Started**

Advanced steganography method that spreads data below the noise floor across frequency bands.

### Planned Features

- [ ] Spread spectrum embedding
- [ ] Survives lossy compression (MP3, AAC)
- [ ] Much lower capacity but more robust
- [ ] Frequency domain manipulation (FFT)

### Technical Approach

1. Convert audio to frequency domain (FFT)
2. Embed bits by modulating specific frequency bands
3. Use pseudo-random sequence for spreading
4. Convert back to time domain (IFFT)

---

## Embedded Data Format

### Header Structure

```
[4 bytes]  Magic: "VVW\x01" (version 1)
[1 byte]   Flags:
           - bit 0: has text
           - bit 1: has audio
           - bit 2: is signed
           - bit 3: symmetric encryption
           - bit 4: asymmetric encryption
[1 byte]   Method: 0=LSB, 1=metadata, 2=spread
[4 bytes]  Payload length (little-endian)
[N bytes]  Payload (encrypted if applicable)
[64 bytes] Signature (Ed25519, if signed)
```

### Payload Structure (before encryption)

```
[4 bytes]  Text length (0 if none)
[N bytes]  Text content (UTF-8)
[4 bytes]  Audio length (0 if none)
[N bytes]  Audio content (Opus-compressed if from WAV)
```

---

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
hound = "3"                    # WAV reading/writing
ed25519-dalek = "2"            # Ed25519 signatures
x25519-dalek = "2"             # X25519 key exchange
chacha20poly1305 = "0.10"      # Authenticated encryption
argon2 = "0.5"                 # Key derivation
base64 = "0.22"                # Key encoding
thiserror = "1"                # Error types
anyhow = "1"                   # Error handling
rand = "0.8"                   # Random number generation
tempfile = "3"                 # Temporary files for playback
which = "7"                    # Find system audio player

# Future: for embedded audio compression
# opus = "0.3"
```

---

## Testing

### Unit Tests

Run with:
```bash
cargo test
```

Covers:
- Payload serialization roundtrips
- Flag encoding/decoding
- Symmetric encryption/decryption
- Asymmetric encryption with single/multiple recipients
- Ed25519 signing and verification
- LSB embed/extract roundtrips
- Metadata chunk roundtrips

### Integration Tests

Located in `tests/integration.rs`. Covers:
- Basic encode/decode cycle
- Symmetric encryption cycle
- Asymmetric encryption cycle
- Signed message verification
- Inspect command output
- Metadata method

### Manual Testing

```bash
# Generate keys
./target/release/vvw keygen --output test

# Basic encode/decode
./target/release/vvw encode test.wav -o out.wav --message "hello"
./target/release/vvw decode out.wav

# Symmetric encryption
./target/release/vvw encode test.wav -o out.wav --message "secret" --passphrase "puzzle"
./target/release/vvw decode out.wav --passphrase "puzzle"

# Asymmetric encryption
./target/release/vvw encode test.wav -o out.wav --message "private" --encrypt-to test.pub
./target/release/vvw decode out.wav --key test.priv

# Signed message
./target/release/vvw encode test.wav -o out.wav --message "verified" --sign --key test.priv
./target/release/vvw decode out.wav --verify test.pub

# Inspect
./target/release/vvw inspect out.wav
```

---

## CLI Reference

### vvw encode

```
Embed text or audio into a WAV file

Usage: vvw encode [OPTIONS] --output <OUTPUT> <INPUT>

Arguments:
  <INPUT>  Input WAV file

Options:
  -o, --output <OUTPUT>        Output WAV file
      --message <MESSAGE>      Text message to embed
      --message-file <FILE>    File containing text message
      --audio <AUDIO>          Audio file to embed
      --passphrase <PASS>      Passphrase for symmetric encryption
      --encrypt-to <PUBKEY>    Public key file (repeatable)
      --sign                   Sign the message
      --key <KEY>              Private key for signing
      --method <METHOD>        lsb (default) or metadata
      --bits <BITS>            Bits per sample, 1-4 (default: 1)
      --channels <CHANNELS>    left, right, or both (default: both)
```

### vvw decode

```
Extract text content from a WAV file

Usage: vvw decode [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input WAV file with embedded data

Options:
      --passphrase <PASS>  Passphrase for symmetric decryption
      --key <KEY>          Private key for asymmetric decryption
      --verify <PUBKEY>    Public key to verify signature
      --bits <BITS>        Bits per sample (default: 1)
      --channels <CHAN>    left, right, or both (default: both)
```

### vvw play

```
Extract and play embedded audio from a WAV file

Usage: vvw play [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input WAV file with embedded audio

Options:
      --passphrase <PASS>    Passphrase for decryption
      --key <KEY>            Private key for decryption
      --extract-to <FILE>    Save to file instead of playing
      --player <PLAYER>      Audio player (default: afplay)
      --bits <BITS>          Bits per sample (default: 1)
      --channels <CHANNELS>  left, right, or both (default: both)
```

### vvw keygen

```
Generate a keypair for encryption and signing

Usage: vvw keygen [OPTIONS]

Options:
  -o, --output <PATH>  Output base path (creates .pub and .priv)
```

### vvw inspect

```
Inspect embedded content metadata without decrypting

Usage: vvw inspect <INPUT>

Arguments:
  <INPUT>  Input WAV file to inspect
```
