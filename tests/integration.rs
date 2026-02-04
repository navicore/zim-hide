mod common;

use common::{presets, TestWavConfig};
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
    assert!(!status.success(), "should fail when message exceeds capacity");
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
    assert!(output_result.status.success(), "decode empty message failed");

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
            .args([
                "decode",
                output.to_str().unwrap(),
                "--channels",
                channel,
            ])
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
