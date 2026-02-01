use hound::{SampleFormat, WavSpec, WavWriter};
use std::process::Command;
use tempfile::tempdir;

fn create_test_wav(path: &std::path::Path) {
    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).unwrap();

    // Generate 1 second of a 440Hz sine wave
    for i in 0..44100 {
        let t = i as f32 / 44100.0;
        let sample = ((t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 10000.0) as i16;
        writer.write_sample(sample).unwrap();
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();
}

fn vvw_binary() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test binary name
    path.pop(); // Remove deps
    path.push("vvw");
    path
}

#[test]
fn test_basic_encode_decode_cycle() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    create_test_wav(&input);

    // Encode
    let status = Command::new(vvw_binary())
        .args(["encode", input.to_str().unwrap(), "-o", output.to_str().unwrap(), "--message", "Hello, world!"])
        .status()
        .unwrap();
    assert!(status.success(), "encode failed");

    // Decode
    let output_result = Command::new(vvw_binary())
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

    create_test_wav(&input);

    // Encode with passphrase
    let status = Command::new(vvw_binary())
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
    let output_result = Command::new(vvw_binary())
        .args(["decode", output.to_str().unwrap(), "--passphrase", "puzzle123"])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "decode with passphrase failed");

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Secret message");

    // Decode without passphrase should fail
    let fail_result = Command::new(vvw_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!fail_result.status.success(), "decode without passphrase should fail");
}

#[test]
fn test_asymmetric_encryption_cycle() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    let keybase = dir.path().join("test");

    create_test_wav(&input);

    // Generate keypair
    let status = Command::new(vvw_binary())
        .args(["keygen", "--output", keybase.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success(), "keygen failed");

    let pub_key = keybase.with_extension("pub");
    let priv_key = keybase.with_extension("priv");
    assert!(pub_key.exists(), "public key not created");
    assert!(priv_key.exists(), "private key not created");

    // Encode with public key
    let status = Command::new(vvw_binary())
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
    let output_result = Command::new(vvw_binary())
        .args([
            "decode",
            output.to_str().unwrap(),
            "--key",
            priv_key.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "decode with private key failed");

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Asymmetric secret");
}

#[test]
fn test_signed_message() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    let keybase = dir.path().join("signer");

    create_test_wav(&input);

    // Generate keypair
    Command::new(vvw_binary())
        .args(["keygen", "--output", keybase.to_str().unwrap()])
        .status()
        .unwrap();

    let pub_key = keybase.with_extension("pub");
    let priv_key = keybase.with_extension("priv");

    // Encode with signature
    let status = Command::new(vvw_binary())
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
    let output_result = Command::new(vvw_binary())
        .args([
            "decode",
            output.to_str().unwrap(),
            "--verify",
            pub_key.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "decode with verification failed");

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Signed message");
}

#[test]
fn test_inspect_command() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    create_test_wav(&input);

    // Encode
    Command::new(vvw_binary())
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
    let output_result = Command::new(vvw_binary())
        .args(["inspect", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "inspect failed");

    let inspect_output = String::from_utf8_lossy(&output_result.stdout);
    assert!(inspect_output.contains("VVW Embedded Data"));
    assert!(inspect_output.contains("Method: LSB"));
    assert!(inspect_output.contains("text"));
}

#[test]
fn test_metadata_method() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");

    create_test_wav(&input);

    // Encode with metadata method
    let status = Command::new(vvw_binary())
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
    let output_result = Command::new(vvw_binary())
        .args(["decode", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output_result.status.success(), "decode metadata failed");

    let decoded = String::from_utf8_lossy(&output_result.stdout);
    assert_eq!(decoded.trim(), "Metadata message");

    // Inspect should show metadata method
    let inspect_result = Command::new(vvw_binary())
        .args(["inspect", output.to_str().unwrap()])
        .output()
        .unwrap();
    let inspect_output = String::from_utf8_lossy(&inspect_result.stdout);
    assert!(inspect_output.contains("Metadata"));
}
