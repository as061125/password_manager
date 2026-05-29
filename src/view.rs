// ============================================================================
// view —— 所有界面渲染
// ============================================================================

use iced::widget::{
    button, center, column, container, row, scrollable, text, text_input, toggler, Stack,
};
use iced::{alignment, Center, Color, Element, Fill, Theme};

use crate::message::Message;
use crate::model::{LockedModel, Model, PasswordEntry, UnlockedModel};

const MASK: char = '●';

pub fn view(model: &Model) -> Element<'_, Message> {
    match model {
        Model::Locked(locked) => lock_view(locked),
        Model::Unlocked(unlocked) => unlocked_view(unlocked),
    }
}

// ════════════════════════════════════════════════════════════════════════════
// 锁定界面
// ════════════════════════════════════════════════════════════════════════════

fn lock_view(locked: &LockedModel) -> Element<'_, Message> {
    let title = if locked.is_new { "创建主密码" } else { "解锁密码本" };
    let hint = if locked.is_new { "设置一个主密码用于加密保险库" } else { "输入主密码解锁" };
    let mut col = column![
        text(title).size(28),
        text(hint).color(Color::from_rgb(0.6, 0.6, 0.6)),
        text_input("主密码", &locked.master_password)
            .on_input(Message::LockPasswordChanged)
            .on_submit(Message::LockSubmit)
            .secure(true).padding(12).width(320),
    ].spacing(12).align_x(Center);
    if let Some(err) = &locked.error {
        col = col.push(container(text(err).color(Color::from_rgb(0.9, 0.3, 0.3))).padding([10.0, 0.0]));
    }
    col = col.push(button(if locked.is_new { "创建" } else { "解锁" }).on_press(Message::LockSubmit).style(button::primary).padding([10.0, 40.0]));
    center(col).into()
}

// ════════════════════════════════════════════════════════════════════════════
// 解锁后的界面
// ════════════════════════════════════════════════════════════════════════════

fn unlocked_view(model: &UnlockedModel) -> Element<'_, Message> {
    // 按优先级叠加弹窗
    if model.show_add_dialog {
        return with_overlay(main_area(model), center(add_dialog(model)).into());
    }
    if model.show_export_dialog {
        return with_overlay(main_area(model), center(export_dialog(model)).into());
    }
    if model.show_import_dialog {
        return with_overlay(main_area(model), center(import_dialog(model)).into());
    }
    if model.cmd_result_visible {
        return with_overlay(main_area(model), center(cmd_result_dialog(model)).into());
    }
    main_area(model)
}

fn with_overlay<'a>(bottom: Element<'a, Message>, top: Element<'a, Message>) -> Element<'a, Message> {
    let overlay = container(top)
        .width(Fill).height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
            ..container::Style::default()
        });
    Stack::new().push(bottom).push(overlay).into()
}

// ── 主区域 ──────────────────────────────────────────────────────────────

fn main_area(model: &UnlockedModel) -> Element<'_, Message> {
    let sidebar: Element<'_, Message> = if model.menu_open {
        let bg = |_: &Theme| container::Style {
            background: Some(Color::from_rgb(0.18, 0.18, 0.18).into()),
            ..container::Style::default()
        };
        let mut items: Vec<Element<'_, Message>> = vec![
            container(button("  <-  菜单").on_press(Message::ToggleMenu).style(button::text).padding(10)).width(Fill).into(),
            container(button("= 密码本").on_press(Message::ToggleSettings).style(button::secondary).width(Fill)).padding(10).into(),
        ];
        if model.settings_visible {
            items.push(
                column![
                    text("  设置").size(14).color(Color::from_rgb(0.6, 0.6, 0.6)),
                    container(
                        row![text("历史备份数:").size(14), text_input("5", &model.settings_history_count).on_input(Message::SettingsHistoryCountChanged).padding([4.0, 6.0]).width(50)]
                    ).padding([4.0, 10.0]),
                    container(
                        toggler(model.settings_verify).label("启动 hash 校验").on_toggle(Message::SettingsVerifyToggled)
                    ).padding([4.0, 10.0]),
                    container(button("保存设置").on_press(Message::SettingsSave).style(button::primary).padding([4.0, 12.0])).padding(10),
                    container(button("导出恢复包").on_press(Message::ShowExportDialog).style(button::text).width(Fill)).padding(10),
                    container(button("导入恢复包").on_press(Message::ShowImportDialog).style(button::text).width(Fill)).padding(10),
                    container(button("历史版本").on_press(Message::ToggleHistory).style(button::text).width(Fill)).padding(10),
                ]
                .spacing(4)
                .width(Fill)
                .into(),
            );
        }
        if model.show_history {
            let mut hist_items: Vec<Element<'_, Message>> = model.history_files.iter().enumerate().map(|(i, name)| {
                container(
                    button(text(name).size(12)).on_press(Message::RestoreHistory(i)).style(button::text).width(Fill).padding([4.0, 8.0]),
                ).into()
            }).collect();
            if hist_items.is_empty() {
                hist_items.push(container(text("暂无历史版本").size(12).color(Color::from_rgb(0.5, 0.5, 0.5))).padding(10).into());
            }
            items.push(column(hist_items).spacing(2).width(Fill).into());
        }
        container(column(items).spacing(2).width(200)).style(bg).height(Fill).into()
    } else {
        container(text("")).width(0).height(Fill).into()
    };

    row![sidebar, right_panel(model)].into()
}

// ── 右侧面板 ────────────────────────────────────────────────────────────

fn right_panel(model: &UnlockedModel) -> Element<'_, Message> {
    let menu_btn: Element<'_, Message> = if model.menu_open {
        container(text("")).width(0).into()
    } else {
        button("=").on_press(Message::ToggleMenu).style(button::text).padding(8).into()
    };

    let toolbar = row![
        menu_btn,
        text("密码本").size(20),
        container(
            row![
                if model.edit_mode { text("  [编辑]").color(Color::from_rgb(0.9, 0.6, 0.2)) } else { text("") },
                if model.show_plaintext { text("  [明文]").color(Color::from_rgb(0.9, 0.3, 0.3)) } else { text("") },
                if model.show_copy { text("  [复制]").color(Color::from_rgb(0.3, 0.7, 0.9)) } else { text("") },
            ].spacing(4),
        ).width(Fill).align_x(alignment::Horizontal::Right),
    ].spacing(8).align_y(Center).padding(10.0);

    let search_bar: Element<'_, Message> = if model.show_search {
        container(text_input("搜索 / <cmd> 命令", &model.search_query)
            .id(text_input::Id::new("search"))
            .on_input(Message::SearchChanged)
            .on_submit(Message::SearchSubmitted)
            .padding(10).width(Fill))
        .padding(10.0).width(Fill).into()
    } else {
        container(text("")).height(0).into()
    };

    let col = column![toolbar, search_bar, password_list(model)].spacing(0);
    container(col).padding([0.0, 20.0]).width(Fill).height(Fill).into()
}

fn list_header(model: &UnlockedModel) -> Element<'_, Message> {
    let mut cols: Vec<Element<'_, Message>> = vec![container(text("").width(36)).into()];
    cols.push(container(text("密码名").size(14).color(Color::from_rgb(0.6, 0.6, 0.6))).width(Fill).padding([0.0, 10.0]).into());
    cols.push(container(text("密码").size(14).color(Color::from_rgb(0.6, 0.6, 0.6))).width(Fill).padding([0.0, 10.0]).into());
    if model.show_copy { cols.push(container(text("").width(50)).into()); }
    row(cols).into()
}

fn password_list(model: &UnlockedModel) -> Element<'_, Message> {
    let display_indices: Vec<usize> = if model.show_search && !model.search_query.is_empty() {
        model.search_results.iter().map(|&(i, _)| i).collect()
    } else {
        (0..model.entries.len()).collect()
    };
    let rows: Vec<Element<'_, Message>> = display_indices.iter().map(|&idx| {
        password_row(idx, &model.entries[idx], Some(idx) == model.best_match, model.edit_mode, model.show_copy, model.show_plaintext)
    }).collect();
    let list_content = column![list_header(model), column(rows).spacing(0)];
    container(scrollable(list_content).id(model.scroll_id.clone()).width(Fill).height(Fill)).width(Fill).height(Fill).into()
}

fn password_row(index: usize, entry: &PasswordEntry, highlighted: bool, edit_mode: bool, show_copy: bool, show_plaintext: bool) -> Element<'_, Message> {
    let display_text: String = if show_plaintext { entry.password.clone() } else { entry.password.chars().map(|_| MASK).collect() };
    let bg = move |_: &Theme| -> container::Style {
        if highlighted { container::Style { background: Some(Color::from_rgba(0.3, 0.5, 0.8, 0.3).into()), ..container::Style::default() } }
        else { container::Style::default() }
    };
    let delete_btn: Element<'_, Message> = if edit_mode {
        container(button(text("[X]").size(12)).on_press(Message::DeleteEntry(index)).style(button::danger).padding([2.0, 4.0])).width(36).align_x(Center).align_y(Center).into()
    } else { container(text("")).width(36).into() };
    let copy_btn: Element<'_, Message> = if show_copy {
        container(button("复制").on_press(Message::CopyPassword(index)).style(button::text).padding([4.0, 6.0])).width(50).align_x(Center).align_y(Center).into()
    } else { container(text("")).width(0).into() };
    let pass_col: Element<'_, Message> = if edit_mode && show_plaintext {
        text_input("", &entry.password).on_input(move |v| Message::EditEntryPassword(index, v)).padding([4.0, 8.0]).width(Fill).into()
    } else {
        container(text(display_text).size(16).color(if show_plaintext { Color::from_rgb(0.9, 0.3, 0.3) } else { Color::from_rgb(0.5, 0.8, 0.5) })).width(Fill).padding([8.0, 10.0]).align_y(Center).into()
    };
    container(row![delete_btn, container(text(&entry.name).size(16)).width(Fill).padding([8.0, 10.0]).align_y(Center), pass_col, copy_btn].align_y(Center)).width(Fill).style(bg).into()
}

// ── 添加密码弹窗 ───────────────────────────────────────────────────────

fn add_dialog(model: &UnlockedModel) -> Element<'_, Message> {
    let (match_text, match_color) = if model.confirm_password.is_empty() || model.new_password.is_empty() {
        ("", Color::from_rgb(0.6, 0.6, 0.6))
    } else if model.new_password == model.confirm_password {
        (" 一致", Color::from_rgb(0.3, 0.8, 0.3))
    } else {
        (" 不一致", Color::from_rgb(0.9, 0.3, 0.3))
    };
    let content = column![
        text("添加密码").size(22),
        text_input("密码名称", &model.new_name).on_input(Message::NewNameChanged).padding(10).width(Fill),
        if let Some(err) = &model.new_name_error {
            text(err).size(13).color(Color::from_rgb(0.9, 0.3, 0.3))
        } else { text("").height(0).into() },
        text_input("密码内容 (仅 ASCII)", &model.new_password).on_input(Message::NewPasswordChanged).on_submit(Message::SubmitNewPassword).padding(10).width(Fill).secure(true),
        row![
            text_input("再次输入密码", &model.confirm_password).on_input(Message::ConfirmPasswordChanged).padding(10).width(Fill).secure(true),
            text(match_text).size(14).color(match_color),
        ].align_y(Center).spacing(4),
        container(row![
            text("位数:").size(14),
            text_input("16", &model.random_length).on_input(Message::RandomLengthChanged).padding([6.0, 8.0]).width(60),
            button("生成随机密码").on_press(Message::GeneratePassword).style(button::primary).padding([6.0, 12.0]),
        ].spacing(6).align_y(Center)).padding(0),
        row![
            button("取消").on_press(Message::HideAddDialog).style(button::secondary).width(Fill),
            button("保存").on_press(Message::SubmitNewPassword).style(button::primary).width(Fill),
        ].spacing(10),
    ].spacing(10).padding(24).max_width(400).align_x(Center);
    container(content).style(container::rounded_box).max_width(420).into()
}

// ── 导出弹窗 ─────────────────────────────────────────────────────────────

fn export_dialog(model: &UnlockedModel) -> Element<'_, Message> {
    let status = model.export_status.as_deref().unwrap_or("");
    let content = column![
        text("导出恢复包").size(22),
        text("将在密码本同级目录下创建 export_recovery/ 文件夹，\n包含 passwords.vault 和 passwords.recovery 文件。\n请妥善保管这两个文件！").size(14).color(Color::from_rgb(0.6, 0.6, 0.6)),
        if !status.is_empty() {
            let color = if status.contains("失败") { Color::from_rgb(0.9, 0.3, 0.3) } else { Color::from_rgb(0.3, 0.8, 0.3) };
            let msg: Element<'_, Message> = container(text(status).size(14).color(color)).into();
            msg
        } else { container(text("")).height(0).into() },
        row![
            button("取消").on_press(Message::HideExportDialog).style(button::secondary).width(Fill),
            button("开始导出").on_press(Message::ExportSubmit).style(button::primary).width(Fill),
        ].spacing(10),
    ].spacing(12).padding(24).max_width(400).align_x(Center);
    container(content).style(container::rounded_box).max_width(450).into()
}

// ── 导入弹窗 ─────────────────────────────────────────────────────────────

fn import_dialog(model: &UnlockedModel) -> Element<'_, Message> {
    let status = model.import_status.as_deref().unwrap_or("");
    let content = column![
        text("导入恢复包").size(22),
        text("导入将替换当前所有密码数据！").size(14).color(Color::from_rgb(0.9, 0.3, 0.3)),
        text_input("vault 文件路径", &model.import_vault_path).on_input(Message::ImportVaultPathChanged).padding(10).width(Fill),
        text_input("recovery 文件路径", &model.import_recovery_path).on_input(Message::ImportRecoveryPathChanged).padding(10).width(Fill),
        if !status.is_empty() {
            let color = if status.contains("失败") { Color::from_rgb(0.9, 0.3, 0.3) } else { Color::from_rgb(0.3, 0.8, 0.3) };
            let msg: Element<'_, Message> = container(text(status).size(14).color(color)).into();
            msg
        } else { container(text("")).height(0).into() },
        row![
            button("取消").on_press(Message::HideImportDialog).style(button::secondary).width(Fill),
            button("导入").on_press(Message::ImportSubmit).style(button::primary).width(Fill),
        ].spacing(10),
    ].spacing(12).padding(24).max_width(400).align_x(Center);
    container(content).style(container::rounded_box).max_width(450).into()
}

// ── <cmd> 命令结果弹窗 ────────────────────────────────────────────────

fn cmd_result_dialog(model: &UnlockedModel) -> Element<'_, Message> {
    let lines: Vec<Element<'_, Message>> = model.cmd_result_text
        .lines()
        .map(|line| text(line).size(14).into())
        .collect();

    let content = column![
        text("命令结果").size(20),
        scrollable(column(lines).spacing(2).padding(10))
            .height(300)
            .width(Fill),
        button("关闭 (Esc)")
            .on_press(Message::HideCmdResult)
            .style(button::secondary)
            .padding([8.0, 30.0]),
    ]
    .spacing(12)
    .padding(20)
    .max_width(500)
    .align_x(Center);

    container(content)
        .style(container::rounded_box)
        .max_width(550)
        .into()
}
