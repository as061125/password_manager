# 密码本 — Password Manager

> ⚠️ **声明**：本项目由 **Reasonix Code**（AI 编程助手）主力编写，**as061125** 提供需求与反馈。

跨平台加密密码管理器，支持 **GUI**（基于 Iced）和 **CLI** 双模式。
登录一次密码后在同一 terminal 中无需重复输入。

---

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

---

## 安装

### 编译

```bash
git clone https://github.com/as061125/password_manager.git
cd password_manager
cargo build --release
```

编译产物在 `target/release/iced_demo.exe`（Windows）或 `target/release/iced_demo`（macOS/Linux）。

### 添加环境变量（任何目录下直接运行 `pwm`）

#### Windows

**方法一：命令（管理员 PowerShell）**
```powershell
[Environment]::SetEnvironmentVariable(
    "Path",
    [Environment]::GetEnvironmentVariable("Path", "User") + ";$pwd\target\release",
    "User"
)
```
> 在 `password_manager` 目录下执行。路径中的 `$pwd` 会自动替换为当前目录。

**方法二：手动（推荐）**
1. 复制 `target/release/iced_demo.exe` 到 `C:\Users\<你的用户名>\AppData\Local\pwm\`
2. 或者记下完整路径，按 <kbd>Win</kbd>+<kbd>R</kbd> → `sysdm.cpl` → 高级 → 环境变量
3. 在「用户变量」中找到 `Path`，编辑，添加你的 `target\release` 完整路径

**验证：**
```cmd
pwm help
```

#### macOS / Linux

**临时生效（当前 terminal）**
```bash
export PATH="$PATH:$(pwd)/target/release"
```

**永久生效**
```bash
# 将路径追加到 shell 配置
echo 'export PATH="$PATH:'$(pwd)'/target/release"' >> ~/.bashrc
# 或如果使用 zsh（macOS 默认）：
echo 'export PATH="$PATH:'$(pwd)'/target/release"' >> ~/.zshrc
# 重新加载
source ~/.bashrc   # 或 source ~/.zshrc
```

**系统级安装（推荐）**
```bash
sudo cp target/release/iced_demo /usr/local/bin/pwm
```
之后直接运行 `pwm help`。

---

## 快速开始

```bash
# GUI 模式（无参数直接运行）
pwm

# CLI 模式
pwm login                           # 首次登录，输入主密码
pwm add github                      # 添加条目
pwm add gmail mypass123             # 添加条目（直接传密码）
pwm list                            # 列出所有
pwm get github                      # 查看（遮罩）
pwm get github -p                   # 查看（明文）
pwm get git -f -c                   # 模糊搜索 + 复制
pwm rm github                       # 删除
pwm history                         # 历史版本
pwm help                            # 帮助
```

---

## 加密方案

| 环节 | 算法 | 说明 |
|------|------|------|
| 密钥派生 | PBKDF2-HMAC-SHA256 | 100,000 次迭代，随机 16 字节盐值 |
| 加密 | AES-256-GCM | 认证加密，防篡改 |
| 文件格式 | salt(16) \|\| nonce(12) \|\| ciphertext | 单文件，二进制 |

---

## 功能特性

### GUI 模式快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+/` | 打开搜索栏（支持 `<cmd>` 命令） |
| `Ctrl+N` | 添加新密码 |
| `Ctrl+I` | 编辑模式（显示删除按钮） |
| `Ctrl+C` | 显示/隐藏复制按钮 |
| `Ctrl+D` | 明/暗切换 |
| `Esc` | 关闭当前弹窗/面板 |

### CLI 命令

```
用法:  pwm [全局选项] <子命令> [参数]

全局选项:
  --password <密码>   直接传入主密码

子命令:
  login               登录并缓存主密码（本次 terminal 有效）
  logout              登出清除缓存
  list              列出所有条目 [-p 明文]
  get <name>        查看条目 [-f 模糊] [-p 明文] [-c 复制]
  add <name> [密码] 添加条目
  rm <name>         删除条目
  mv <旧> <新>       重命名
  export [目录]      导出恢复包
  import <v> <r>    导入恢复包
  history           历史版本列表
  restore <序号>     回滚
  help              帮助
  version           版本
```

### 搜索栏命令

GUI 搜索栏中以 `<cmd>` 开头，按回车执行：

```
<cmd>list             列出
<cmd>get xxx          查看
<cmd>add name pwd     添加
<cmd>rm xxx           删除
<cmd>mv old new       重命名
<cmd>display          明文
<cmd>hide             遮罩
<cmd>help             帮助
```

---

## 安全特性

- **AES-256-GCM 认证加密**：密文被篡改后解密失败并报错
- **PBKDF2 密钥派生**：主密码不直接用作加密密钥
- **随机盐值 + 随机 nonce**：每次保存密文不同
- **历史备份**：保存最近 N 个版本（可在设置中配置 0~10）
- **SHA-256 校验**：启动时可验证 vault 文件完整性
- **原子写入**：写临时文件 → fsync → rename，避免崩溃损坏
- **双文件恢复包**：vault + recovery key，不依赖主密码恢复
- **剪贴板自动清除**：复制密码后 spawn 独立子进程，60 秒后自动清空剪贴板（主进程退出不影响）

---

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

---

## License

MIT
