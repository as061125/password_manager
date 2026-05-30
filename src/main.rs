#![windows_subsystem = "windows"]

// ============================================================================
// 密码本 —— 生产级加密密码管理器
//
// 项目结构：
//   main.rs  — 入口（自动判断 CLI/GUI 模式）
//   cli.rs   — 命令行模式
//   vault.rs — AES-256-GCM + PBKDF2 加密存储
//   model.rs — 数据结构
//   message.rs — 消息枚举 + PWM 命令解析（CLI 和搜索栏共享）
//   search.rs — FZF 模糊搜索
//   update.rs — 业务逻辑 + 状态转换
//   view.rs   — 界面渲染
// ============================================================================

mod cli;
mod vault;
mod model;
mod message;
mod search;
mod update;
mod view;

use std::time::Duration;

use iced::keyboard;
use iced::time;
use iced::{Font, Subscription, Task, Theme};

use crate::model::{LockedModel, Model};

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // 后台守护进程模式：等待父进程退出后清除 session
    if args.len() == 2 && args[0] == "--guard-session" {
        let session_file = std::env::temp_dir().join("pwm_session");
        if let Ok(ppid) = args[1].parse::<u32>() {
            #[cfg(windows)]
            {
                use std::ffi::c_void;
                extern "system" {
                    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut c_void;
                    fn WaitForSingleObject(hHandle: *mut c_void, dwMilliseconds: u32) -> u32;
                    fn CloseHandle(hObject: *mut c_void) -> i32;
                }
                const SYNCHRONIZE: u32 = 0x00100000;
                const INFINITE: u32 = 0xFFFFFFFF;

                unsafe {
                    let h = OpenProcess(SYNCHRONIZE, 0, ppid);
                    if !h.is_null() {
                        WaitForSingleObject(h, INFINITE);
                        CloseHandle(h);
                        let _ = std::fs::remove_file(&session_file);
                    }
                }
            }
            #[cfg(unix)]
            {
                // Unix: 用 kill -0 检查父进程是否存活
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(10));
                    let alive = std::process::Command::new("kill")
                        .args(["-0", &ppid.to_string()])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                    if !alive {
                        let _ = std::fs::remove_file(&session_file);
                        break;
                    }
                }
            }
        }
        return Ok(());
    }

    // 后台清除剪贴板模式（由 pwm 自身 spawn，60 秒后清除）
    if args.len() == 1 && args[0] == "--clear-clipboard" {
        std::thread::sleep(Duration::from_secs(60));
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.clear();
        }
        return Ok(());
    }

    // CLI 模式
    if !args.is_empty() {
        match cli::run(&args) {
            Ok(()) => {}
            Err(e) => eprintln!("错误: {e}"),
        }
        return Ok(());
    }

    // GUI 模式
    let vault_path = vault::default_vault_path();
    let _ = vault::ensure_vault_dir(&vault_path);
    let is_new = !vault::exists(&vault_path);

    iced::application("密码本", update::update, view::view)
        .theme(|_| Theme::Dark)
        .default_font(Font::with_name("Microsoft YaHei"))
        .subscription(subscription)
        .run_with(move || {
            (
                Model::Locked(LockedModel {
                    vault_path: vault_path.clone(),
                    master_password: String::new(),
                    error: None,
                    is_new,
                }),
                Task::none(),
            )
        })
}

fn subscription(model: &Model) -> Subscription<message::Message> {
    match model {
        Model::Locked(_) => Subscription::none(),
        Model::Unlocked(_) => {
            let kb = keyboard::on_key_press(|key, modifiers| {
                Some(message::Message::KeyPressed(key, modifiers))
            });
            // 每 10 秒检查无操作超时
            let timer = time::every(Duration::from_secs(10))
                .map(|_| message::Message::CheckInactivity);
            Subscription::batch([kb, timer])
        }
    }
}
