# 密码本 — Password Manager

跨平台加密密码管理器，支持 **GUI**（基于 Iced）和 **CLI** 双模式。

## 截图

```
┌────────────────────────────────────────────┐
│ 创建主密码                                  │
│ 设置一个主密码用于加密保险库                 │
│ ┌──────────────────────────────┐           │
│ │ 主密码                       │           │
│ └──────────────────────────────┘           │
│            [创建]                           │
└────────────────────────────────────────────┘
```

## 快速开始

```bash
# 安装
git clone https://github.com/as061125/password_manager.git
cd password_manager/iced_demo
cargo build --release

# GUI 模式（无参数）
./target/release/iced_demo

# CLI 模式（带参数）
./target/release/iced_demo help
```

## 加密方案

| 环节 | 算法 | 说明 |
|------|------|------|
| 密钥派生 | PBKDF2-HMAC-SHA256 | 100,000 次迭代，随机 16 字节盐值 |
| 加密 | AES-256-GCM | 认证加密，防篡改 |
| 文件格式 | salt(16) \|\| nonce(12) \|\| ciphertext | 单文件，二进 |

## 功能特性

### GUI 模式

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+/` | 打开搜索栏（支持 `<cmd>` 命令） |
| `Ctrl+N` | 添加新密码 |
| `Ctrl+I` | 编辑模式（显示删除按钮） |
| `Ctrl+C` | 显示/隐藏复制按钮 |
| `Ctrl+D` | 明/暗切换 |
| `Esc` | 关闭当前弹窗/面板 |

### CLI 模式

```bash
# 全局选项
--password <密码>     直接传入主密码

# 命令
pwm login              登录并缓存主密码
pwm logout             登出清除缓存
pwm list              列出所有条目
  -p, --plaintext      明文显示
pwm get <name>        查看条目（默认严格+遮罩）
  -f, --fuzzy          模糊匹配
  -p, --plaintext      明文显示
  -c, --copy           复制到剪贴板
pwm add <name> [密码]  添加条目
pwm rm <name>         删除条目
pwm mv <旧> <新>       重命名
pwm export [目录]      导出恢复包
pwm import <v> <r>    导入恢复包
pwm history           历史版本列表
pwm restore <序号>     回滚
pwm help              帮助
pwm version           版本
```

### 搜索栏命令

GUI 搜索栏中以 `<cmd>` 开头输入：

```
<cmd>list             列出所有条目
<cmd>get xxx          查看条目
<cmd>add name pwd     添加条目
<cmd>rm xxx           删除条目
<cmd>mv old new       重命名
<cmd>display          明文显示
<cmd>hide             恢复遮罩
<cmd>help             显示帮助
<cmd>version          版本
```

## 安全特性

- **AES-256-GCM 认证加密**：密文被篡改后解密失败并报错
- **PBKDF2 密钥派生**：主密码不直接用作加密密钥
- **随机盐值 + 随机 nonce**：每次保存密文不同
- **历史备份**：保存最近 N 个版本（可在设置中配置 0~10）
- **SHA-256 校验**：启动时可验证 vault 文件完整性
- **原子写入**：写临时文件 → fsync → rename，避免崩溃损坏
- **双文件恢复包**：vault + recovery key，不依赖主密码恢复

## 项目结构

```
src/
├── main.rs      — 入口（自动判断 CLI / GUI 模式）
├── cli.rs       — 命令行模式
├── vault.rs     — 加密存储（零 Iced 依赖）
├── model.rs     — 数据结构
├── message.rs   — 消息枚举 + 命令解析（CLI 和搜索栏共享）
├── search.rs    — FZF 模糊搜索
├── update.rs    — 业务逻辑 + 状态转换
└── view.rs      — GUI 界面渲染
```

## 依赖

| Crate | 用途 |
|-------|------|
| `iced` | GUI 框架 |
| `aes-gcm` | AES-256-GCM 加密 |
| `pbkdf2` + `sha2` | PBKDF2 密钥派生 |
| `rand` | 随机盐值/nonce/密码 |
| `serde` + `serde_json` | 序列化 |
| `hex` | SHA-256 格式化 |
| `rpassword` | CLI 密码不回显 |
| `arboard` | CLI 剪贴板 |

## License

MIT
