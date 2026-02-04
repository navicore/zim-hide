mod common;

use common::{AudioPattern, TestWavConfig, presets};
use std::process::Command;
use tempfile::tempdir;

fn zimhide_binary() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test binary name
    path.pop(); // Remove deps
    path.push("zimhide");
    path
}

// ============================================================================
// Basic functionality tests (using standard sine wave)
// ============================================================================

#[test]
fn test_basic_encode_decode_cycle() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::standard().write_to_path(&input);

    // Encode
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Hello, world!",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode failed");

    // Decode
    let output_result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "decode failed");

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Hello, world!");
}

#[test]
fn test_symmetric_encryption_cycle() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::standard().write_to_path(&input);

    // Encode with passphrase
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Secret message",
            "--passphrase",
            "puzzle123",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode with passphrase failed");

    // Decode with correct passphrase
    let output_result = Command::new(zimhide_binary())
        .args([
            "decode",
            output.to_str().unwrap(),
            "--passphrase",
            "puzzle123",
        ])
        .output()
        .unwrap();
    assert!(
        output_result.status.success(),
        "decode with passphrase failed"
    );

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Secret message");

    // Decode without passphrase should fail
    let fail_result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        !fail_result.status.success(),
        "decode without passphrase should fail"
    );
}

#[test]
fn test_asymmetric_encryption_cycle() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    let keybase = dir.path().join("test");

    presets::standard().write_to_path(&input);

    // Generate keypair
    let status = Command::new(zimhide_binary())
        .args(["keygen", "--output", keybase.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success(), "keygen failed");

    let pub_key = keybase.with_extension("pub");
    let priv_key = keybase.with_extension("priv");
    assert!(pub_key.exists(), "public key not created");
    assert!(priv_key.exists(), "private key not created");

    // Encode with public key
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Asymmetric secret",
            "--encrypt-to",
            pub_key.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode with public key failed");

    // Decode with private key
    let output_result = Command::new(zimhide_binary())
        .args([
            "decode",
            output.to_str().unwrap(),
            "--key",
            priv_key.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output_result.status.success(),
        "decode with private key failed"
    );

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Asymmetric secret");
}

#[test]
fn test_signed_message() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    let keybase = dir.path().join("signer");

    presets::standard().write_to_path(&input);

    // Generate keypair
    Command::new(zimhide_binary())
        .args(["keygen", "--output", keybase.to_str().unwrap()])
        .status()
        .unwrap();

    let pub_key = keybase.with_extension("pub");
    let priv_key = keybase.with_extension("priv");

    // Encode with signature
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Signed message",
            "--sign",
            "--key",
            priv_key.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode with signature failed");

    // Decode and verify
    let output_result = Command::new(zimhide_binary())
        .args([
            "decode",
            output.to_str().unwrap(),
            "--verify",
            pub_key.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output_result.status.success(),
        "decode with verification failed"
    );

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Signed message");
}

#[test]
fn test_inspect_command() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::standard().write_to_path(&input);

    // Encode
    Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Test message",
        ])
        .status()
        .unwrap();

    // Inspect
    let output_result = Command::new(zimhide_binary())
        .args(["inspect", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "inspect failed");

    let inspect_output = String::from_utf8_lossy(&output_result.stdout);
    assert!(inspect_output.contains("Zimhide Embedded Data"));
    assert!(inspect_output.contains("Method: LSB"));
    assert!(inspect_output.contains("text"));
}

#[test]
fn test_metadata_method() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::standard().write_to_path(&input);

    // Encode with metadata method
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Metadata message",
            "--method",
            "metadata",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode with metadata method failed");

    // Decode
    let output_result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "decode metadata failed");

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Metadata message");

    // Inspect should show metadata method
    let inspect_result = Command::new(zimhide_binary())
        .args(["inspect", output.to_str().unwrap()])
        .output()
        .unwrap();
    let inspect_output = String::from_utf8_lossy(&inspect_result.stdout);
    assert!(inspect_output.contains("Metadata"));
}

// ============================================================================
// Audio pattern variation tests
// ============================================================================

/// Helper to test encode/decode cycle with a specific audio config
fn test_encode_decode_with_config(config: TestWavConfig, test_name: &str) {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    config.write_to_path(&input);

    let message = format!("Test message for {test_name}");

    // Encode
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            &message,
        ])
        .status()
        .unwrap();
    assert!(status.success(), "{test_name}: encode failed");

    // Decode
    let output_result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "{test_name}: decode failed");

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), message, "{test_name}: message mismatch");
}

#[test]
fn test_audio_pattern_silence() {
    test_encode_decode_with_config(presets::silence(), "silence");
}

#[test]
fn test_audio_pattern_white_noise() {
    test_encode_decode_with_config(presets::noise(), "white_noise");
}

#[test]
fn test_audio_pattern_multi_frequency() {
    test_encode_decode_with_config(presets::complex(), "multi_frequency");
}

#[test]
fn test_audio_pattern_amplitude_sweep() {
    test_encode_decode_with_config(presets::sweep(), "amplitude_sweep");
}

#[test]
fn test_audio_pattern_loud_clipping() {
    test_encode_decode_with_config(presets::loud(), "loud_clipping");
}

#[test]
fn test_audio_pattern_very_quiet() {
    test_encode_decode_with_config(presets::quiet(), "very_quiet");
}

#[test]
fn test_audio_pattern_square_wave() {
    test_encode_decode_with_config(presets::square(), "square_wave");
}

// ============================================================================
// Format variation tests
// ============================================================================

#[test]
fn test_format_mono_22k() {
    test_encode_decode_with_config(presets::mono_22k(), "mono_22k");
}

#[test]
fn test_format_stereo_48k() {
    test_encode_decode_with_config(presets::stereo_48k(), "stereo_48k");
}

#[test]
fn test_format_short_100ms() {
    // Short file - use a smaller message to fit capacity
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::short_100ms().write_to_path(&input);

    // Encode with a short message
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Hi",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "short_100ms: encode failed");

    // Decode
    let output_result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "short_100ms: decode failed");

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Hi");
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_capacity_exceeded() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    // Very short file with minimal capacity
    TestWavConfig::default()
        .duration(0.01) // 10ms = ~441 samples stereo = ~110 bytes capacity
        .write_to_path(&input);

    // Try to embed a message that's too large
    let large_message = "X".repeat(500);

    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            &large_message,
        ])
        .status()
        .unwrap();

    // Should fail due to insufficient capacity
    assert!(
        !status.success(),
        "should fail when message exceeds capacity"
    );
}

#[test]
fn test_empty_message() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::standard().write_to_path(&input);

    // Encode with empty message
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode empty message failed");

    // Decode
    let output_result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output_result.status.success(),
        "decode empty message failed"
    );

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "");
}

#[test]
fn test_unicode_message() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::standard().write_to_path(&input);

    let unicode_msg = "Hello 世界! 🎵 Привет мир! αβγδ";

    // Encode
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            unicode_msg,
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode unicode failed");

    // Decode
    let output_result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "decode unicode failed");

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), unicode_msg);
}

#[test]
fn test_bits_per_sample_variations() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");

    presets::standard().write_to_path(&input);

    for bits in [1, 2, 3, 4] {
        let output = dir.path().join(format!("output_{bits}bit.wav"));
        let message = format!("Testing {bits} bits per sample");

        // Encode with specific bits
        let status = Command::new(zimhide_binary())
            .args([
                "encode",
                input.to_str().unwrap(),
                "-o",
                output.to_str().unwrap(),
                "--message",
                &message,
                "--bits",
                &bits.to_string(),
            ])
            .status()
            .unwrap();
        assert!(status.success(), "encode with {bits} bits failed");

        // Decode with same bits setting
        let output_result = Command::new(zimhide_binary())
            .args([
                "decode",
                output.to_str().unwrap(),
                "--bits",
                &bits.to_string(),
            ])
            .output()
            .unwrap();
        assert!(
            output_result.status.success(),
            "decode with {bits} bits failed"
        );

        let decoded = String::from_utf8_lossy(&output_result.stdout);
        assert_eq!(decoded.trim(), message, "message mismatch for {bits} bits");
    }
}

#[test]
fn test_channel_variations() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");

    presets::standard().write_to_path(&input);

    for channel in ["left", "right", "both"] {
        let output = dir.path().join(format!("output_{channel}.wav"));
        let message = format!("Testing {channel} channel");

        // Encode with specific channel
        let status = Command::new(zimhide_binary())
            .args([
                "encode",
                input.to_str().unwrap(),
                "-o",
                output.to_str().unwrap(),
                "--message",
                &message,
                "--channels",
                channel,
            ])
            .status()
            .unwrap();
        assert!(status.success(), "encode with {channel} channel failed");

        // Decode with same channel setting
        let output_result = Command::new(zimhide_binary())
            .args(["decode", output.to_str().unwrap(), "--channels", channel])
            .output()
            .unwrap();
        assert!(
            output_result.status.success(),
            "decode with {channel} channel failed"
        );

        let decoded = String::from_utf8_lossy(&output_result.stdout);
        assert_eq!(
            decoded.trim(),
            message,
            "message mismatch for {channel} channel"
        );
    }
}

// ============================================================================
// Combined feature tests
// ============================================================================

#[test]
fn test_encrypted_with_noise_audio() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::noise().write_to_path(&input);

    // Encode with encryption
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Secret in noise",
            "--passphrase",
            "noisy123",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode encrypted in noise failed");

    // Decode
    let output_result = Command::new(zimhide_binary())
        .args([
            "decode",
            output.to_str().unwrap(),
            "--passphrase",
            "noisy123",
        ])
        .output()
        .unwrap();
    assert!(
        output_result.status.success(),
        "decode encrypted from noise failed"
    );

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Secret in noise");
}

#[test]
fn test_signed_with_quiet_audio() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    let keybase = dir.path().join("quietkey");

    presets::quiet().write_to_path(&input);

    // Generate keypair
    Command::new(zimhide_binary())
        .args(["keygen", "--output", keybase.to_str().unwrap()])
        .status()
        .unwrap();

    let pub_key = keybase.with_extension("pub");
    let priv_key = keybase.with_extension("priv");

    // Encode signed message in quiet audio
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Signed quiet message",
            "--sign",
            "--key",
            priv_key.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode signed in quiet audio failed");

    // Decode and verify
    let output_result = Command::new(zimhide_binary())
        .args([
            "decode",
            output.to_str().unwrap(),
            "--verify",
            pub_key.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output_result.status.success(),
        "decode/verify from quiet audio failed"
    );

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Signed quiet message");
}

#[test]
fn test_metadata_method_with_mono() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::mono_22k().write_to_path(&input);

    // Encode with metadata method
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Mono metadata",
            "--method",
            "metadata",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode metadata in mono failed");

    // Decode
    let output_result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output_result.status.success(),
        "decode metadata from mono failed"
    );

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Mono metadata");
}

// ============================================================================
// Audio embedding tests (--audio flag)
// ============================================================================

#[test]
fn test_audio_embedding_basic() {
    let dir = tempdir().unwrap();
    let carrier = dir.path().join("carrier.wav");
    let audio_to_embed = dir.path().join("embed.wav");
    let output = dir.path().join("output.wav");
    let extracted = dir.path().join("extracted.wav");

    // Create a larger carrier file (2 seconds for capacity)
    TestWavConfig::default()
        .duration(2.0)
        .write_to_path(&carrier);

    // Create a small audio file to embed (100ms at 48kHz for Opus compression)
    TestWavConfig::default()
        .sample_rate(48000)
        .duration(0.1)
        .pattern(AudioPattern::Sine(880.0))
        .write_to_path(&audio_to_embed);

    // Embed audio using metadata method (LSB capacity may be too small for raw WAV)
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            carrier.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--audio",
            audio_to_embed.to_str().unwrap(),
            "--method",
            "metadata",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode with audio failed");

    // Extract using play --extract-to
    let status = Command::new(zimhide_binary())
        .args([
            "play",
            output.to_str().unwrap(),
            "--extract-to",
            extracted.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "play --extract-to failed");

    // Verify extracted file exists and is a valid WAV
    // Note: Opus is lossy, so we can't compare bytes exactly
    assert!(extracted.exists(), "extracted file not created");
    let extracted_size = std::fs::metadata(&extracted).unwrap().len();
    assert!(extracted_size > 0, "extracted file is empty");
}

#[test]
fn test_audio_embedding_with_text() {
    let dir = tempdir().unwrap();
    let carrier = dir.path().join("carrier.wav");
    let audio_to_embed = dir.path().join("embed.wav");
    let output = dir.path().join("output.wav");
    let extracted = dir.path().join("extracted.wav");

    TestWavConfig::default()
        .duration(2.0)
        .write_to_path(&carrier);

    // 48kHz required for Opus compression
    TestWavConfig::default()
        .sample_rate(48000)
        .duration(0.1)
        .write_to_path(&audio_to_embed);

    // Embed both text and audio
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            carrier.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Text with audio",
            "--audio",
            audio_to_embed.to_str().unwrap(),
            "--method",
            "metadata",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode with text+audio failed");

    // Decode should show text
    let output_result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "decode failed");
    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Text with audio");

    // Extract audio
    let status = Command::new(zimhide_binary())
        .args([
            "play",
            output.to_str().unwrap(),
            "--extract-to",
            extracted.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "play --extract-to failed");
    assert!(extracted.exists());
}

#[test]
fn test_audio_embedding_encrypted() {
    let dir = tempdir().unwrap();
    let carrier = dir.path().join("carrier.wav");
    let audio_to_embed = dir.path().join("embed.wav");
    let output = dir.path().join("output.wav");
    let extracted = dir.path().join("extracted.wav");

    TestWavConfig::default()
        .duration(2.0)
        .write_to_path(&carrier);

    // 48kHz required for Opus compression
    TestWavConfig::default()
        .sample_rate(48000)
        .duration(0.1)
        .write_to_path(&audio_to_embed);

    // Embed encrypted audio
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            carrier.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--audio",
            audio_to_embed.to_str().unwrap(),
            "--passphrase",
            "secret123",
            "--method",
            "metadata",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode encrypted audio failed");

    // Extract without passphrase should fail
    let fail_result = Command::new(zimhide_binary())
        .args([
            "play",
            output.to_str().unwrap(),
            "--extract-to",
            extracted.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !fail_result.status.success(),
        "should fail without passphrase"
    );

    // Extract with passphrase should succeed
    let status = Command::new(zimhide_binary())
        .args([
            "play",
            output.to_str().unwrap(),
            "--extract-to",
            extracted.to_str().unwrap(),
            "--passphrase",
            "secret123",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "play with passphrase failed");

    // Verify extracted file exists and is valid (Opus is lossy, so no byte comparison)
    assert!(extracted.exists(), "extracted file not created");
    let extracted_size = std::fs::metadata(&extracted).unwrap().len();
    assert!(extracted_size > 0, "extracted file is empty");
}

// ============================================================================
// Multi-recipient encryption tests
// ============================================================================

#[test]
fn test_multi_recipient_encryption() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    let key1_base = dir.path().join("recipient1");
    let key2_base = dir.path().join("recipient2");
    let key3_base = dir.path().join("recipient3");

    presets::standard().write_to_path(&input);

    // Generate three keypairs
    for keybase in [&key1_base, &key2_base, &key3_base] {
        Command::new(zimhide_binary())
            .args(["keygen", "--output", keybase.to_str().unwrap()])
            .status()
            .unwrap();
    }

    // Encode to multiple recipients
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Multi-recipient secret",
            "--encrypt-to",
            key1_base.with_extension("pub").to_str().unwrap(),
            "--encrypt-to",
            key2_base.with_extension("pub").to_str().unwrap(),
            "--encrypt-to",
            key3_base.with_extension("pub").to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode to multiple recipients failed");

    // Each recipient should be able to decrypt
    for (i, keybase) in [&key1_base, &key2_base, &key3_base].iter().enumerate() {
        let output_result = Command::new(zimhide_binary())
            .args([
                "decode",
                output.to_str().unwrap(),
                "--key",
                keybase.with_extension("priv").to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            output_result.status.success(),
            "recipient {} failed to decrypt",
            i + 1
        );
        let decoded = String::from_utf8_lossy(&output_result.stdout);
        assert_eq!(decoded.trim(), "Multi-recipient secret");
    }
}

// ============================================================================
// Signed + encrypted combination tests
// ============================================================================

#[test]
fn test_signed_and_symmetric_encrypted() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    let keybase = dir.path().join("signer");

    presets::complex().write_to_path(&input);

    // Generate signing keypair
    Command::new(zimhide_binary())
        .args(["keygen", "--output", keybase.to_str().unwrap()])
        .status()
        .unwrap();

    let pub_key = keybase.with_extension("pub");
    let priv_key = keybase.with_extension("priv");

    // Encode with both signing and symmetric encryption
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Signed and encrypted",
            "--sign",
            "--key",
            priv_key.to_str().unwrap(),
            "--passphrase",
            "secret123",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode signed+encrypted failed");

    // Decode and verify
    let output_result = Command::new(zimhide_binary())
        .args([
            "decode",
            output.to_str().unwrap(),
            "--passphrase",
            "secret123",
            "--verify",
            pub_key.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output_result.status.success(),
        "decode signed+encrypted failed"
    );

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Signed and encrypted");
}

#[test]
fn test_signed_and_asymmetric_encrypted() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    let signer_key = dir.path().join("signer");
    let recipient_key = dir.path().join("recipient");

    presets::noise().write_to_path(&input);

    // Generate keypairs
    Command::new(zimhide_binary())
        .args(["keygen", "--output", signer_key.to_str().unwrap()])
        .status()
        .unwrap();
    Command::new(zimhide_binary())
        .args(["keygen", "--output", recipient_key.to_str().unwrap()])
        .status()
        .unwrap();

    // Encode with signing and asymmetric encryption
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Signed for recipient",
            "--sign",
            "--key",
            signer_key.with_extension("priv").to_str().unwrap(),
            "--encrypt-to",
            recipient_key.with_extension("pub").to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode signed+asymmetric failed");

    // Decode with recipient key and verify signature
    let output_result = Command::new(zimhide_binary())
        .args([
            "decode",
            output.to_str().unwrap(),
            "--key",
            recipient_key.with_extension("priv").to_str().unwrap(),
            "--verify",
            signer_key.with_extension("pub").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output_result.status.success(),
        "decode signed+asymmetric failed"
    );

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Signed for recipient");
}

// ============================================================================
// Message file tests
// ============================================================================

#[test]
fn test_message_file() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    let message_file = dir.path().join("message.txt");

    presets::standard().write_to_path(&input);

    // Create message file with multi-line content
    let message_content =
        "This is a message from a file.\nIt has multiple lines.\nAnd some special chars: @#$%";
    std::fs::write(&message_file, message_content).unwrap();

    // Encode using message file
    let status = Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message-file",
            message_file.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "encode with message-file failed");

    // Decode
    let output_result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "decode failed");

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), message_content);
}

// ============================================================================
// Error handling tests
// ============================================================================

#[test]
fn test_decode_wrong_passphrase() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::standard().write_to_path(&input);

    // Encode with passphrase
    Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Secret",
            "--passphrase",
            "correct",
        ])
        .status()
        .unwrap();

    // Decode with wrong passphrase should fail
    let result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap(), "--passphrase", "wrong"])
        .output()
        .unwrap();
    assert!(
        !result.status.success(),
        "should fail with wrong passphrase"
    );
}

#[test]
fn test_decode_wrong_key() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    let key1 = dir.path().join("key1");
    let key2 = dir.path().join("key2");

    presets::standard().write_to_path(&input);

    // Generate two different keypairs
    Command::new(zimhide_binary())
        .args(["keygen", "--output", key1.to_str().unwrap()])
        .status()
        .unwrap();
    Command::new(zimhide_binary())
        .args(["keygen", "--output", key2.to_str().unwrap()])
        .status()
        .unwrap();

    // Encode to key1
    Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Secret",
            "--encrypt-to",
            key1.with_extension("pub").to_str().unwrap(),
        ])
        .status()
        .unwrap();

    // Decode with key2 should fail
    let result = Command::new(zimhide_binary())
        .args([
            "decode",
            output.to_str().unwrap(),
            "--key",
            key2.with_extension("priv").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success(), "should fail with wrong key");
}

#[test]
fn test_verify_wrong_public_key() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    let signer_key = dir.path().join("signer");
    let other_key = dir.path().join("other");

    presets::standard().write_to_path(&input);

    // Generate two keypairs
    Command::new(zimhide_binary())
        .args(["keygen", "--output", signer_key.to_str().unwrap()])
        .status()
        .unwrap();
    Command::new(zimhide_binary())
        .args(["keygen", "--output", other_key.to_str().unwrap()])
        .status()
        .unwrap();

    // Sign with signer's key
    Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Signed message",
            "--sign",
            "--key",
            signer_key.with_extension("priv").to_str().unwrap(),
        ])
        .status()
        .unwrap();

    // Verify with other's public key should fail
    let result = Command::new(zimhide_binary())
        .args([
            "decode",
            output.to_str().unwrap(),
            "--verify",
            other_key.with_extension("pub").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !result.status.success(),
        "should fail with wrong verify key"
    );
}

#[test]
fn test_decode_mismatched_bits() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::standard().write_to_path(&input);

    // Encode with 2 bits
    Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Test",
            "--bits",
            "2",
        ])
        .status()
        .unwrap();

    // Decode with 1 bit should fail (wrong magic/corrupt data)
    let result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap(), "--bits", "1"])
        .output()
        .unwrap();
    assert!(!result.status.success(), "should fail with mismatched bits");
}

#[test]
fn test_decode_mismatched_channels() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    presets::standard().write_to_path(&input);

    // Encode with left channel only
    Command::new(zimhide_binary())
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Test",
            "--channels",
            "left",
        ])
        .status()
        .unwrap();

    // Decode with right channel should fail
    let result = Command::new(zimhide_binary())
        .args(["decode", output.to_str().unwrap(), "--channels", "right"])
        .output()
        .unwrap();
    assert!(
        !result.status.success(),
        "should fail with mismatched channels"
    );
}

#[test]
fn test_encode_nonexistent_input() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("output.wav");

    let result = Command::new(zimhide_binary())
        .args([
            "encode",
            "/nonexistent/path/input.wav",
            "-o",
            output.to_str().unwrap(),
            "--message",
            "Test",
        ])
        .output()
        .unwrap();
    assert!(
        !result.status.success(),
        "should fail with nonexistent input"
    );
}

#[test]
fn test_decode_no_embedded_data() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");

    // Create a plain WAV with no embedded data
    presets::standard().write_to_path(&input);

    let result = Command::new(zimhide_binary())
        .args(["decode", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        !result.status.success(),
        "should fail on file without embedded data"
    );
}

// ============================================================================
// Expanded audio pattern coverage with crypto features
// ============================================================================

#[test]
fn test_symmetric_encryption_all_patterns() {
    let dir = tempdir().unwrap();

    let patterns = [
        ("silence", presets::silence()),
        ("noise", presets::noise()),
        ("complex", presets::complex()),
        ("sweep", presets::sweep()),
        ("loud", presets::loud()),
        ("quiet", presets::quiet()),
        ("square", presets::square()),
    ];

    for (name, config) in patterns {
        let input = dir.path().join(format!("input_{name}.wav"));
        let output = dir.path().join(format!("output_{name}.wav"));

        config.write_to_path(&input);

        let message = format!("Encrypted message in {name}");

        // Encode
        let status = Command::new(zimhide_binary())
            .args([
                "encode",
                input.to_str().unwrap(),
                "-o",
                output.to_str().unwrap(),
                "--message",
                &message,
                "--passphrase",
                "pattern_test",
            ])
            .status()
            .unwrap();
        assert!(status.success(), "{name}: encode failed");

        // Decode
        let result = Command::new(zimhide_binary())
            .args([
                "decode",
                output.to_str().unwrap(),
                "--passphrase",
                "pattern_test",
            ])
            .output()
            .unwrap();
        assert!(result.status.success(), "{name}: decode failed");

        let decoded = String::from_utf8_lossy(&result.stdout);
        assert_eq!(decoded.trim(), message, "{name}: message mismatch");
    }
}

#[test]
fn test_metadata_method_all_patterns() {
    let dir = tempdir().unwrap();

    let patterns = [
        ("silence", presets::silence()),
        ("noise", presets::noise()),
        ("mono", presets::mono_22k()),
        ("stereo48k", presets::stereo_48k()),
    ];

    for (name, config) in patterns {
        let input = dir.path().join(format!("input_{name}.wav"));
        let output = dir.path().join(format!("output_{name}.wav"));

        config.write_to_path(&input);

        let message = format!("Metadata in {name}");

        // Encode with metadata
        let status = Command::new(zimhide_binary())
            .args([
                "encode",
                input.to_str().unwrap(),
                "-o",
                output.to_str().unwrap(),
                "--message",
                &message,
                "--method",
                "metadata",
            ])
            .status()
            .unwrap();
        assert!(status.success(), "{name}: encode metadata failed");

        // Decode
        let result = Command::new(zimhide_binary())
            .args(["decode", output.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(result.status.success(), "{name}: decode metadata failed");

        let decoded = String::from_utf8_lossy(&result.stdout);
        assert_eq!(decoded.trim(), message, "{name}: message mismatch");
    }
}
