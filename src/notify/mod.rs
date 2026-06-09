// 通知与音效封装
// 桌面通知用 notify-rust（跨平台），音效用 rodio 合成正弦波，无需打包音频文件

mod desktop;
mod sound;

pub use sound::SoundPlayer;

use crate::reminders::{Intensity, ReminderKind};
use std::cell::RefCell;
use std::time::{Duration, Instant};

const DESKTOP_NOTIFICATION_HANDLE_TTL: Duration = Duration::from_secs(6);

pub struct Notifier {
    pub sound: SoundPlayer,
    desktop_notifications: RefCell<Vec<ActiveDesktopNotification>>,
}

struct ActiveDesktopNotification {
    _handle: desktop::DesktopNotification,
    expires_at: Instant,
}

impl Notifier {
    pub fn new() -> Self {
        Self {
            sound: SoundPlayer::new(),
            desktop_notifications: RefCell::new(Vec::new()),
        }
    }

    /// 综合处理一次提醒：桌面通知 + 音效（依据强度）
    /// body 为通知正文（由调用方按需轮换具体动作/小贴士）
    pub fn dispatch(&self, kind: ReminderKind, body: &str, desktop_on: bool, sound_on: bool, volume: f32) {
        let intensity = kind.intensity();
        self.prune_desktop_notifications();
        if desktop_on {
            let title = format!("Lifetime · {}", kind.label());
            if let Ok(Some(handle)) = desktop::send_notification(&title, body) {
                self.keep_desktop_notification(handle);
            }
        }
        if sound_on {
            self.sound.play(intensity, volume);
        }
    }

    #[allow(dead_code)]
    pub fn beep(&self, intensity: Intensity, volume: f32) {
        self.sound.play(intensity, volume);
    }

    fn keep_desktop_notification(&self, handle: desktop::DesktopNotification) {
        self.desktop_notifications
            .borrow_mut()
            .push(ActiveDesktopNotification {
                _handle: handle,
                expires_at: Instant::now() + DESKTOP_NOTIFICATION_HANDLE_TTL,
            });
    }

    fn prune_desktop_notifications(&self) {
        let now = Instant::now();
        self.desktop_notifications
            .borrow_mut()
            .retain(|notification| notification.expires_at > now);
    }
}

impl Default for Notifier {
    fn default() -> Self {
        Self::new()
    }
}
