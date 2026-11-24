//! Small authenticated encryption helper for app data and mod-owned secrets.
//!
//! The default key is generated once and stored on the card.  A user key is
//! stored wrapped by that device key; this is deliberately obfuscation-at-rest,
//! not protection against someone who can read both card files.

use std::path::Path;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::digest;
use ring::rand::{SecureRandom, SystemRandom};

pub const CRYPTO_KEY_REL: &str = "linkSlot/Config/crypto.key";
/// The user key is stored encrypted by [`CRYPTO_KEY_REL`]; the plaintext
/// passphrase never touches the card.
pub const CRYPTO_USER_REL: &str = "linkSlot/Config/crypto.user";
pub const MAX_PLAINTEXT: usize = 1024 * 1024;
pub const MAX_TOKEN: usize = 2 * 1024 * 1024;

const KEY_BYTES: usize = 32;
const NONCE_BYTES: usize = 12;
const VERSION: &str = "v1";
const USER_KEY_SCOPE: &str = "config:user-key";

/// Read or create the random per-card default key.
pub fn device_key(root: &Path) -> Result<[u8; KEY_BYTES], String> {
    let path = root.join(CRYPTO_KEY_REL);
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Some(key) = parse_hex(text.trim()) {
            return Ok(key);
        }
    }
    let mut key = [0u8; KEY_BYTES];
    SystemRandom::new()
        .fill(&mut key)
        .map_err(|_| "无法生成设备加密密钥".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("无法创建加密配置目录：{error}"))?;
    }
    let body = format!("{}\n", hex(&key));
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, body.as_bytes())
        .map_err(|error| format!("无法写入设备加密密钥：{error}"))?;
    std::fs::rename(&tmp, &path)
        .map_err(|error| format!("无法保存设备加密密钥：{error}"))?;
    Ok(key)
}

/// Read the optional user key, unwrapping it with the device key.
pub fn user_key(root: &Path) -> Result<Option<String>, String> {
    let Ok(token) = std::fs::read_to_string(root.join(CRYPTO_USER_REL)) else {
        return Ok(None);
    };
    let token = token.trim();
    if token.is_empty() {
        return Ok(None);
    }
    let material = device_key(root)?;
    let key_bytes = key_from_material(USER_KEY_SCOPE, &material);
    Ok(Some(open_with_key(&key_bytes, token)?))
}

/// Persist or clear the user key.  `None`/empty clears it.
pub fn set_user_key(root: &Path, key: Option<&str>) -> Result<(), String> {
    let path = root.join(CRYPTO_USER_REL);
    let Some(key) = key.map(str::trim).filter(|key| !key.is_empty()) else {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    };
    if key.len() > 4096 {
        return Err("加密密钥过长".to_string());
    }
    let material = device_key(root)?;
    let token = seal(&material, USER_KEY_SCOPE, key.as_bytes())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("无法创建加密配置目录：{error}"))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, format!("{token}\n").as_bytes())
        .map_err(|error| format!("无法写入用户密钥：{error}"))?;
    std::fs::rename(&tmp, &path)
        .map_err(|error| format!("无法保存用户密钥：{error}"))
}

fn parse_hex(text: &str) -> Option<[u8; KEY_BYTES]> {
    if text.len() != KEY_BYTES * 2 {
        return None;
    }
    let mut out = [0u8; KEY_BYTES];
    for (index, pair) in text.as_bytes().chunks_exact(2).enumerate() {
        let high = (pair[0] as char).to_digit(16)?;
        let low = (pair[1] as char).to_digit(16)?;
        out[index] = (high * 16 + low) as u8;
    }
    Some(out)
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn key_from_material(scope: &str, material: &[u8]) -> [u8; KEY_BYTES] {
    let mut ctx = digest::Context::new(&digest::SHA256);
    ctx.update(b"linkSlot-crypto-v1\0");
    ctx.update(scope.as_bytes());
    ctx.update(&[0]);
    ctx.update(material);
    let digest = ctx.finish();
    let mut out = [0u8; KEY_BYTES];
    out.copy_from_slice(digest.as_ref());
    out
}

fn material_for(root: &Path, caller_key: Option<&str>) -> Result<Vec<u8>, String> {
    if let Some(key) = caller_key.map(str::trim).filter(|key| !key.is_empty()) {
        return Ok(key.as_bytes().to_vec());
    }
    if let Some(key) = user_key(root)? {
        return Ok(key.into_bytes());
    }
    Ok(device_key(root)?.to_vec())
}

fn seal(material: &[u8], scope: &str, plaintext: &[u8]) -> Result<String, String> {
    let key_bytes = key_from_material(scope, material);
    let unbound = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| "无法初始化加密密钥".to_string())?;
    let key = LessSafeKey::new(unbound);
    let mut nonce_bytes = [0u8; NONCE_BYTES];
    SystemRandom::new()
        .fill(&mut nonce_bytes)
        .map_err(|_| "无法生成加密随机数".to_string())?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = plaintext.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| "加密失败".to_string())?;
    let mut packed = Vec::with_capacity(NONCE_BYTES + in_out.len());
    packed.extend_from_slice(&nonce_bytes);
    packed.extend_from_slice(&in_out);
    Ok(format!("{VERSION}:{}", BASE64.encode(packed)))
}

fn open_with_key(key_bytes: &[u8; KEY_BYTES], token: &str) -> Result<String, String> {
    if token.len() > MAX_TOKEN {
        return Err("密文超过上限".to_string());
    }
    let Some(body) = token.strip_prefix(VERSION).and_then(|body| body.strip_prefix(':')) else {
        return Err("密文格式无效或密钥错误".to_string());
    };
    let packed = BASE64
        .decode(body)
        .map_err(|_| "密文格式无效或密钥错误".to_string())?;
    if packed.len() < NONCE_BYTES + 16 {
        return Err("密文格式无效或密钥错误".to_string());
    }
    let (nonce_bytes, ciphertext) = packed.split_at(NONCE_BYTES);
    let mut nonce = [0u8; NONCE_BYTES];
    nonce.copy_from_slice(nonce_bytes);
    let unbound = UnboundKey::new(&AES_256_GCM, key_bytes)
        .map_err(|_| "无法初始化加密密钥".to_string())?;
    let key = LessSafeKey::new(unbound);
    let mut in_out = ciphertext.to_vec();
    let plain = key
        .open_in_place(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut in_out)
        .map_err(|_| "密文格式无效或密钥错误".to_string())?;
    String::from_utf8(plain.to_vec()).map_err(|_| "解密结果不是 UTF-8".to_string())
}

/// Encrypt UTF-8 text.  An explicit caller key overrides the persisted user
/// key, which overrides the generated device key.
pub fn encrypt(
    root: &Path,
    plaintext: &str,
    scope: &str,
    caller_key: Option<&str>,
) -> Result<String, String> {
    if plaintext.len() > MAX_PLAINTEXT {
        return Err("明文超过加密上限".to_string());
    }
    let material = material_for(root, caller_key)?;
    seal(&material, scope, plaintext.as_bytes())
}

/// Decrypt a token produced by [`encrypt`].  Any authentication failure is
/// reported as one generic error so callers cannot use it as an oracle.
pub fn decrypt(
    root: &Path,
    token: &str,
    scope: &str,
    caller_key: Option<&str>,
) -> Result<String, String> {
    let material = material_for(root, caller_key)?;
    let key_bytes = key_from_material(scope, &material);
    open_with_key(&key_bytes, token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_key_is_stable_and_encrypts_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let token = encrypt(root, "secret", "mod:one", None).unwrap();
        assert_eq!(decrypt(root, &token, "mod:one", None).unwrap(), "secret");
        assert_eq!(
            device_key(root).unwrap(),
            device_key(root).unwrap(),
            "the default key was regenerated"
        );
    }

    #[test]
    fn a_wrong_scope_or_key_cannot_decrypt() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let token = encrypt(root, "secret", "mod:one", None).unwrap();
        assert!(decrypt(root, &token, "mod:two", None).is_err());
        assert!(decrypt(root, &token, "mod:one", Some("different")).is_err());
    }

    #[test]
    fn a_persisted_user_key_overrides_the_device_key() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        set_user_key(root, Some("user-passphrase")).unwrap();
        let token = encrypt(root, "secret", "app", None).unwrap();
        let key_file = std::fs::read_to_string(root.join(CRYPTO_USER_REL)).unwrap();
        assert!(!key_file.contains("user-passphrase"));
        assert_eq!(decrypt(root, &token, "app", None).unwrap(), "secret");
        assert!(decrypt(root, &token, "app", Some("different")).is_err());
        set_user_key(root, None).unwrap();
        assert!(decrypt(root, &token, "app", None).is_err());
    }
}
