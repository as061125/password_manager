// ============================================================================
// update —— 所有业务逻辑 + 状态转换
// ============================================================================

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use iced::clipboard;
use iced::keyboard::{self, key::Named};
use iced::widget::{scrollable, text_input};
use iced::Task;
use std::path::Path;

use crate::{
    message::{self, Message, PwmCommand},
    model::{Model, PasswordEntry, UnlockedModel, VaultSettings},
    search, vault,
};

const ROW_HEIGHT: f32 = 36.0;

/// 检查条目名是否重复（大小写不敏感）
fn has_duplicate_name(entries: &[PasswordEntry], name: &str, skip_index: Option<usize>) -> bool {
    entries.iter().enumerate().any(|(i, e)| {
        Some(i) != skip_index && e.name.to_lowercase() == name.trim().to_lowercase()
    })
}

// ── 强密码生成 ──────────────────────────────────────────────────────────

pub(crate) fn generate_strong_password(length: usize) -> String {
    use rand::seq::SliceRandom;
    let lower: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
    let upper: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect();
    let digits: Vec<char> = "0123456789".chars().collect();
    let special: Vec<char> = "!@#$%^&*()-_=+[]{}|;:,.<>?".chars().collect();
    let all: Vec<char> = lower.iter().chain(upper.iter()).chain(digits.iter()).chain(special.iter()).copied().collect();
    let length = length.max(4);
    let mut rng = rand::thread_rng();
    let mut chars: Vec<char> = Vec::with_capacity(length);
    chars.push(*lower.choose(&mut rng).unwrap());
    chars.push(*upper.choose(&mut rng).unwrap());
    chars.push(*digits.choose(&mut rng).unwrap());
    chars.push(*special.choose(&mut rng).unwrap());
    for _ in 4..length { chars.push(*all.choose(&mut rng).unwrap()); }
    chars.shuffle(&mut rng);
    chars.into_iter().collect()
}

// ── 保存（内部辅助） ──────────────────────────────────────────────────────

fn persist(model: &UnlockedModel) -> Task<Message> {
    let path = model.vault_path.clone();
    let salt = model.vault_salt;
    let key = model.enc_key;
    let content = vault::VaultContent {
        version: 1,
        entries: model.entries.clone(),
        settings: vault::VaultSettings {
            history_count: model.settings.history_count,
            verify_hash: model.settings.verify_hash,
            pbkdf2_iter: model.settings.pbkdf2_iter,
        },
    };

    Task::perform(
        async move {
            std::thread::spawn(move || {
                let _ = vault::save(&path, &salt, &key, &content);
            });
        },
        |_| Message::LockClearError,
    )
}

// ── 主 update ──────────────────────────────────────────────────────────────

pub fn update(model: &mut Model, message: Message) -> Task<Message> {
    match model {
        Model::Locked(locked) => match message {
            Message::LockPasswordChanged(p) => {
                locked.master_password = p;
                Task::none()
            }
            Message::LockSubmit => {
                let pwd = locked.master_password.clone();
                let path = locked.vault_path.clone();
                let is_new = locked.is_new;

                if is_new {
                    match vault::create(&path, &pwd, &VaultSettings::default()) {
                        Ok((salt, key)) => {
                            let content = vault::VaultContent {
                                version: 1,
                                entries: vec![],
                                settings: VaultSettings::default(),
                            };
                            *model = Model::Unlocked(UnlockedModel::new(
                                vec![], salt, key, path, pwd.clone(), content.settings,
                            ));
                            Task::none()
                        }
                        Err(e) => {
                            locked.error = Some(e);
                            Task::none()
                        }
                    }
                } else {
                    match vault::load(&path, &pwd, false) {
                        Ok((content, salt, key)) => {
                            let settings = VaultSettings {
                                history_count: content.settings.history_count,
                                verify_hash: content.settings.verify_hash,
                                pbkdf2_iter: content.settings.pbkdf2_iter,
                            };
                            *model = Model::Unlocked(UnlockedModel::new(
                                content.entries, salt, key, path, pwd.clone(), settings,
                            ));
                            Task::none()
                        }
                        Err(e) => {
                            locked.error = Some(e);
                            Task::none()
                        }
                    }
                }
            }
            Message::LockClearError => {
                locked.error = None;
                Task::none()
            }
            _ => Task::none(),
        },

        Model::Unlocked(u) => {
            let result = match message {
                // ── 导航 ──
                Message::ToggleMenu => {
                    u.menu_open = !u.menu_open;
                    if !u.menu_open { u.settings_visible = false; u.show_history = false; }
                    None
                }
                Message::ToggleSettings => {
                    u.settings_visible = !u.settings_visible;
                    u.show_history = false;
                    if u.settings_visible {
                        u.settings_history_count = u.settings.history_count.to_string();
                        u.settings_verify = u.settings.verify_hash;
                        u.settings_pbkdf2_iter = u.settings.pbkdf2_iter.to_string();
                    }
                    None
                }
                Message::ToggleHistory => {
                    u.show_history = !u.show_history;
                    u.settings_visible = false;
                    if u.show_history {
                        // List history files
                        let hdir = vault::history_dir(&u.vault_path);
                        let mut files = vec![];
                        if let Ok(entries) = std::fs::read_dir(&hdir) {
                            for e in entries.flatten() {
                                if let Some(name) = e.file_name().to_str().map(String::from) {
                                    files.push(name);
                                }
                            }
                        }
                        files.sort();
                        u.history_files = files;
                    }
                    None
                }
                Message::ToggleEditMode => { u.edit_mode = !u.edit_mode; None }
                Message::ToggleCopy => { u.show_copy = !u.show_copy; None }
                Message::TogglePlaintext => { u.show_plaintext = !u.show_plaintext; None }

                // ── 搜索 ──
                Message::ToggleSearch => {
                    u.show_search = !u.show_search;
                    if u.show_search {
                        Some(Task::batch([
                            text_input::focus(text_input::Id::new("search")),
                            scrollable::scroll_to(u.scroll_id.clone(), scrollable::AbsoluteOffset { x: 0.0, y: 0.0 }),
                        ]))
                    } else {
                        u.search_query.clear(); u.search_results.clear(); u.best_match = None;
                        None
                    }
                }
                Message::SearchChanged(query) => {
                    u.search_query.clone_from(&query);
                    // <cmd> 命令不在按键时执行，仅清除搜索结果避免错误匹配
                    if query.starts_with("<cmd>") {
                        u.search_results.clear(); u.best_match = None;
                        return Task::none();
                    }
                    u.search_results = search::run_search(&query, &u.entries);
                    u.best_match = u.search_results.first().map(|&(i, _)| i);
                    if let Some(idx) = u.best_match {
                        return Task::batch([
                            scrollable::scroll_to(u.scroll_id.clone(), scrollable::AbsoluteOffset { x: 0.0, y: idx as f32 * ROW_HEIGHT }),
                            persist(u),
                        ]);
                    }
                    None
                }
                Message::SearchSubmitted => {
                    let is_cmd = u.search_query.starts_with("<cmd>");
                    if is_cmd {
                        if let Some(cmd) = message::parse_pwm(&u.search_query) {
                            match cmd {
                                PwmCommand::Display => u.show_plaintext = true,
                                PwmCommand::Hide => u.show_plaintext = false,
                                PwmCommand::List { plaintext } => {
                                    let mut lines: Vec<String> = u.entries.iter().map(|e| {
                                        if plaintext { format!("{} : {}", e.name, e.password) }
                                        else { format!("{} : {}", e.name, e.password.chars().map(|_| '●').collect::<String>()) }
                                    }).collect();
                                    if lines.is_empty() { lines.push("（空）".into()); }
                                    u.cmd_result_text = lines.join("\n");
                                    u.cmd_result_visible = true;
                                }
                                PwmCommand::Get { ref name, .. } => {
                                    let matched: Vec<_> = u.entries.iter().filter(|e|
                                        e.name.to_lowercase().contains(&name.to_lowercase())
                                    ).collect();
                                    if matched.is_empty() {
                                        u.cmd_result_text = format!("未找到: {}", name);
                                    } else {
                                        u.cmd_result_text = matched.iter().map(|e| format!("{} : {}", e.name, e.password)).collect::<Vec<_>>().join("\n");
                                    }
                                    u.cmd_result_visible = true;
                                }
                                PwmCommand::Add { ref name, ref password } => {
                                    if has_duplicate_name(&u.entries, name, None) {
                                        u.cmd_result_text = format!("已存在同名密码: {}", name);
                                        u.cmd_result_visible = true;
                                    } else {
                                        u.show_add_dialog = true;
                                        u.new_name = name.clone();
                                        u.new_password = password.clone().unwrap_or_default();
                                        u.confirm_password.clear();
                                        u.new_name_error = None;
                                    }
                                }
                                PwmCommand::Rm { ref name } => {
                                    let before = u.entries.len();
                                    u.entries.retain(|e| !e.name.to_lowercase().contains(&name.to_lowercase()));
                                    let removed = before - u.entries.len();
                                    u.cmd_result_text = format!("已删除 {} 条", removed);
                                    u.cmd_result_visible = true;
                                    if removed > 0 { return Task::batch([persist(u), clipboard::write(u.cmd_result_text.clone())]); }
                                }
                                PwmCommand::Mv { ref old_name, ref new_name } => {
                                    let src_idx = u.entries.iter().position(|e| e.name.to_lowercase() == old_name.to_lowercase());
                                    match src_idx {
                                        None => { u.cmd_result_text = format!("未找到: {}", old_name); }
                                        Some(idx) if has_duplicate_name(&u.entries, new_name, Some(idx)) => {
                                            u.cmd_result_text = format!("目标名称已存在: {}", new_name);
                                        }
                                        Some(idx) => {
                                            u.entries[idx].name = new_name.clone();
                                            u.cmd_result_text = format!("已重命名: {} → {}", old_name, new_name);
                                            u.cmd_result_visible = true;
                                            return persist(u);
                                        }
                                    }
                                    u.cmd_result_visible = true;
                                }
                                PwmCommand::Help { .. } => {
                                    u.cmd_result_text = message::help_text().to_string();
                                    u.cmd_result_visible = true;
                                }
                                PwmCommand::Version => {
                                    u.cmd_result_text = "PWM v1.0.0".into();
                                    u.cmd_result_visible = true;
                                }
                                PwmCommand::Login | PwmCommand::Logout
                                | PwmCommand::Export { .. } | PwmCommand::Import { .. }
                                | PwmCommand::History | PwmCommand::Restore { .. } => {
                                    u.cmd_result_text = "该命令仅在 CLI 中可用".into();
                                    u.cmd_result_visible = true;
                                }
                            }
                        }
                    }
                    u.show_search = false; u.search_query.clear(); u.search_results.clear(); u.best_match = None;
                    None
                }

                // ── 添加密码 ──
                Message::HideAddDialog => { u.show_add_dialog = false; u.confirm_password.clear(); None }
                Message::NewNameChanged(val) => { u.new_name = val; u.new_name_error = None; None }
                Message::NewPasswordChanged(val) => { u.new_password = val.chars().filter(|c| c.is_ascii()).collect(); None }
                Message::ConfirmPasswordChanged(val) => { u.confirm_password = val.chars().filter(|c| c.is_ascii()).collect(); None }
                Message::RandomLengthChanged(val) => { u.random_length = val.chars().filter(|c| c.is_ascii_digit()).collect(); None }
                Message::GeneratePassword => {
                    let len: usize = u.random_length.parse().unwrap_or(16);
                    let pwd = generate_strong_password(len);
                    u.new_password.clone_from(&pwd);
                    u.confirm_password = pwd;
                    None
                }
                Message::SubmitNewPassword => {
                    let name = u.new_name.trim().to_string();
                    if name.is_empty() || u.new_password.is_empty() || u.new_password != u.confirm_password {
                        u.show_add_dialog = false; u.confirm_password.clear(); u.new_name_error = None;
                        return Task::none();
                    }
                    if has_duplicate_name(&u.entries, &name, None) {
                        u.new_name_error = Some("已存在同名密码".into());
                        return Task::none();
                    }
                    u.entries.push(PasswordEntry { name, password: u.new_password.clone() });
                    if !u.search_query.is_empty() {
                        u.search_results = search::run_search(&u.search_query, &u.entries);
                        u.best_match = u.search_results.first().map(|&(i, _)| i);
                    }
                    u.show_add_dialog = false; u.confirm_password.clear(); u.new_name_error = None;
                    Some(persist(u))
                }
                Message::EditEntryPassword(idx, val) => {
                    if idx < u.entries.len() {
                        u.entries[idx].password = val.chars().filter(|c| c.is_ascii()).collect();
                    }
                    Some(persist(u))
                }
                Message::DeleteEntry(idx) => {
                    if idx < u.entries.len() {
                        use zeroize::Zeroize;
                        u.entries[idx].password.zeroize();
                        u.entries.remove(idx);
                        if !u.search_query.is_empty() {
                            u.search_results = search::run_search(&u.search_query, &u.entries);
                            u.best_match = u.search_results.first().map(|&(i, _)| i);
                        }
                    }
                    Some(persist(u))
                }
                Message::CopyPassword(idx) => {
                    if let Some(entry) = u.entries.get(idx) {
                        // 生成后台进程，60 秒后自动清除剪贴板
                        if let Ok(exe) = std::env::current_exe() {
                            let mut cmd = std::process::Command::new(exe);
                            cmd.arg("--clear-clipboard");
                            #[cfg(windows)]
                            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                            let _ = cmd.spawn();
                        }
                        return clipboard::write(entry.password.clone());
                    }
                    None
                }
                Message::CheckClipboardTimer => { None }

                // ── 设置 ──
                Message::SettingsHistoryCountChanged(val) => {
                    u.settings_history_count = val.chars().filter(|c| c.is_ascii_digit()).collect();
                    None
                }
                Message::SettingsVerifyToggled(v) => { u.settings_verify = v; None }
                Message::SettingsPbkdf2IterChanged(val) => {
                    u.settings_pbkdf2_iter = val.chars().filter(|c| c.is_ascii_digit()).collect();
                    None
                }
                Message::SettingsSave => {
                    let hc: usize = u.settings_history_count.parse().unwrap_or(5).clamp(0, 10);
                    u.settings.history_count = hc;
                    u.settings.verify_hash = u.settings_verify;
                    let old_iter = u.settings.pbkdf2_iter;
                    let iter: u32 = u.settings_pbkdf2_iter.parse().unwrap_or(100_000).clamp(10_000, 10_000_000);
                    u.settings.pbkdf2_iter = iter;

                    u.settings_visible = false;
                    // 如果迭代次数改变了，需要用主密码重新派生密钥
                    if iter != old_iter && !u.master_password.is_empty() {
                        let path = u.vault_path.clone();
                        let pwd = u.master_password.clone();
                        let salt = u.vault_salt;
                        let content = vault::VaultContent {
                            version: 1,
                            entries: u.entries.clone(),
                            settings: vault::VaultSettings {
                                history_count: u.settings.history_count,
                                verify_hash: u.settings.verify_hash,
                                pbkdf2_iter: iter,
                            },
                        };
                        match vault::reencrypt(&path, &pwd, iter, &content, &salt) {
                            Ok(new_key) => { u.enc_key = new_key; }
                            Err(_) => { u.settings.pbkdf2_iter = old_iter; }
                        }
                        None
                    } else {
                        Some(persist(u))
                    }
                }

                // ── 导出 ──
                Message::ShowExportDialog => { u.show_export_dialog = true; u.export_status = None; None }
                Message::HideExportDialog => { u.show_export_dialog = false; u.export_status = None; None }
                Message::ExportSubmit => {
                    let path = u.vault_path.clone();
                    let salt = u.vault_salt;
                    let key = u.enc_key;
                    let content = vault::VaultContent {
                        version: 1,
                        entries: u.entries.clone(),
                        settings: vault::VaultSettings { history_count: u.settings.history_count, verify_hash: u.settings.verify_hash, pbkdf2_iter: u.settings.pbkdf2_iter },
                    };
                    // 导出到 vault 同级目录下的 export/ 文件夹
                    let export_dir = path.parent().unwrap_or(Path::new(".")).join("export_recovery");
                    let _ = std::fs::create_dir_all(&export_dir);
                    match vault::export_vault(&path, &export_dir, &salt, &key, &content) {
                        Ok(()) => { u.export_status = Some(format!("已导出到: {}", export_dir.display())); }
                        Err(e) => { u.export_status = Some(format!("导出失败: {e}")); }
                    }
                    None
                }

                // ── 导入 ──
                Message::ShowImportDialog => { u.show_import_dialog = true; u.import_status = None; None }
                Message::HideImportDialog => { u.show_import_dialog = false; u.import_status = None; None }
                Message::ImportVaultPathChanged(val) => { u.import_vault_path = val; None }
                Message::ImportRecoveryPathChanged(val) => { u.import_recovery_path = val; None }
                Message::ImportSubmit => {
                    let vp = Path::new(&u.import_vault_path);
                    let rp = Path::new(&u.import_recovery_path);
                    match vault::import_vault(vp, rp) {
                        Ok((content, salt, key)) => {
                            let settings = VaultSettings { history_count: content.settings.history_count, verify_hash: content.settings.verify_hash, pbkdf2_iter: content.settings.pbkdf2_iter };
                            u.entries = content.entries;
                            u.vault_salt = salt;
                            u.enc_key = key;
                            u.settings = settings;
                            u.show_import_dialog = false;
                            u.import_status = None;
                            return persist(u);
                        }
                        Err(e) => { u.import_status = Some(format!("导入失败: {e}")); None }
                    }
                }

                // ── 历史版本 ──
                Message::RestoreHistory(idx) => {
                    let hdir = vault::history_dir(&u.vault_path);
                    if let Ok(entries) = std::fs::read_dir(&hdir) {
                        let mut files: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
                        files.sort();
                        if idx < files.len() {
                            let src = &files[idx];
                            // Load from history file (same password-derived key, same salt)
                            match vault::load(src, &u.master_password, false) {
                                Ok((content, _salt, _key)) => {
                                    u.entries = content.entries;
                                    return persist(u);
                                }
                                Err(e) => { u.import_status = Some(format!("恢复失败: {e}")); }
                            }
                        }
                    }
                    None
                }

                Message::HideCmdResult => { u.cmd_result_visible = false; None }

                // ── 键盘 ──
                Message::KeyPressed(key, modifiers) => match key {
                    keyboard::Key::Named(Named::Escape) => {
                        if u.show_add_dialog { u.show_add_dialog = false; u.confirm_password.clear(); }
                        else if u.cmd_result_visible { u.cmd_result_visible = false; }
                        else if u.settings_visible { u.settings_visible = false; }
                        else if u.show_export_dialog { u.show_export_dialog = false; }
                        else if u.show_import_dialog { u.show_import_dialog = false; }
                        else if u.show_history { u.show_history = false; }
                        else if u.show_search { u.show_search = false; u.search_query.clear(); u.search_results.clear(); u.best_match = None; }
                        None
                    }
                    _ if modifiers.command() && key == keyboard::Key::Character("/".into()) => {
                        if !u.show_add_dialog { return update(model, Message::ToggleSearch); }
                        None
                    }
                    _ if modifiers.command() && key == keyboard::Key::Character("n".into()) => {
                        if !u.show_add_dialog {
                            u.show_add_dialog = true; u.new_name.clear(); u.new_password.clear(); u.confirm_password.clear();
                        }
                        None
                    }
                    _ if modifiers.command() && key == keyboard::Key::Character("i".into()) => {
                        if !u.show_add_dialog { return update(model, Message::ToggleEditMode); }
                        None
                    }
                    _ if modifiers.command() && key == keyboard::Key::Character("c".into()) => {
                        if !u.show_add_dialog { return update(model, Message::ToggleCopy); }
                        None
                    }
                    _ if modifiers.command() && key == keyboard::Key::Character("d".into()) => {
                        if !u.show_add_dialog { return update(model, Message::TogglePlaintext); }
                        None
                    }
                    _ => None,
                },

                _ => None,
            };

            match result {
                Some(task) => task,
                None => Task::none(),
            }
        }
    }
}
