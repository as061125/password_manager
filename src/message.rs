// ============================================================================
// 消息枚举 + PWM 命令解析（CLI 和搜索栏共享同一套语法）
// ============================================================================

use iced::keyboard;

/// 所有用户操作
#[derive(Debug, Clone)]
pub enum Message {
    // ── 锁定界面 ──
    LockPasswordChanged(String),
    LockSubmit,
    LockClearError,

    // ── 解锁后的操作 ──
    ToggleMenu,
    ToggleSearch,
    TogglePlaintext,
    ToggleEditMode,
    ToggleCopy,
    SearchChanged(String),
    SearchSubmitted,
    HideAddDialog,
    NewNameChanged(String),
    NewPasswordChanged(String),
    ConfirmPasswordChanged(String),
    SubmitNewPassword,
    RandomLengthChanged(String),
    GeneratePassword,
    DeleteEntry(usize),
    EditEntryPassword(usize, String),
    CopyPassword(usize),

    // ── 设置 ──
    ToggleSettings,
    SettingsHistoryCountChanged(String),
    SettingsVerifyToggled(bool),
    SettingsSave,

    // ── 导出 ──
    ShowExportDialog,
    HideExportDialog,
    ExportSubmit,

    // ── 导入 ──
    ShowImportDialog,
    HideImportDialog,
    ImportVaultPathChanged(String),
    ImportRecoveryPathChanged(String),
    ImportSubmit,

    // ── 定时器 ──
    CheckClipboardTimer,

    // ── <cmd> 命令结果 ──
    HideCmdResult,

    // ── 历史版本 ──
    ToggleHistory,
    RestoreHistory(usize),

    // ── 搜索栏 <cmd> 统一入口 ──
    PwmCommand(PwmCommand),

    // ── 键盘 ──
    KeyPressed(keyboard::Key, keyboard::Modifiers),
}

// ════════════════════════════════════════════════════════════════════════════
// PWM 命令枚举 —— CLI 和搜索栏共享
// ════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum PwmCommand {
    /// 明文显示 / 恢复遮罩（仅搜索栏）
    Display,
    Hide,
    /// 列出所有
    List { plaintext: bool },
    /// 获取指定条目
    Get { name: String, fuzzy: bool, copy: bool, plaintext: bool },
    /// 添加
    Add { name: String, password: Option<String> },
    /// 删除
    Rm { name: String },
    /// 导出恢复包
    Export { dir: String },
    /// 从恢复包导入
    Import { vault_path: String, recovery_path: String },
    /// 历史版本列表
    History,
    /// 回滚到指定历史版本
    Restore { index: usize },
    /// 登录（缓存主密码到当前 terminal session）
    Login,
    /// 登出（清除缓存的 session）
    Logout,
    /// 重命名
    Mv { old_name: String, new_name: String },
    /// 帮助
    Help { subcommand: Option<String> },
    /// 版本
    Version,
}

/// 统一解析器：搜索栏以 `<cmd>` 开头，CLI 直接传参
pub fn parse_pwm(input: &str) -> Option<PwmCommand> {
    let input = input.trim();
    // 搜索栏模式：去掉 <cmd> 前缀
    let cmdline = if input.starts_with("<cmd>") {
        input[5..].trim()
    } else {
        input
    };

    let parts: Vec<&str> = cmdline.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    match parts[0] {
        // ── 显示控制 ──
        "display" | "unhide" => Some(PwmCommand::Display),
        "hide" => Some(PwmCommand::Hide),

        // ── 数据操作 ──
        "list" | "ls" => {
            let plaintext = parts.iter().any(|&p| p == "--plaintext" || p == "-p");
            Some(PwmCommand::List { plaintext })
        }
        "get" => {
            let name = parts.get(1)?.to_string();
            let fuzzy = parts.iter().any(|&p| p == "--fuzzy" || p == "-f");
            let copy = parts.iter().any(|&p| p == "--copy" || p == "-c");
            let plaintext = parts.iter().any(|&p| p == "--plaintext" || p == "-p");
            Some(PwmCommand::Get { name, fuzzy, copy, plaintext })
        }
        "add" => {
            let name = parts.get(1)?.to_string();
            let password = parts.get(2).map(|s| s.to_string());
            Some(PwmCommand::Add { name, password })
        }
        "rm" | "remove" | "delete" => {
            let name = parts.get(1)?.to_string();
            Some(PwmCommand::Rm { name })
        }
        "login" => Some(PwmCommand::Login),
        "logout" => Some(PwmCommand::Logout),
        "mv" | "rename" => {
            let old_name = parts.get(1)?.to_string();
            let new_name = parts.get(2)?.to_string();
            Some(PwmCommand::Mv { old_name, new_name })
        }

        // ── 导出/导入 ──
        "export" => {
            let dir = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
            Some(PwmCommand::Export { dir })
        }
        "import" => {
            let vault_path = parts.get(1)?.to_string();
            let recovery_path = parts.get(2)?.to_string();
            Some(PwmCommand::Import { vault_path, recovery_path })
        }

        // ── 历史 ──
        "history" | "hist" => Some(PwmCommand::History),
        "restore" => {
            let index: usize = parts.get(1)?.parse().ok()?;
            Some(PwmCommand::Restore { index })
        }

        // ── 元操作 ──
        "help" => {
            let subcommand = parts.get(1).map(|s| s.to_string());
            Some(PwmCommand::Help { subcommand })
        }
        "version" | "--version" | "-v" => Some(PwmCommand::Version),

        _ => None,
    }
}


/// 生成帮助文本
pub fn help_text() -> &'static str {
    r#"PWM — Password Manager

用法（CLI）:  pwm [全局选项] <子命令> [参数]
用法（搜索栏）: <cmd><子命令> [参数]

全局选项:
  --password <密码>   直接传入主密码（优先级最高）
  --password=<密码>   等号语法

子命令:
  display             明文显示密码（仅搜索栏）
  hide                恢复遮罩（仅搜索栏）
  login               登录并缓存主密码（本次 terminal 有效）
  logout              登出清除缓存
  list, ls           列出所有条目
    -p, --plaintext   明文显示
  get <name>          查看条目（默认严格+遮罩）
    -f, --fuzzy       模糊匹配
    -p, --plaintext   明文显示
    -c, --copy        复制到剪贴板（-f -c 静默复制）
  add <name> [密码]   添加条目
                        未指定密码时交互式选择：
                          设置密码(s) — 两次确认
                          自动生成(a) — 可选位数(4-64)
  rm, remove <name>   删除条目
  mv, rename <旧> <新> 重命名
  export [目录]       导出恢复包
  import <vault> <recovery>  从恢复包导入
  history, hist       查看历史版本
  restore <序号>      回滚到指定历史版本
  help [子命令]       显示帮助
  version, -v         显示版本号

密码来源（优先级）:
  1. --password 参数
  2. 已缓存的 session（pwm login）
  3. PWM_PASSWORD 环境变量
  4. 交互式输入
"#
}
