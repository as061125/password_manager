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

use std::path::PathBuf;

use iced::keyboard;
use iced::{Font, Subscription, Task, Theme};

use crate::model::{LockedModel, Model};

const VAULT_FILE: &str = "passwords.vault";

fn main() -> iced::Result {
    // 检测 CLI 模式：有命令行参数且不是由 Iced 启动
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        match cli::run(&args) {
            Ok(()) => {}
            Err(e) => eprintln!("错误: {e}"),
        }
        return Ok(());
    }

    // GUI 模式
    let vault_path = PathBuf::from(VAULT_FILE);
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
            keyboard::on_key_press(|key, modifiers| {
                Some(message::Message::KeyPressed(key, modifiers))
            })
        }
    }
}
