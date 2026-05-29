// ============================================================================
// CLI 入口 —— 命令行密码管理器
//
// 所有命令与搜索栏 <cmd> 共享同一套解析器（message::parse_pwm）。
// ============================================================================

use std::path::Path;

use crate::message::{self, PwmCommand};
use crate::model::{PasswordEntry, VaultSettings};
use crate::vault;

/// session 文件路径（OS 临时目录）
fn session_path() -> std::path::PathBuf {
    std::env::temp_dir().join("pwm_session")
}

/// 缓存主密码到 session 文件（owner-only 权限）
fn save_session(password: &str) {
    let path = session_path();
    if let Ok(mut f) = std::fs::File::create(&path) {
        use std::io::Write;
        let _ = f.write_all(password.as_bytes());
        let _ = f.sync_all();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        #[cfg(windows)]
        {
            // Windows 下无简单 owner-only 权限设置，依赖 temp 目录的隔离性
        }
        println!("主密码已缓存（本次 terminal session 有效）");
    }
}

/// 读取缓存的 session 密码
fn load_session() -> Option<String> {
    let path = session_path();
    std::fs::read_to_string(&path).ok().filter(|s| !s.is_empty())
}

/// 清除 session
fn clear_session() {
    let _ = std::fs::remove_file(session_path());
    println!("已登出，session 已清除");
}

/// 获取主密码：--password 参数 > session 缓存 > PWM_PASSWORD 环境变量 > 交互式输入
fn get_master_password(cli_pwd: Option<String>) -> String {
    if let Some(pwd) = cli_pwd {
        if !pwd.is_empty() { return pwd; }
    }
    if let Some(pwd) = load_session() {
        return pwd;
    }
    if let Ok(pwd) = std::env::var("PWM_PASSWORD") {
        if !pwd.is_empty() { return pwd; }
    }
    let pwd = rpassword::prompt_password("主密码: ").unwrap_or_default();
    save_session(&pwd);
    pwd
}

/// 检查是否有同名条目（大小写不敏感）
fn has_duplicate_name(entries: &[PasswordEntry], name: &str) -> bool {
    entries.iter().any(|e| e.name.to_lowercase() == name.trim().to_lowercase())
}

/// 确保控制台输出 UTF-8（Windows 默认代码页非 UTF-8）
fn init_utf8() {
    #[cfg(windows)]
    {
        // SetConsoleOutputCP(65001) — 不依赖外部 crate
        extern "system" {
            fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
        }
        unsafe { SetConsoleOutputCP(65001); }
    }
}

/// CLI 主入口，返回是否处理成功
pub fn run(args: &[String]) -> Result<(), String> {
    init_utf8();

    // 提取 --password 参数（可通过 --password=xxx 或 --password xxx 传入）
    let mut cmd_args: Vec<String> = Vec::new();
    let mut cli_password: Option<String> = None;
    let mut skip_next = false;
    for a in args {
        if skip_next { cli_password = Some(a.clone()); skip_next = false; continue; }
        if let Some(pwd) = a.strip_prefix("--password=") {
            cli_password = Some(pwd.to_string());
        } else if a == "--password" {
            skip_next = true; // 下一个参数是密码值
        } else {
            cmd_args.push(a.clone());
        }
    }

    let input = cmd_args.join(" ");
    let cmd = message::parse_pwm(&input).ok_or_else(|| {
        format!("未知命令。输入 `pwm help` 查看帮助\n{}", message::help_text())
    })?;

    // --version 和 help 不需要加载 vault
    match &cmd {
        PwmCommand::Version => {
            println!("PWM Password Manager v1.0.0");
            return Ok(());
        }
        PwmCommand::Help { subcommand } => {
            if subcommand.is_none() {
                print!("{}", message::help_text());
            } else {
                println!("`pwm help {}` — 暂未实现详细帮助", subcommand.as_ref().unwrap());
            }
            return Ok(());
        }
        // display/hide 仅在 GUI 搜索栏有效
        PwmCommand::Display | PwmCommand::Hide => {
            return Err("`display` 和 `hide` 命令仅在 GUI 搜索栏中有效".into());
        }
        PwmCommand::Login => {
            let pwd = rpassword::prompt_password("主密码: ").unwrap_or_default();
            save_session(&pwd);
            return Ok(());
        }
        PwmCommand::Logout => {
            clear_session();
            return Ok(());
        }
        _ => {}
    }

    // 加载 vault（先获取主密码，仅一次）
    let vault_path = vault::default_vault_path();
    let _ = vault::ensure_vault_dir(&vault_path);
    let password = get_master_password(cli_password);

    if !vault::exists(&vault_path) {
        // 如果是 add 命令且 vault 不存在，自动创建
        let is_add = matches!(cmd, PwmCommand::Add { .. });
        if !is_add {
            return Err("保险库不存在，请先通过 GUI 创建或导入".into());
        }
        vault::create(&vault_path, &password, &VaultSettings::default())
            .map_err(|e| format!("创建保险库失败: {e}"))?;
        println!("已创建新保险库");
    }

    let (mut content, salt, key) =
        vault::load(&vault_path, &password, false).map_err(|e| format!("加载失败: {e}"))?;

    // 执行命令
    match cmd {
        PwmCommand::List { plaintext } => {
            if content.entries.is_empty() {
                println!("（空）");
            }
            for e in &content.entries {
                if plaintext {
                    println!("{} : {}", e.name, e.password);
                } else {
                    let masked: String = e.password.chars().map(|_| '●').collect();
                    println!("{} : {}", e.name, masked);
                }
            }
        }
        PwmCommand::Get { ref name, fuzzy, copy, plaintext } => {
            let matched: Vec<_> = content
                .entries
                .iter()
                .filter(|e| {
                    if fuzzy { e.name.to_lowercase().contains(&name.to_lowercase()) }
                    else { e.name.to_lowercase() == name.to_lowercase() }
                })
                .collect();
            if matched.is_empty() {
                println!("未找到匹配: {}", name);
            } else {
                for e in &matched {
                    if plaintext {
                        println!("{} : {}", e.name, e.password);
                    } else if copy && fuzzy {
                        // -f -c：静默复制，只输出确认信息
                        println!("已复制: {}", e.name);
                    } else {
                        let masked: String = e.password.chars().map(|_| '●').collect();
                        println!("{} : {}", e.name, masked);
                    }
                }
                // 复制模式（非 -f -f 静默复制的情况也输出遮罩行，此处只额外复制）
                if copy && !(fuzzy && !plaintext) {
                    if let Some(e) = matched.first() {
                        if let Ok(mut cb) = arboard::Clipboard::new() {
                            let _ = cb.set_text(e.password.clone());
                        }
                        if !plaintext {
                            println!("(已复制到剪贴板)");
                        }
                    }
                }
            }
        }
        PwmCommand::Add { ref name, ref password } => {
            let pwd = if let Some(p) = password {
                // 命令行直接传了密码
                p.clone()
            } else {
                // 交互式选择：设置密码 / 自动生成
                print!("设置密码(s) / 自动生成(a) [a]: ");
                use std::io::Write;
                let _ = std::io::stdout().flush();
                let mut choice = String::new();
                std::io::stdin().read_line(&mut choice).ok();
                let choice = choice.trim().to_lowercase();

                if choice == "s" || choice == "设置" {
                    // 手动设置密码，需二次确认
                    loop {
                        let p1 = rpassword::prompt_password("密码: ").unwrap_or_default();
                        let p2 = rpassword::prompt_password("确认密码: ").unwrap_or_default();
                        if p1 == p2 && !p1.is_empty() {
                            break p1;
                        } else if p1 != p2 {
                            println!("两次输入的密码不一致，请重新输入");
                        } else {
                            println!("密码不能为空");
                        }
                    }
                } else {
                    // 自动生成
                    print!("密码位数 [16]: ");
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    let mut len_str = String::new();
                    std::io::stdin().read_line(&mut len_str).ok();
                    let len: usize = len_str.trim().parse().unwrap_or(16).clamp(4, 64);
                    let gen = crate::update::generate_strong_password(len);
                    let masked: String = gen.chars().map(|_| '●').collect();
                    println!("已生成密码: {} ({}位)", masked, len);
                    gen
                }
            };

            if has_duplicate_name(&content.entries, name) {
                println!("已存在同名密码: {}", name);
            } else {
                content.entries.push(crate::model::PasswordEntry {
                    name: name.clone(),
                    password: pwd,
                });
                vault::save(&vault_path, &salt, &key, &content)
                    .map_err(|e| format!("保存失败: {e}"))?;
                println!("已添加: {}", name);
            }
        }
        PwmCommand::Rm { ref name } => {
            let before = content.entries.len();
            content.entries.retain(|e| {
                !e.name.to_lowercase().contains(&name.to_lowercase())
            });
            let removed = before - content.entries.len();
            if removed == 0 {
                println!("未找到匹配: {}", name);
            } else {
                vault::save(&vault_path, &salt, &key, &content)
                    .map_err(|e| format!("保存失败: {e}"))?;
                println!("已删除 {} 个条目", removed);
            }
        }
        PwmCommand::Export { ref dir } => {
            let dst = if dir.is_empty() {
                Path::new("export_recovery")
            } else {
                Path::new(dir)
            };
            std::fs::create_dir_all(dst).map_err(|e| format!("创建目录失败: {e}"))?;
            vault::export_vault(&vault_path, dst, &salt, &key, &content)
                .map_err(|e| format!("导出失败: {e}"))?;
            println!("已导出到: {}", dst.display());
        }
        PwmCommand::Import { vault_path: ref vp_str, ref recovery_path } => {
            let vp = Path::new(vp_str);
            let rp = Path::new(recovery_path);
            let (imported, _salt2, _key2) =
                vault::import_vault(vp, rp).map_err(|e| format!("导入失败: {e}"))?;
            content.entries = imported.entries;
            vault::save(&vault_path, &salt, &key, &content)
                .map_err(|e| format!("保存失败: {e}"))?;
            println!("已导入 {} 个条目", content.entries.len());
        }
        PwmCommand::History => {
            let hdir = vault::history_dir(&vault_path);
            if let Ok(entries) = std::fs::read_dir(&hdir) {
                let mut files: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();
                files.sort();
                if files.is_empty() {
                    println!("暂无历史版本");
                } else {
                    for (i, f) in files.iter().enumerate() {
                        println!("{}. {}", i, f);
                    }
                }
            } else {
                println!("暂无历史版本");
            }
        }
        PwmCommand::Restore { index } => {
            let hdir = vault::history_dir(&vault_path);
            if let Ok(entries) = std::fs::read_dir(&hdir) {
                let mut files: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .collect();
                files.sort();
                if let Some(src) = files.get(index) {
                    let (hist_content, _s, _k) =
                        vault::load(src, &password, false)
                            .map_err(|e| format!("读取历史版本失败: {e}"))?;
                    content.entries = hist_content.entries;
                    vault::save(&vault_path, &salt, &key, &content)
                        .map_err(|e| format!("保存失败: {e}"))?;
                    println!("已恢复到历史版本 #{}", index);
                } else {
                    return Err(format!("无效的序号: {}", index));
                }
            } else {
                return Err("没有历史版本".into());
            }
        }
        PwmCommand::Mv { ref old_name, ref new_name } => {
            let mut found = false;
            for e in &mut content.entries {
                if e.name.to_lowercase() == old_name.to_lowercase() {
                    e.name = new_name.clone();
                    found = true;
                    break;
                }
            }
            if found {
                vault::save(&vault_path, &salt, &key, &content)
                    .map_err(|e| format!("保存失败: {e}"))?;
                println!("已重命名: {} → {}", old_name, new_name);
            } else {
                println!("未找到: {}", old_name);
            }
        }
        // 这些在 CLI 中不会匹配到（之前已处理）
        PwmCommand::Display | PwmCommand::Hide | PwmCommand::Help { .. }
        | PwmCommand::Version | PwmCommand::Login | PwmCommand::Logout => {}
    }

    Ok(())
}
