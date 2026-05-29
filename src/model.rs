// ============================================================================
// 数据结构
// ============================================================================

use std::path::PathBuf;

use iced::widget::scrollable;

pub use crate::vault::VaultSettings;

/// 一条密码记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PasswordEntry {
    pub name: String,
    pub password: String,
}

/// 锁定状态 —— 输入主密码解锁或创建
#[derive(Debug, Clone)]
pub struct LockedModel {
    pub vault_path: PathBuf,
    pub master_password: String,
    pub error: Option<String>,
    pub is_new: bool,
}

/// 解锁后的完整应用状态
#[derive(Debug, Clone)]
pub struct UnlockedModel {
    pub vault_path: PathBuf,
    pub vault_salt: [u8; 16],
    pub enc_key: [u8; 32],
    /// 保留主密码，用于从历史备份恢复（每个历史文件有不同盐值）
    pub master_password: String,
    pub settings: VaultSettings,
    pub entries: Vec<PasswordEntry>,
    pub menu_open: bool,
    pub edit_mode: bool,
    pub show_copy: bool,
    pub show_plaintext: bool,
    pub show_search: bool,
    pub search_query: String,
    pub search_results: Vec<(usize, u32)>,
    pub best_match: Option<usize>,
    pub show_add_dialog: bool,
    pub new_name: String,
    pub new_name_error: Option<String>,
    pub new_password: String,
    pub confirm_password: String,
    pub random_length: String,
    // ── 设置面板 ──
    pub settings_visible: bool,
    pub settings_history_count: String,
    pub settings_verify: bool,
    pub settings_pbkdf2_iter: String,
    // ── 导出 ──
    pub show_export_dialog: bool,
    pub export_status: Option<String>,
    // ── 导入 ──
    pub show_import_dialog: bool,
    pub import_vault_path: String,
    pub import_recovery_path: String,
    pub import_status: Option<String>,
    // ── <cmd> 命令结果 ──
    pub cmd_result_visible: bool,
    pub cmd_result_text: String,
    // ── 历史版本列表 ──
    pub show_history: bool,
    pub history_files: Vec<String>,
    // ── 滚动 ──
    pub scroll_id: scrollable::Id,
}

impl UnlockedModel {
    pub fn new(
        entries: Vec<PasswordEntry>,
        vault_salt: [u8; 16],
        enc_key: [u8; 32],
        vault_path: PathBuf,
        master_password: String,
        settings: VaultSettings,
    ) -> Self {
        Self {
            vault_path,
            vault_salt,
            enc_key,
            master_password,
            settings: settings.clone(),
            entries,
            menu_open: false,
            edit_mode: false,
            show_copy: false,
            show_plaintext: false,
            show_search: false,
            search_query: String::new(),
            search_results: vec![],
            best_match: None,
            show_add_dialog: false,
            new_name: String::new(),
            new_name_error: None,
            new_password: String::new(),
            confirm_password: String::new(),
            random_length: String::from("16"),
            settings_visible: false,
            settings_history_count: settings.history_count.to_string(),
            settings_verify: settings.verify_hash,
            settings_pbkdf2_iter: settings.pbkdf2_iter.to_string(),
            show_export_dialog: false,
            export_status: None,
            show_import_dialog: false,
            import_vault_path: String::new(),
            import_recovery_path: String::new(),
            import_status: None,
            cmd_result_visible: false,
            cmd_result_text: String::new(),
            show_history: false,
            history_files: vec![],
            scroll_id: scrollable::Id::unique(),
        }
    }
}

/// 顶层状态
#[derive(Debug)]
pub enum Model {
    Locked(LockedModel),
    Unlocked(UnlockedModel),
}
