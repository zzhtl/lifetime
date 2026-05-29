# Lifetime · 健康助手

一款面向程序员 / 长期久坐人群的科学健康提醒工具。  
按时间节律自动提醒护眼、起身、喝水、颈椎活动、番茄钟与强制大休息，并内置可检索的健康知识库。

> 用 Rust + eframe (egui) 写成，单二进制，跨平台（Linux / macOS / Windows），数据本地保存（SQLite），不联网。

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

### 健康知识库

9 大类共 40+ 条带步骤的健康技巧：护眼、颈椎与肩、腰背、手腕（防 RSI）、腿部循环、呼吸与心理、饮食与水分、姿势与工位、睡眠。

### 长期统计

每次工作会话、提醒事件都进 SQLite，统计面板内置 30 天趋势折线 + 提醒类型分布柱状图。

## 运行 / 构建

```bash
cargo run --release
```

数据/配置自动落在以下位置（首次启动自动生成）：

- Linux: `~/.config/lifetime/`
- macOS: `~/Library/Application Support/lifetime/`
- Windows: `%APPDATA%\lifetime\`

里面有 `config.toml`（可手动编辑）和 `lifetime.db`。

### 系统依赖

- **Linux**：ALSA / PulseAudio（音效）和 D-Bus / libnotify（通知）。Debian 系：
  ```bash
  sudo apt install libasound2-dev libdbus-1-dev
  ```
- **macOS**：使用系统 PingFang 字体显示中文；通知首次会要求授权。
- **Windows**：内置 WinRT 后端，开箱即用。

## 测试

```bash
cargo test
```

覆盖 SQLite 增删查改、调度器周期匹配、知识库加载等关键逻辑。

## 设计文档

完整设计与实施步骤见 `/home/qingteng/.claude/plans/jolly-riding-graham.md`。

## 路线图（v2+）

- 系统托盘 + 关闭最小化
- 屏幕空闲自动暂停
- 数据导出 CSV / 周报
- 联网更新健康知识库
- 多人协作模式

## License

MIT
