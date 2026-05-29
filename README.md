# 密码本 — Password Manager

> ⚠️ **声明**：本项目由 **Reasonix Code**（AI 编程助手）主力编写，**as061125** 提供需求与反馈。

跨平台加密密码管理器，支持 **GUI**（基于 Iced）和 **CLI** 双模式。  
AES-256-GCM 认证加密 + PBKDF2 密钥派生，登录一次密码后同 terminal 无需重复输入。

---

## 安装

```bash
git clone https://github.com/as061125/password_manager.git
cd password_manager
cargo build --release
```

编译产物在 `target/release/iced_demo.exe`（Windows）或 `target/release/iced_demo`（macOS/Linux）。

### 添加环境变量

**Windows** — 将 `target\release` 路径加入 `Path` 系统变量，或复制 `iced_demo.exe` 到 `C:\Users\<你>\AppData\Local\pwm\`。

**macOS / Linux**

```bash
# 系统级（推荐）
sudo cp target/release/iced_demo /usr/local/bin/pwm

# 或添加到 shell 配置
echo 'export PATH="$PATH:'$(pwd)'/target/release"' >> ~/.zshrc
source ~/.zshrc
```

---

## 快速上手

```bash
# GUI 模式
pwm

# CLI 模式
pwm login                           # 首次登录，后续免密
pwm add github                      # 添加条目（交互式）
pwm add gmail mypass123             # 添加条目（直接传密码）
pwm list                            # 列出所有（遮罩）
pwm list -p                         # 列出所有（明文）
pwm get github                      # 查看（严格匹配 + 遮罩）
pwm get git -f                      # 模糊搜索
pwm get git -f -c                   # 模糊搜索 + 复制
pwm rm github                       # 删除
pwm mv github GitLab                # 重命名
pwm clear                           # 清空所有（需确认）
pwm history                         # 历史版本列表
pwm restore 1                       # 回滚到第 2 个版本
pwm logout                          # 登出清除缓存
```

---

## 加密方案

| 环节 | 算法 | 参数 |
|------|------|------|
| 密钥派生 | PBKDF2-HMAC-SHA256 | 100,000 次迭代（可配置），随机 16 字节盐值 |
| 加密 | AES-256-GCM | 认证加密，防篡改 |
| 文件格式 | salt(16) \|\| nonce(12) \|\| ciphertext | 二进制单文件 |
| 路径 | Windows: `%APPDATA%/pwm/passwords.vault` | macOS/Linux: `~/.local/share/pwm/` |

---

## GUI 模式

### 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+/` | 打开搜索栏（支持 `<cmd>` 命令） |
| `Ctrl+N` | 添加新密码 |
| `Ctrl+I` + `<cmd>display` | 编辑模式 + 明文 → 密码列变为可编辑输入框 |
| `Ctrl+C` | 显示/隐藏每行的复制按钮 |
| `Ctrl+D` | 切换明文/遮罩显示 |
| `Esc` | 关闭当前弹窗/面板 |

### 添加密码弹窗

```
┌──────────────────────────────┐
│ 添加密码                      │
│ [密码名称___________]        │
│ [密码内容 (仅 ASCII)__] [一致]│
│ [再次输入密码________]       │
│ 位数: [16] [生成随机密码]    │
│      [取消] [保存]           │
└──────────────────────────────┘
```

- 手动键入密码需**二次确认**
- 点击「生成随机密码」→ 可选 4~64 位强密码（大小写+数字+特殊符号）
- **重名检测**：同名时不能保存，显示红色提示

### 设置面板

侧边栏点击「=」打开设置：

```
设置
├ 历史备份数: [5]          (0~10)
├ [x] 启动 hash 校验
├ PBKDF2 迭代: [100000]    (10000~10000000, 改后自动重加密)
├ 自动锁定(分): [0]        (0=不锁定, >0 分钟后自动锁屏)
├ [保存设置]
├ 导出恢复包
├ 导入恢复包
└ 历史版本
```

### 搜索栏命令

`Ctrl+/` 打开搜索栏，输入 `<cmd>` 开头命令按回车执行：

| 命令 | 效果 |
|------|------|
| `<cmd>list` | 弹窗列出所有条目 |
| `<cmd>list -p` | 弹窗明文列出 |
| `<cmd>get xxx` | 弹窗显示匹配结果 |
| `<cmd>add name pwd` | 打开添加弹窗并预填 |
| `<cmd>rm xxx` | 删除匹配条目 |
| `<cmd>mv old new` | 重命名 |
| `<cmd>display` | 切换到明文显示 |
| `<cmd>hide` | 恢复遮罩 |
| `<cmd>help` | 弹窗显示帮助 |

---

## CLI 模式

### 全局选项

| 选项 | 说明 |
|------|------|
| `--password <pwd>` | 直接传入主密码（优先级最高） |
| `--password=<pwd>` | 等号语法 |

### 命令

| 命令 | 说明 |
|------|------|
| `login` | 登录并缓存主密码（同时 spawn 守护进程监控 terminal） |
| `logout` | 登出清除缓存 |
| `list` | 列出所有 |
| `list -p` | 明文列出 |
| `get <name>` | 严格匹配 + 遮罩 |
| `get <name> -f` | 模糊匹配 |
| `get <name> -p` | 明文输出 |
| `get <name> -f -c` | 模糊匹配 + 复制（静默） |
| `add <name>` | 交互式：选择 设置密码(s) / 自动生成(a) |
| `add <name> <pwd>` | 直接传入密码 |
| `rm <name>` | 删除 |
| `mv <旧> <新>` | 重命名 |
| `export [目录]` | 导出恢复包 |
| `import <vault> <rec>` | 从恢复包导入 |
| `history` | 历史版本列表 |
| `restore <n>` | 回滚到指定版本 |
| `clear` | 清空所有密码（需确认 y/N） |
| `set-path <路径>` | 更改 vault 路径 |
| `help` | 显示帮助 |
| `version` | 显示版本号 |

### 密码来源优先级

```
1. --password 参数
2. session 缓存文件（pwm login 后自动创建）
3. PWM_PASSWORD 环境变量
4. 交互式输入（自动缓存 + spawn 守护进程）
```

---

## 安全特性

| 特性 | 说明 |
|------|------|
| **AES-256-GCM 认证加密** | 密文被篡改后解密失败并报错 |
| **PBKDF2 密钥派生** | 主密码不直接用作加密密钥，迭代次数可配置 |
| **随机盐值 + 随机 nonce** | 每次保存密文不同，防重放 |
| **Zeroize 内存覆写** | 删除条目 / 锁定 / 释放时覆写密码和密钥，不留痕迹 |
| **原子写入** | 写 tmp → fsync → rename，崩溃不损坏 |
| **SHA-256 校验** | 启动时验证 vault 文件完整性（可配置） |
| **历史备份** | 保存最近 N 个版本（0~10，可配置） |
| **双文件恢复包** | vault + recovery key，可在不依赖主密码的情况下恢复 |
| **剪贴板自动清除** | 复制密码后 spawn 独立进程，60 秒后自动清空 |
| **GUI 自动锁屏** | 无操作超时自动切回锁屏（可配置分钟数） |
| **CLI session 守护** | 监听 shell 进程，终端关闭自动登出 |
| **文件防篡改** | hash 校验 + 认证加密双重检测 |

---

## 项目结构

```
src/
├── main.rs      — 入口（CLI/GUI 自动判断 + 守护/剪贴板后台模式）
├── cli.rs       — 命令行模式
├── vault.rs     — 加密存储（AES-256-GCM + PBKDF2，零 Iced 依赖）
├── model.rs     — 数据结构（含 Zeroize Drop 实现）
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
| `rpassword` | CLI 密码不回显输入 |
| `arboard` | 剪贴板操作 |
| `dirs` | 跨平台数据目录 |
| `zeroize` | 内存敏感数据覆写 |
| `winres` | Windows .exe 图标嵌入 |

---

## License

MIT
