# Zim Steganography Toolkit (zimhide)

A Rust CLI for embedding and extracting encrypted text and audio in WAV files. Part of the Zim tool family.

## Installation

### Build

```bash
# With Opus compression (default) - requires libopus
cargo build --release

# Without Opus (no system dependencies, but larger embedded audio)
cargo build --release --no-default-features
```

The binary will be at `target/release/zimhide`.

### Opus Compression (Optional)

By default, embedded audio is compressed with Opus (~10x smaller). This requires libopus:

- **macOS**: `brew install opus`
- **Ubuntu/Debian**: `apt install libopus-dev`
- **Fedora**: `dnf install opus-devel`

If libopus is not available, build with `--no-default-features` to embed raw WAV bytes instead.

## Quick Start

```bash
# Embed a message
zimhide encode input.wav -o output.wav --message "Hello, world!"

# Extract the message
zimhide decode output.wav
```

## Commands

### encode / decode

Embed and extract text from WAV files.

```bash
# Basic text embedding
zimhide encode input.wav -o output.wav --message "secret message"
zimhide decode output.wav

# From a file
zimhide encode input.wav -o output.wav --message-file secret.txt
zimhide decode output.wav

# Symmetric encryption (passphrase)
zimhide encode input.wav -o output.wav --message "secret" --passphrase "puzzle"
zimhide decode output.wav --passphrase "puzzle"

# Asymmetric encryption (public key)
zimhide encode input.wav -o output.wav --message "secret" --encrypt-to alice.pub
zimhide decode output.wav --key alice.priv

# Multi-recipient encryption
zimhide encode input.wav -o output.wav --message "secret" \
    --encrypt-to alice.pub --encrypt-to bob.pub
zimhide decode output.wav --key alice.priv   # Either recipient can decrypt
zimhide decode output.wav --key bob.priv

# Signed message
zimhide encode input.wav -o output.wav --message "verified" --sign --key my.priv
zimhide decode output.wav --verify my.pub

# Metadata method (stores in RIFF chunk, not hidden but preserves audio)
zimhide encode input.wav -o output.wav --message "data" --method metadata
zimhide decode output.wav

# LSB options
zimhide encode input.wav -o output.wav --message "data" --bits 2 --channels left
zimhide decode output.wav
```

### Audio Embedding

Embed audio files inside a carrier WAV. The embedded audio is compressed with Opus (~10x compression).

**Requirements**: Embedded audio must be 48kHz, 16-bit WAV (mono or stereo).

```bash
# Convert audio to required format
ffmpeg -i voice.mp3 -ar 48000 -sample_fmt s16 voice_48k.wav

# Embed audio in carrier WAV
zimhide encode carrier.wav -o output.wav --audio voice_48k.wav --method metadata

# Embed audio with encryption
zimhide encode carrier.wav -o output.wav --audio voice_48k.wav --passphrase "secret"

# Embed both text and audio
zimhide encode carrier.wav -o output.wav --message "Note" --audio voice_48k.wav
```

### play

Extract and play embedded audio.

```bash
# Play embedded audio (requires system audio player)
zimhide play output.wav

# Extract to file instead
zimhide play output.wav --extract-to recovered.wav

# With decryption
zimhide play output.wav --passphrase "secret"
zimhide play output.wav --key my.priv
```

### keygen

Generate a keypair for encryption and signing.

```bash
# Save to files
zimhide keygen --output mykey
# Creates: mykey.pub and mykey.priv

# Output to stdout
zimhide keygen
```

### inspect

Show embedded content metadata without decrypting.

```bash
zimhide inspect output.wav

# Example output:
# Zimhide Embedded Data
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

Stores data in a custom RIFF chunk (`zimH`). Does not modify audio samples at all, but the chunk is visible to tools like `ffprobe`. Useful when audio fidelity is critical.

## Cryptography

- **Symmetric**: Argon2id key derivation + ChaCha20-Poly1305
- **Asymmetric**: X25519 key exchange + XChaCha20-Poly1305
- **Signatures**: Ed25519

Key files use a PEM-like format:
```
-----BEGIN ZIMHIDE PUBLIC KEY-----
<base64 encoded key>
-----END ZIMHIDE PUBLIC KEY-----
```

## Embedded Data Format

```
[4 bytes]  Magic: "ZIMH"
[1 byte]   Flags (text, audio, signed, symmetric, asymmetric)
[1 byte]   Method (0=LSB, 1=metadata, 2=spread)
[4 bytes]  Payload length
[N bytes]  Payload (encrypted if applicable)
[64 bytes] Signature (if signed)
```

## Shell Completions

Generate shell completions for your shell:

```bash
# Bash
zimhide completions bash > ~/.local/share/bash-completion/completions/zimhide

# Zsh (add to fpath)
zimhide completions zsh > ~/.zfunc/_zimhide

# Fish
zimhide completions fish > ~/.config/fish/completions/zimhide.fish

# PowerShell
zimhide completions powershell >> $PROFILE
```

For zsh, ensure `~/.zfunc` is in your fpath. Add to `~/.zshrc`:
```bash
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

## License

MIT
