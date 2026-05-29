// ============================================================================
// 加密保险库 —— AES-256-GCM + PBKDF2
//
// 新增功能：
//   - 原子写入（写 tmp → fsync → rename）
//   - SHA-256 校验（.vault.meta 文件）
//   - 时间戳历史备份（保留 N 份，N 可配 0~10）
//   - 导出恢复包（vault + recovery key）
//   - 从恢复包导入（不依赖主密码）
// ============================================================================

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::PasswordEntry;

pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;
pub const KEY_LEN: usize = 32;
const PBKDF2_ITER: u32 = 100_000;

// ── 序列化结构 ─────────────────────────────────────────────────────────────

/// 加密载荷：密码条目 + 设置（每次加密/解密这个结构）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VaultContent {
    pub version: u32,
    pub entries: Vec<PasswordEntry>,
    pub settings: VaultSettings,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VaultSettings {
    pub history_count: usize,
    pub verify_hash: bool,
}

impl Default for VaultSettings {
    fn default() -> Self {
        Self {
            history_count: 5,
            verify_hash: true,
        }
    }
}

// ── 密钥派生 ───────────────────────────────────────────────────────────────

pub fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITER, &mut key);
    key
}

// ── 原子写入: tmp → fsync → rename ──────────────────────────────────────

fn atomic_write(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("vault.tmp");
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("创建临时文件失败: {e}"))?;
        f.write_all(data).map_err(|e| format!("写入临时文件失败: {e}"))?;
        f.sync_all().map_err(|e| format!("fsync 失败: {e}"))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("重命名文件失败: {e}"))
}

// ── SHA-256 ────────────────────────────────────────────────────────────────

pub fn sha256(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

fn meta_path(path: &Path) -> PathBuf {
    let mut p = path.to_path_buf();
    let name = format!(
        "{}.meta",
        path.file_stem().unwrap_or_default().to_string_lossy()
    );
    p.set_file_name(name);
    p
}

pub fn history_dir(path: &Path) -> PathBuf {
    let mut p = path.to_path_buf();
    if let Some(parent) = path.parent() {
        p = parent.to_path_buf();
    }
    p.join("vault_history")
}

/// 时间戳字符串
fn timestamp() -> String {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", since.as_secs())
}

// ── 加密 + 保存（原子 + meta + 历史） ──────────────────────────────────────

/// 加密 content 并原子写入，同时更新 meta 和历史备份
pub fn save(
    path: &Path,
    salt: &[u8; SALT_LEN],
    key: &[u8; KEY_LEN],
    content: &VaultContent,
) -> Result<(), String> {
    // 1. 序列化 + 加密
    let plaintext = serde_json::to_string(content).map_err(|e| e.to_string())?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let aes_key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("加密失败: {e}"))?;

    // 2. 拼装二进制: salt || nonce || ciphertext
    let mut buf = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    buf.extend_from_slice(salt);
    buf.extend_from_slice(&nonce_bytes);
    buf.extend_from_slice(&ciphertext);

    // 3. 保存当前主文件的历史备份（在覆盖之前）
    let hc = content.settings.history_count;
    if hc > 0 && path.exists() {
        let hdir = history_dir(path);
        let _ = fs::create_dir_all(&hdir);
        let stamp = timestamp();
        let hist_path = hdir.join(format!("vault.{}", stamp));
        let _ = fs::copy(path, &hist_path);
        // 清理旧历史（保留 hc 份）
        if let Ok(entries) = std::fs::read_dir(&hdir) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map_or(false, |e| e != "tmp"))
                .collect();
            files.sort();
            while files.len() > hc {
                if let Some(oldest) = files.first() {
                    let _ = fs::remove_file(oldest);
                }
                files.remove(0);
            }
        }
    }

    // 4. 原子写入主文件
    atomic_write(path, &buf)?;

    // 5. 更新 .vault.meta
    if content.settings.verify_hash {
        let hash = sha256(&buf);
        let meta_p = meta_path(path);
        let _ = fs::write(&meta_p, format!("sha256:{}\n", hash));
    }

    Ok(())
}

// ── 解密 + 加载（含 hash 校验） ────────────────────────────────────────────

/// 读取、解密并校验 vault 文件，返回 (VaultContent, salt, derived_key)
pub fn load(
    path: &Path,
    password: &str,
    verify_hash: bool,
) -> Result<(VaultContent, [u8; SALT_LEN], [u8; KEY_LEN]), String> {
    // 读取文件
    let data = std::fs::read(path).map_err(|e| format!("读取文件失败: {e}"))?;

    // 可选：hash 校验
    if verify_hash {
        let meta_p = meta_path(path);
        if let Ok(meta_str) = std::fs::read_to_string(&meta_p) {
            if let Some(stored) = meta_str.trim().strip_prefix("sha256:") {
                let actual = sha256(&data);
                if actual != stored {
                    return Err("文件已被篡改或损坏（hash 不匹配）".into());
                }
            }
        }
        // meta 文件不存在则静默跳过
    }

    if data.len() < SALT_LEN + NONCE_LEN {
        return Err("文件格式错误".into());
    }

    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&data[..SALT_LEN]);
    let nonce_bytes = &data[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &data[SALT_LEN + NONCE_LEN..];

    let key = derive_key(password, &salt);
    let aes_key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "主密码错误或数据损坏".to_string())?;

    let content: VaultContent =
        serde_json::from_slice(&plaintext).map_err(|e| e.to_string())?;

    Ok((content, salt, key))
}

// ── 创建新保险库 ───────────────────────────────────────────────────────────

pub fn create(
    path: &Path,
    password: &str,
    settings: &VaultSettings,
) -> Result<([u8; SALT_LEN], [u8; KEY_LEN]), String> {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let key = derive_key(password, &salt);
    let content = VaultContent {
        version: 1,
        entries: vec![],
        settings: settings.clone(),
    };
    save(path, &salt, &key, &content)?;
    Ok((salt, key))
}

// ── 文件存在性 ─────────────────────────────────────────────────────────────

pub fn exists(path: &Path) -> bool {
    path.exists()
}

// ── 导出恢复包 ─────────────────────────────────────────────────────────────

/// 导出 vault 文件 + recovery key 到指定目录
pub fn export_vault(
    _src_path: &Path,
    dst_dir: &Path,
    salt: &[u8; SALT_LEN],
    key: &[u8; KEY_LEN],
    content: &VaultContent,
) -> Result<(), String> {
    // 1. 写入 vault 文件
    let vault_dst = dst_dir.join("passwords.vault");
    save(&vault_dst, salt, key, content)?;

    // 2. 写入 recovery key（salt || derived_key）
    let recovery_dst = dst_dir.join("passwords.recovery");
    let mut rbuf = Vec::with_capacity(SALT_LEN + KEY_LEN);
    rbuf.extend_from_slice(salt);
    rbuf.extend_from_slice(key);
    std::fs::write(&recovery_dst, &rbuf)
        .map_err(|e| format!("写入 recovery 文件失败: {e}"))?;

    Ok(())
}

/// 从恢复包导入（用 recovery key 解密，不需主密码）
pub fn import_vault(
    vault_path: &Path,
    recovery_path: &Path,
) -> Result<(VaultContent, [u8; SALT_LEN], [u8; KEY_LEN]), String> {
    let data = std::fs::read(vault_path).map_err(|e| format!("读取 vault 失败: {e}"))?;
    if data.len() < SALT_LEN + NONCE_LEN {
        return Err("vault 文件格式错误".into());
    }

    // 从 recovery 文件读取 salt + key
    let recovery = std::fs::read(recovery_path).map_err(|e| format!("读取 recovery 文件失败: {e}"))?;
    if recovery.len() < SALT_LEN + KEY_LEN {
        return Err("recovery 文件格式错误".into());
    }

    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&recovery[..SALT_LEN]);
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&recovery[SALT_LEN..SALT_LEN + KEY_LEN]);

    let nonce_bytes = &data[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &data[SALT_LEN + NONCE_LEN..];

    let aes_key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "恢复文件与 vault 不匹配或数据损坏".to_string())?;

    let content: VaultContent =
        serde_json::from_slice(&plaintext).map_err(|e| e.to_string())?;

    Ok((content, salt, key))
}
