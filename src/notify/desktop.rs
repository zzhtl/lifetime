// 跨平台桌面通知
// 对 notify-rust 做最薄封装，失败时降级到日志

use anyhow::Result;

pub fn send_notification(title: &str, body: &str) -> Result<()> {
    // notify-rust 在不同平台行为略有差异；这里统一捕获错误
    let res = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .appname("Lifetime")
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
