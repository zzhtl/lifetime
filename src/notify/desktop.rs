// 跨平台桌面通知
// 对 notify-rust 做最薄封装，失败时降级到日志
//
// 平台说明：
//   - Linux/BSD：走 D-Bus（org.freedesktop.Notifications），appname/icon/hint 均生效。
//   - Windows：走 WinRT toast，使用 appname 归类；从源码运行通常即可弹出。
//   - macOS：走 mac-notification-sys，功能受限；裸二进制运行会以通用 bundle 名义弹出，
//     要显示自有应用名/图标需打包成 .app（带 Info.plist 的 CFBundleIdentifier）。

use anyhow::Result;

pub fn send_notification(title: &str, body: &str) -> Result<()> {
    // notify-rust 在不同平台行为略有差异；这里统一捕获错误
    let res = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .appname("Lifetime")
        // freedesktop 标准图标名：Linux 下显示信息图标，其它平台安全忽略
        .icon("dialog-information")
        .timeout(notify_rust::Timeout::Milliseconds(5_000))
        .show();
    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!("桌面通知失败: {e}");
            Ok(())
        }
    }
}
