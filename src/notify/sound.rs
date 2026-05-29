// 程序化合成提示音
// 三档强度对应三种 beep 模式，避免外部音频文件依赖
//
// 实现细节：
//   - rodio::OutputStream 在创建时打开默认音频设备；
//   - SineWave 生成纯正弦波，TakeDuration 限制时长；
//   - 不同强度通过频率 + 节拍数差异区分；
//   - 播放放到独立线程，避免阻塞 UI；
//   - 若设备初始化失败（headless / 无声卡），降级为静默。

use rodio::source::{SineWave, Source};
use rodio::OutputStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::reminders::Intensity;

pub struct SoundPlayer {
    /// 暂存 stream，确保设备保活；None 表示初始化失败
    _stream: Arc<Mutex<Option<OutputStream>>>,
    stream_handle: Option<rodio::OutputStreamHandle>,
}

impl SoundPlayer {
    pub fn new() -> Self {
        match OutputStream::try_default() {
            Ok((stream, handle)) => Self {
                _stream: Arc::new(Mutex::new(Some(stream))),
                stream_handle: Some(handle),
            },
            Err(e) => {
                log::warn!("音频设备初始化失败 ({e})，将静默播放");
                Self {
                    _stream: Arc::new(Mutex::new(None)),
                    stream_handle: None,
                }
            }
        }
    }

    pub fn play(&self, intensity: Intensity, volume: f32) {
        let Some(handle) = self.stream_handle.clone() else {
            return;
        };
        let vol = volume.clamp(0.0, 1.0);
        thread::spawn(move || {
            let _ = play_pattern(&handle, intensity, vol);
        });
    }
}

fn play_pattern(
    handle: &rodio::OutputStreamHandle,
    intensity: Intensity,
    volume: f32,
) -> Result<(), rodio::PlayError> {
    let sink = rodio::Sink::try_new(handle)?;
    sink.set_volume(volume);
    let beats: &[(u32, u64)] = match intensity {
        // (frequency Hz, duration ms)
        Intensity::Soft => &[(660, 140)],
        Intensity::Medium => &[(660, 140), (0, 80), (880, 160)],
        Intensity::Strong => &[(660, 160), (0, 100), (880, 160), (0, 100), (1100, 220)],
    };
    for (freq, ms) in beats {
        if *freq == 0 {
            sink.append(
                SineWave::new(0.0)
                    .take_duration(Duration::from_millis(*ms))
                    .amplify(0.0),
            );
        } else {
            sink.append(
                SineWave::new(*freq as f32)
                    .take_duration(Duration::from_millis(*ms))
                    .fade_in(Duration::from_millis(20)),
            );
        }
    }
    sink.sleep_until_end();
    Ok(())
}
