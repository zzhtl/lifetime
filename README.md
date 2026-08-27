# Lifetime · 健康助手

[![Release](https://img.shields.io/github/v/release/zzhtl/lifetime)](https://github.com/zzhtl/lifetime/releases/latest)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

一款面向程序员 / 长期久坐人群的科学健康提醒工具。
按时间节律自动提醒护眼、起身、喝水、颈椎活动、番茄钟与强制大休息，并内置可检索的健康知识库。

> 用 Rust + eframe (egui) 写成，单二进制，跨平台（Linux / macOS / Windows），数据本地保存（SQLite），不联网。

## 安装

### Linux（一键安装）

```bash
curl -fsSL https://raw.githubusercontent.com/zzhtl/lifetime/main/install.sh | sh
```

自动下载最新 release、校验 sha256、安装到 `~/.local/bin`。可用环境变量定制：

```bash
# 指定版本 / 安装目录
LIFETIME_VERSION=v0.1.0 LIFETIME_INSTALL_DIR=/usr/local/bin \
  curl -fsSL https://raw.githubusercontent.com/zzhtl/lifetime/main/install.sh | sh
```

### Ubuntu / Debian（deb 包）

从 [Releases](https://github.com/zzhtl/lifetime/releases/latest) 下载 `.deb` 后：

```bash
sudo apt install ./lifetime_*_amd64.deb
```

会自动带上运行时依赖（ALSA / OpenGL）。

### Windows（一键安装）

PowerShell 中执行：

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://raw.githubusercontent.com/zzhtl/lifetime/main/install.ps1 | iex"
```

安装到 `%LOCALAPPDATA%\Programs\Lifetime` 并加入用户 PATH。也可直接从 [Releases](https://github.com/zzhtl/lifetime/releases/latest) 下载 zip 解压使用。

### 源码构建（含 macOS）

```bash
git clone https://github.com/zzhtl/lifetime && cd lifetime
cargo build --release   # 产物在 target/release/lifetime
```

安装后首次运行会自动在应用菜单 / 开始菜单创建快捷方式，之后可从菜单启动。

### 运行环境要求

- **Linux**：x86_64，glibc ≥ 2.35（Ubuntu 22.04+ / Debian 12+）。运行时需要 ALSA（音效）与桌面通知服务：
  ```bash
  sudo apt install libasound2t64   # Ubuntu 24.04+；22.04 用 libasound2
  ```
  从源码构建还需要 `pkg-config` 与 `libasound2-dev`。
- **Windows**：x86_64，Windows 10+，开箱即用。
- **macOS**：暂无预编译包，从源码构建；使用系统 PingFang 字体显示中文，通知首次会要求授权。

## 功能一览

| 类型 | 周期 | 强度 |
|------|------|------|
| 20-20-20 护眼 | 每 20 min | 桌面通知 |
| 起身舒展 | 每 30 min | 桌面通知 |
| 喝水 | 每 45 min | 桌面通知 |
| 颈椎活动 | 每 60 min | 通知 + 声音 |
| 番茄钟 50/10 | 循环 | 通知 + 声音 |
| 大休息（强制） | 每 90 min | 全屏模态遮罩 + 声音 |
| 午餐 | 12:00 | 通知 + 声音 |
| 下班 | 累计 8 h | 通知 + 声音 |
| 睡眠 | 22:30 | 通知 + 声音 |

所有周期、强度、勿扰时段都在「设置」面板里可调。

默认开启每日自动工作时段：应用保持运行时，09:00 自动开始会话，19:00 自动停止；时间与开关可在「设置 → 日程提醒」调整。当天手动结束后不会被立即重新启动。

### 健康知识库

9 大类共 40+ 条带步骤的健康技巧：护眼、颈椎与肩、腰背、手腕（防 RSI）、腿部循环、呼吸与心理、饮食与水分、姿势与工位、睡眠。

### 长期统计

每次工作会话、提醒事件都进 SQLite，统计面板内置 30 天趋势折线 + 提醒类型分布柱状图。

## 数据与配置

数据/配置自动落在以下位置（首次启动自动生成）：

- Linux: `~/.config/lifetime/`
- macOS: `~/Library/Application Support/lifetime/`
- Windows: `%APPDATA%\lifetime\`

里面有 `config.toml`（可手动编辑）和 `lifetime.db`。

## 测试

```bash
cargo test
```

覆盖 SQLite 增删查改、调度器周期匹配、知识库加载等关键逻辑。

## 发布流程（维护者）

版本号改在 `Cargo.toml`，然后打 tag 推送即可，CI（`.github/workflows/release.yml`）自动完成构建与发布：

```bash
git tag v0.1.0 && git push origin v0.1.0
```

自动产出并上传到 GitHub Release：

- `lifetime-v<版本>-x86_64-unknown-linux-gnu.tar.gz` — Linux 通用二进制（ubuntu:22.04 容器内构建）
- `lifetime_<版本>-1_amd64.deb` — Ubuntu / Debian 安装包
- `lifetime-v<版本>-x86_64-pc-windows-msvc.zip` — Windows
- `sha256sums.txt` — 校验和（`install.sh` 自动校验）

tag 版本与 `Cargo.toml` 的 `version` 不一致时 CI 会直接失败。

## 路线图（v2+）

- 系统托盘 + 关闭最小化
- 屏幕空闲自动暂停
- 数据导出 CSV / 周报
- 联网更新健康知识库
- 多人协作模式

## License

[Apache-2.0](LICENSE)
