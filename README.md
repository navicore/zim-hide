# VVW - WAV Steganography Toolkit

A Rust CLI for embedding and extracting encrypted text and audio in WAV files.

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/vvw`.

## Quick Start

```bash
# Embed a message
vvw encode input.wav -o output.wav --message "Hello, world!"

# Extract the message
vvw decode output.wav
```

## Commands

### encode

Embed text or audio into a WAV file.

```bash
# Basic text embedding
vvw encode input.wav -o output.wav --message "secret message"
vvw encode input.wav -o output.wav --message-file secret.txt

# Embed audio
vvw encode input.wav -o output.wav --audio hidden.wav

# Symmetric encryption (passphrase)
vvw encode input.wav -o output.wav --message "secret" --passphrase "puzzle"

# Asymmetric encryption (public key)
vvw encode input.wav -o output.wav --message "secret" --encrypt-to alice.pub

# Multi-recipient
vvw encode input.wav -o output.wav --message "secret" \
    --encrypt-to alice.pub --encrypt-to bob.pub

# Sign the message
vvw encode input.wav -o output.wav --message "verified" --sign --key my.priv

# Use metadata method (stores in RIFF chunk, not hidden but preserves audio)
vvw encode input.wav -o output.wav --message "data" --method metadata

# LSB options
vvw encode input.wav -o output.wav --message "data" --bits 2 --channels left
```

### decode

Extract text content from a WAV file.

```bash
# Basic extraction
vvw decode output.wav

# Decrypt with passphrase
vvw decode output.wav --passphrase "puzzle"

# Decrypt with private key
vvw decode output.wav --key my.priv

# Verify signature
vvw decode output.wav --verify alice.pub
```

### play

Extract and play embedded audio.

```bash
# Play embedded audio
vvw play output.wav

# Extract to file instead
vvw play output.wav --extract-to recovered.wav

# With decryption
vvw play output.wav --passphrase "puzzle"
vvw play output.wav --key my.priv
```

### keygen

Generate a keypair for encryption and signing.

```bash
# Save to files
vvw keygen --output mykey
# Creates: mykey.pub and mykey.priv

# Output to stdout
vvw keygen
```

### inspect

Show embedded content metadata without decrypting.

```bash
vvw inspect output.wav

# Example output:
# VVW Embedded Data
# =================
#
# Method: LSB (Least Significant Bit)
# Content: text
# Payload size: 83 bytes (encrypted)
# Encryption: symmetric (passphrase)
# Signed: no
#
# Total embedded: 93 bytes
# Capacity used: 0.8%
# Available: 11018 bytes
```

## Steganography Methods

### LSB (Least Significant Bit)

Default method. Modifies the least significant bits of audio samples to embed data. With 1 bit per sample (default), the modification is inaudible (-96dB for 16-bit audio).

Options:
- `--bits 1-4` - Bits per sample (higher = more capacity, more audible)
- `--channels left|right|both` - Which channels to use

### Metadata

Stores data in a custom RIFF chunk (`vvwD`). Does not modify audio samples at all, but the chunk is visible to tools like `ffprobe`. Useful when audio fidelity is critical.

## Cryptography

- **Symmetric**: Argon2id key derivation + ChaCha20-Poly1305
- **Asymmetric**: X25519 key exchange + XChaCha20-Poly1305
- **Signatures**: Ed25519

Key files use a PEM-like format:
```
-----BEGIN VVW PUBLIC KEY-----
<base64 encoded key>
-----END VVW PUBLIC KEY-----
```

## Embedded Data Format

```
[4 bytes]  Magic: "VVW\x01"
[1 byte]   Flags (text, audio, signed, symmetric, asymmetric)
[1 byte]   Method (0=LSB, 1=metadata, 2=spread)
[4 bytes]  Payload length
[N bytes]  Payload (encrypted if applicable)
[64 bytes] Signature (if signed)
```

## License

MIT
