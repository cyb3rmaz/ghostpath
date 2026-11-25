use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, CHACHA20_POLY1305, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use zeroize::Zeroizing;

use crate::error::{CoreError, Result};
use crate::fs_safe::{atomic_write_private, read_to_string_no_follow};

const ENC_PREFIX: &str = "enc:v1:";
const KEY_LEN: usize = 32;

/// Derives the keyfile path adjacent to `config_path` (e.g., `/var/lib/ghostpath/config/.master.key`).
#[must_use]
pub fn key_path_for_config(config_path: &Path) -> PathBuf {
    match config_path.parent() {
        Some(parent) => parent.join(".master.key"),
        None => PathBuf::from(".master.key"),
    }
}

/// Loads or creates a 256-bit `ChaCha20-Poly1305` master key with `0600` permissions.
pub fn load_or_create_key(key_path: &Path) -> Result<Zeroizing<[u8; KEY_LEN]>> {
    if key_path.exists() {
        let encoded = read_to_string_no_follow(key_path, 256)?;
        let raw = BASE64
            .decode(encoded.trim())
            .map_err(|e| CoreError::Config(format!("Invalid master key encoding: {e}")))?;
        if raw.len() != KEY_LEN {
            return Err(CoreError::Config(format!(
                "Invalid master key length: expected {KEY_LEN}, got {}",
                raw.len()
            )));
        }
        let mut key = Zeroizing::new([0u8; KEY_LEN]);
        key.copy_from_slice(&raw);
        return Ok(key);
    }

    let rng = SystemRandom::new();
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    rng.fill(key.as_mut())
        .map_err(|_| CoreError::Internal("Failed to generate CSPRNG master key".to_string()))?;

    let encoded = Zeroizing::new(BASE64.encode(*key));
    atomic_write_private(key_path, encoded.as_bytes())?;
    Ok(key)
}

/// Encrypts `plaintext` with `ChaCha20-Poly1305` and returns `enc:v1:<base64(nonce || ct || tag)>`.
pub fn encrypt_secret(plaintext: &str, key: &[u8; KEY_LEN]) -> Result<String> {
    if plaintext.starts_with(ENC_PREFIX) {
        return Ok(plaintext.to_string());
    }

    let unbound = UnboundKey::new(&CHACHA20_POLY1305, key)
        .map_err(|_| CoreError::Internal("Invalid AEAD key".to_string()))?;
    let aead_key = LessSafeKey::new(unbound);

    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| CoreError::Internal("Failed to generate AEAD nonce".to_string()))?;

    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = plaintext.as_bytes().to_vec();
    aead_key
        .seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| CoreError::Internal("AEAD encryption failed".to_string()))?;

    let mut payload = Vec::with_capacity(NONCE_LEN + in_out.len());
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&in_out);

    Ok(format!("{ENC_PREFIX}{}", BASE64.encode(&payload)))
}

/// Decrypts an `enc:v1:...` secret. Legacy plaintext values without the prefix are returned as-is.
pub fn decrypt_secret(value: &str, key: &[u8; KEY_LEN]) -> Result<String> {
    let Some(b64) = value.strip_prefix(ENC_PREFIX) else {
        return Ok(value.to_string());
    };

    let mut payload = BASE64
        .decode(b64.trim())
        .map_err(|e| CoreError::Config(format!("Invalid encrypted secret encoding: {e}")))?;

    if payload.len() < NONCE_LEN + CHACHA20_POLY1305.tag_len() {
        return Err(CoreError::Config(
            "Truncated encrypted secret payload".to_string(),
        ));
    }

    let mut nonce_bytes = [0u8; NONCE_LEN];
    nonce_bytes.copy_from_slice(&payload[..NONCE_LEN]);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let unbound = UnboundKey::new(&CHACHA20_POLY1305, key)
        .map_err(|_| CoreError::Internal("Invalid AEAD key".to_string()))?;
    let aead_key = LessSafeKey::new(unbound);

    let plaintext = aead_key
        .open_in_place(nonce, Aad::empty(), &mut payload[NONCE_LEN..])
        .map_err(|_| {
            CoreError::Config("Secret decryption failed (authentication tag mismatch)".to_string())
        })?;

    String::from_utf8(plaintext.to_vec())
        .map_err(|_| CoreError::Config("Decrypted secret is not valid UTF-8".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip_and_tamper_detection() {
        let key = [0x42u8; KEY_LEN];
        let secret = "vless://uuid-1234@example.com:443?pbk=secretkey#Test";
        let sealed = encrypt_secret(secret, &key).unwrap();
        assert!(sealed.starts_with(ENC_PREFIX));
        assert!(!sealed.contains("uuid-1234"));
        assert!(!sealed.contains("secretkey"));

        let opened = decrypt_secret(&sealed, &key).unwrap();
        assert_eq!(opened, secret);

        // Tamper with ciphertext
        let mut chars: Vec<char> = sealed.chars().collect();
        let last = chars.len() - 2;
        chars[last] = if chars[last] == 'A' { 'B' } else { 'A' };
        let tampered: String = chars.into_iter().collect();
        assert!(decrypt_secret(&tampered, &key).is_err());

        // Legacy plaintext passthrough
        assert_eq!(
            decrypt_secret("legacy-plain", &key).unwrap(),
            "legacy-plain"
        );
    }
}
