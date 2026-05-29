// 通知与音效封装
// 桌面通知用 notify-rust（跨平台），音效用 rodio 合成正弦波，无需打包音频文件

mod desktop;
mod sound;

pub use sound::SoundPlayer;

use crate::reminders::{Intensity, ReminderKind};

pub struct Notifier {
    pub sound: SoundPlayer,
}

impl Notifier {
    pub fn new() -> Self {
        Self {
            sound: SoundPlayer::new(),
        }
    }

    /// 综合处理一次提醒：桌面通知 + 音效（依据强度）
    pub fn dispatch(&self, kind: ReminderKind, desktop_on: bool, sound_on: bool, volume: f32) {
        let intensity = kind.intensity();
        if desktop_on {
            let title = format!("Lifetime · {}", kind.label());
            let _ = desktop::send_notification(&title, kind.brief());
        }
        if sound_on {
            self.sound.play(intensity, volume);
        }
    }

    #[allow(dead_code)]
    pub fn beep(&self, intensity: Intensity, volume: f32) {
        self.sound.play(intensity, volume);
    }
}

impl Default for Notifier {
    fn default() -> Self {
        Self::new()
    }
}
